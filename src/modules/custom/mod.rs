mod r#box;
mod button;
mod image;
mod label;
mod progress;
mod slider;

use self::image::ImageWidget;
use self::label::LabelWidget;
use self::r#box::BoxWidget;
use self::slider::SliderWidget;
use crate::config::CommonConfig;
use crate::modules::custom::button::ButtonWidget;
use crate::modules::custom::progress::ProgressWidget;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::popup::WidgetGeometry;
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

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Widget {
    Box(BoxWidget),
    Label(LabelWidget),
    Button(ButtonWidget),
    Image(ImageWidget),
    Slider(SliderWidget),
    Progress(ProgressWidget),
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

/// Creates a new widget of type `ty`,
/// setting its name and class based on
/// the values available on `self`.
#[macro_export]
macro_rules! build {
    ($self:ident, $ty:ty) => {{
        let mut builder = <$ty>::builder();

        if let Some(name) = &$self.name {
            builder = builder.name(name);
        }

        let widget = builder.build();

        if let Some(class) = &$self.class {
            widget.style_context().add_class(class);
        }

        widget
    }};
}

/// Sets the widget length,
/// using either a width or height request
/// based on the bar's orientation.
pub fn set_length<W: WidgetExt>(widget: &W, length: i32, bar_orientation: Orientation) {
    match bar_orientation {
        Orientation::Horizontal => widget.set_width_request(length),
        Orientation::Vertical => widget.set_height_request(length),
        _ => {}
    };
}

/// Attempts to parse an `Orientation` from `String`.
/// Will accept `horizontal`, `vertical`, `h` or `v`.
/// Ignores case.
fn try_get_orientation(orientation: &str) -> Result<Orientation> {
    match orientation.to_lowercase().as_str() {
        "horizontal" | "h" => Ok(Orientation::Horizontal),
        "vertical" | "v" => Ok(Orientation::Vertical),
        _ => Err(Report::msg("Invalid orientation string in config")),
    }
}

impl Widget {
    /// Creates this widget and adds it to the parent container
    fn add_to(self, parent: &gtk::Box, context: CustomWidgetContext) {
        match self {
            Widget::Box(widget) => parent.add(&widget.into_widget(context)),
            Widget::Label(widget) => parent.add(&widget.into_widget(context)),
            Widget::Button(widget) => parent.add(&widget.into_widget(context)),
            Widget::Image(widget) => parent.add(&widget.into_widget(context)),
            Widget::Slider(widget) => parent.add(&widget.into_widget(context)),
            Widget::Progress(widget) => parent.add(&widget.into_widget(context)),
        }
    }
}

#[derive(Debug)]
pub struct ExecEvent {
    cmd: String,
    args: Option<Vec<String>>,
    geometry: WidgetGeometry,
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

                    let args = event.args.unwrap_or(vec![]);

                    if let Err(err) = script.get_output(Some(&args)).await {
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
