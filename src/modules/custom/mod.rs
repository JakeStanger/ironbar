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
use crate::modules::{
    wrap_widget, Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, WidgetContext,
};
use crate::script::Script;
use crate::{send_async, spawn};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{Button, IconTheme, Orientation};
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error};

#[derive(Debug, Deserialize, Clone)]
pub struct CustomModule {
    /// Widgets to add to the bar container
    bar: Vec<WidgetConfig>,
    /// Widgets to add to the popup container
    popup: Option<Vec<WidgetConfig>>,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WidgetConfig {
    #[serde(flatten)]
    widget: Widget,
    #[serde(flatten)]
    common: CommonConfig,
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

#[derive(Clone)]
struct CustomWidgetContext<'a> {
    tx: &'a mpsc::Sender<ExecEvent>,
    bar_orientation: Orientation,
    icon_theme: &'a IconTheme,
    popup_buttons: Rc<RefCell<Vec<Button>>>,
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
    fn add_to(self, parent: &gtk::Box, context: &CustomWidgetContext, common: CommonConfig) {
        macro_rules! create {
            ($widget:expr) => {
                wrap_widget(
                    &$widget.into_widget(context.clone()),
                    common,
                    context.bar_orientation,
                )
            };
        }

        let event_box = match self {
            Self::Box(widget) => create!(widget),
            Self::Label(widget) => create!(widget),
            Self::Button(widget) => create!(widget),
            Self::Image(widget) => create!(widget),
            Self::Slider(widget) => create!(widget),
            Self::Progress(widget) => create!(widget),
        };

        parent.add(&event_box);
    }
}

#[derive(Debug)]
pub struct ExecEvent {
    cmd: String,
    args: Option<Vec<String>>,
    id: usize,
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
        tx: mpsc::Sender<ModuleUpdateEvent<Self::SendMessage>>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        spawn(async move {
            while let Some(event) = rx.recv().await {
                if event.cmd.starts_with('!') {
                    let script = Script::from(&event.cmd[1..]);

                    debug!("executing command: '{}'", script.cmd);

                    let args = event.args.unwrap_or_default();

                    if let Err(err) = script.get_output(Some(&args)).await {
                        error!("{err:?}");
                    }
                } else if event.cmd == "popup:toggle" {
                    send_async!(tx, ModuleUpdateEvent::TogglePopup(event.id));
                } else if event.cmd == "popup:open" {
                    send_async!(tx, ModuleUpdateEvent::OpenPopup(event.id));
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
    ) -> Result<ModuleParts<gtk::Box>> {
        let orientation = info.bar_position.get_orientation();
        let container = gtk::Box::builder().orientation(orientation).build();

        let popup_buttons = Rc::new(RefCell::new(Vec::new()));

        let custom_context = CustomWidgetContext {
            tx: &context.controller_tx,
            bar_orientation: orientation,
            icon_theme: info.icon_theme,
            popup_buttons: popup_buttons.clone(),
        };

        self.bar.clone().into_iter().for_each(|widget| {
            widget
                .widget
                .add_to(&container, &custom_context, widget.common);
        });

        let popup = self
            .into_popup(context.controller_tx.clone(), context.subscribe(), info)
            .into_popup_parts_owned(popup_buttons.take());

        Ok(ModuleParts {
            widget: container,
            popup,
        })
    }

    fn into_popup(
        self,
        tx: mpsc::Sender<Self::ReceiveMessage>,
        _rx: broadcast::Receiver<Self::SendMessage>,
        info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        if let Some(popup) = self.popup {
            let custom_context = CustomWidgetContext {
                tx: &tx,
                bar_orientation: info.bar_position.get_orientation(),
                icon_theme: info.icon_theme,
                popup_buttons: Rc::new(RefCell::new(vec![])),
            };

            for widget in popup {
                widget
                    .widget
                    .add_to(&container, &custom_context, widget.common);
            }
        }

        container.show_all();

        Some(container)
    }
}
