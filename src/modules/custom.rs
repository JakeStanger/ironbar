use crate::config::CommonConfig;
use crate::dynamic_string::DynamicString;
use crate::image::ImageProvider;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::popup::{ButtonGeometry, Popup};
use crate::script::Script;
use crate::{send_async, try_send};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{Button, IconTheme, Label, Orientation};
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
    pub common: Option<CommonConfig>,
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
    src: Option<String>,
    size: Option<i32>,
}

/// Supported GTK widget types
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    Box,
    Label,
    Button,
    Image,
}

impl Widget {
    /// Creates this widget and adds it to the parent container
    fn add_to(
        self,
        parent: &gtk::Box,
        tx: Sender<ExecEvent>,
        bar_orientation: Orientation,
        icon_theme: &IconTheme,
    ) {
        match self.widget_type {
            WidgetType::Box => parent.add(&self.into_box(&tx, bar_orientation, icon_theme)),
            WidgetType::Label => parent.add(&self.into_label()),
            WidgetType::Button => parent.add(&self.into_button(tx, bar_orientation)),
            WidgetType::Image => parent.add(&self.into_image(icon_theme)),
        }
    }

    /// Creates a `gtk::Box` from this widget
    fn into_box(
        self,
        tx: &Sender<ExecEvent>,
        bar_orientation: Orientation,
        icon_theme: &IconTheme,
    ) -> gtk::Box {
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
            widgets.into_iter().for_each(|widget| {
                widget.add_to(&container, tx.clone(), bar_orientation, icon_theme)
            });
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
                try_send!(
                    tx,
                    ExecEvent {
                        cmd: exec.clone(),
                        geometry: Popup::button_pos(button, bar_orientation),
                    }
                );
            });
        }

        button
    }

    fn into_image(self, icon_theme: &IconTheme) -> gtk::Image {
        let mut builder = gtk::Image::builder();

        if let Some(name) = self.name {
            builder = builder.name(&name);
        }

        let gtk_image = builder.build();

        if let Some(src) = self.src {
            let size = self.size.unwrap_or(32);
            if let Err(err) = ImageProvider::parse(src, icon_theme, size)
                .and_then(|image| image.load_into_image(gtk_image.clone()))
            {
                error!("{err:?}");
            }
        }

        if let Some(class) = self.class {
            gtk_image.style_context().add_class(&class);
        }

        gtk_image
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

    fn name() -> &'static str {
        "custom"
    }

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
                    send_async!(tx, ModuleUpdateEvent::TogglePopup(event.geometry));
                } else if event.cmd == "popup:open" {
                    send_async!(tx, ModuleUpdateEvent::OpenPopup(event.geometry));
                } else if event.cmd == "popup:close" {
                    send_async!(tx, ModuleUpdateEvent::ClosePopup);
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
            widget.add_to(
                &container,
                context.controller_tx.clone(),
                orientation,
                info.icon_theme,
            );
        });

        let popup = self.into_popup(context.controller_tx, context.popup_rx, info);

        Ok(ModuleWidget {
            widget: container,
            popup,
        })
    }

    fn into_popup(
        self,
        tx: Sender<Self::ReceiveMessage>,
        _rx: glib::Receiver<Self::SendMessage>,
        info: &ModuleInfo,
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
            popup.into_iter().for_each(|widget| {
                widget.add_to(
                    &container,
                    tx.clone(),
                    Orientation::Horizontal,
                    info.icon_theme,
                )
            });
        }

        Some(container)
    }
}
