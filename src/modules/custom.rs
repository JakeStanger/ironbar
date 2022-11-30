use crate::config::CommonConfig;
use crate::dynamic_string::DynamicString;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::popup::{ButtonGeometry, Popup};
use crate::script::Script;
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{Button, Label, Orientation};
use serde::Deserialize;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug, error};

#[derive(Debug, Deserialize, Clone)]
pub struct CustomModule {
    /// Container class name
    class: Option<String>,
    /// Widgets to add to the bar container
    bar: Vec<Widget>,
    /// Widgets to add to the popup container
    popup: Option<Vec<Widget>>,

    #[serde(flatten)]
    pub common: CommonConfig,
}

/// Attempts to parse an `Orientation` from `String`
fn try_get_orientation(orientation: &str) -> Result<Orientation> {
    match orientation.to_lowercase().as_str() {
        "horizontal" | "h" => Ok(Orientation::Horizontal),
        "vertical" | "v" => Ok(Orientation::Vertical),
        _ => Err(Report::msg("Invalid orientation string in config")),
    }
}

/// Widget attributes
#[derive(Debug, Deserialize, Clone)]
pub struct Widget {
    /// Type of GTK widget to add
    #[serde(rename = "type")]
    widget_type: WidgetType,
    widgets: Option<Vec<Widget>>,
    label: Option<String>,
    name: Option<String>,
    class: Option<String>,
    on_click: Option<String>,
    orientation: Option<String>,
}

/// Supported GTK widget types
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    Box,
    Label,
    Button,
}

impl Widget {
    /// Creates this widget and adds it to the parent container
    fn add_to(self, parent: &gtk::Box, tx: Sender<ExecEvent>, bar_orientation: Orientation) {
        match self.widget_type {
            WidgetType::Box => parent.add(&self.into_box(&tx, bar_orientation)),
            WidgetType::Label => parent.add(&self.into_label()),
            WidgetType::Button => parent.add(&self.into_button(tx, bar_orientation)),
        }
    }

    /// Creates a `gtk::Box` from this widget
    fn into_box(self, tx: &Sender<ExecEvent>, bar_orientation: Orientation) -> gtk::Box {
        let mut builder = gtk::Box::builder();

        if let Some(name) = self.name {
            builder = builder.name(&name);
        }

        if let Some(orientation) = self.orientation {
            builder = builder
                .orientation(try_get_orientation(&orientation).unwrap_or(Orientation::Horizontal));
        }

        let container = builder.build();

        if let Some(class) = self.class {
            container.style_context().add_class(&class);
        }

        if let Some(widgets) = self.widgets {
            widgets
                .into_iter()
                .for_each(|widget| widget.add_to(&container, tx.clone(), bar_orientation));
        }

        container
    }

    /// Creates a `gtk::Label` from this widget
    fn into_label(self) -> Label {
        let mut builder = Label::builder().use_markup(true);

        if let Some(name) = self.name {
            builder = builder.name(&name);
        }

        let label = builder.build();

        if let Some(class) = self.class {
            label.style_context().add_class(&class);
        }

        let text = self.label.map_or_else(String::new, |text| text);

        {
            let label = label.clone();
            DynamicString::new(&text, move |string| {
                label.set_label(&string);
                Continue(true)
            });
        }

        label

        // DynamicString::new(label, &text)
    }

    /// Creates a `gtk::Button` from this widget
    fn into_button(self, tx: Sender<ExecEvent>, bar_orientation: Orientation) -> Button {
        let mut builder = Button::builder();

        if let Some(name) = self.name {
            builder = builder.name(&name);
        }

        let button = builder.build();

        if let Some(text) = self.label {
            let label = Label::new(None);
            label.set_use_markup(true);
            label.set_markup(&text);
            button.add(&label);
        }

        if let Some(class) = self.class {
            button.style_context().add_class(&class);
        }

        if let Some(exec) = self.on_click {
            button.connect_clicked(move |button| {
                tx.try_send(ExecEvent {
                    cmd: exec.clone(),
                    geometry: Popup::button_pos(button, bar_orientation),
                })
                .expect("Failed to send exec message");
            });
        }

        button
    }
}

#[derive(Debug)]
pub struct ExecEvent {
    cmd: String,
    geometry: ButtonGeometry,
}

impl Module<gtk::Box> for CustomModule {
    type SendMessage = ();
    type ReceiveMessage = ExecEvent;

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        spawn(async move {
            while let Some(event) = rx.recv().await {
                if event.cmd.starts_with('!') {
                    let script = Script::from(&event.cmd[1..]);

                    debug!("executing command: '{}'", script.cmd);
                    // TODO: Migrate to use script.run
                    if let Err(err) = script.get_output().await {
                        error!("{err:?}");
                    }
                } else if event.cmd == "popup:toggle" {
                    tx.send(ModuleUpdateEvent::TogglePopup(event.geometry))
                        .await
                        .expect("Failed to send open popup event");
                } else if event.cmd == "popup:open" {
                    tx.send(ModuleUpdateEvent::OpenPopup(event.geometry))
                        .await
                        .expect("Failed to send open popup event");
                } else if event.cmd == "popup:close" {
                    tx.send(ModuleUpdateEvent::ClosePopup)
                        .await
                        .expect("Failed to send open popup event");
                } else {
                    error!("Received invalid command: '{}'", event.cmd);
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleWidget<gtk::Box>> {
        let orientation = info.bar_position.get_orientation();
        let container = gtk::Box::builder().orientation(orientation).build();

        if let Some(ref class) = self.class {
            container.style_context().add_class(class);
        }

        self.bar.clone().into_iter().for_each(|widget| {
            widget.add_to(&container, context.controller_tx.clone(), orientation);
        });

        let popup = self.into_popup(context.controller_tx, context.popup_rx);

        Ok(ModuleWidget {
            widget: container,
            popup,
        })
    }

    fn into_popup(
        self,
        tx: Sender<Self::ReceiveMessage>,
        _rx: glib::Receiver<Self::SendMessage>,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::builder().name("popup-custom").build();

        if let Some(class) = self.class {
            container
                .style_context()
                .add_class(format!("popup-{class}").as_str());
        }

        if let Some(popup) = self.popup {
            popup
                .into_iter()
                .for_each(|widget| widget.add_to(&container, tx.clone(), Orientation::Horizontal));
        }

        Some(container)
    }
}
