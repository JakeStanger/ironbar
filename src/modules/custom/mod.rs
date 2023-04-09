mod r#box;
mod button;
mod image;
mod label;

use self::image::ImageWidget;
use self::label::LabelWidget;
use self::r#box::BoxWidget;
use crate::config::CommonConfig;
use crate::modules::custom::button::ButtonWidget;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::popup::ButtonGeometry;
use crate::script::Script;
use crate::send_async;
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{IconTheme, Orientation};
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

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Widget {
    Box(BoxWidget),
    Label(LabelWidget),
    Button(ButtonWidget),
    Image(ImageWidget)
}

#[derive(Clone, Copy)]
struct CustomWidgetContext<'a> {
    tx: &'a Sender<ExecEvent>,
    bar_orientation: Orientation,
    icon_theme: &'a IconTheme,
}

trait CustomWidget {
    type Widget;

    fn into_widget(self, context: CustomWidgetContext) -> Self::Widget;
}

impl Widget {
    /// Creates this widget and adds it to the parent container
    fn add_to(self, parent: &gtk::Box, context: CustomWidgetContext) {
        match self {
            Widget::Box(widget) => parent.add(&widget.into_widget(context)),
            Widget::Label(widget) => parent.add(&widget.into_widget(context)),
            Widget::Button(widget) => parent.add(&widget.into_widget(context)),
            Widget::Image(widget) => parent.add(&widget.into_widget(context)),
        }
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

        let custom_context = CustomWidgetContext {
            tx: &context.controller_tx,
            bar_orientation: orientation,
            icon_theme: info.icon_theme,
        };

        self.bar.clone().into_iter().for_each(|widget| {
            widget.add_to(&container, custom_context);
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
            let custom_context = CustomWidgetContext {
                tx: &tx,
                bar_orientation: info.bar_position.get_orientation(),
                icon_theme: info.icon_theme,
            };

            for widget in popup {
                widget.add_to(&container, custom_context);
            }
        }

        container.show_all();

        Some(container)
    }
}