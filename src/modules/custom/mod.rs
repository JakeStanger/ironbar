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
use crate::config::{CommonConfig, ModuleConfig};
use crate::modules::custom::button::ButtonWidget;
use crate::modules::custom::progress::ProgressWidget;
use crate::modules::{
    wrap_widget, Module, ModuleInfo, ModuleParts, ModulePopup, ModuleUpdateEvent, WidgetContext,
};
use crate::script::Script;
use crate::{send_async, spawn, Ironbar};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{Button, IconTheme, Orientation};
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error};
use crate::popup::Popup;

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
    widget: WidgetOrModule,
    #[serde(flatten)]
    common: CommonConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
pub enum WidgetOrModule {
    Widget(Widget),
    Module(ModuleConfig),
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
    ironbar: Rc<Ironbar>,
    info: &'a ModuleInfo<'a>,
    popup: Rc<Popup>,
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

impl WidgetOrModule {
    fn add_to(self, parent: &gtk::Box, context: &CustomWidgetContext, common: CommonConfig) {
        match self {
            WidgetOrModule::Widget(widget) => widget.add_to(parent, context, common),
            WidgetOrModule::Module(config) => {
                let ironbar = &context.ironbar;
                let popup = &context.popup;
                let orientation = context.bar_orientation;
                let info = context.info;

                macro_rules! add_module {
                    ($module:expr, $id:expr) => {{
                        let common = $module.common.take().expect("common config to exist");

                        let widget_parts = crate::modules::create_module(
                            *$module,
                            $id,
                            ironbar.clone(),
                            common.name.clone(),
                            &info,
                            &popup,
                        );

                        match widget_parts {
                            Ok(widget_parts) => {
                                crate::modules::set_widget_identifiers(&widget_parts, &common);

                                let container = wrap_widget(&widget_parts.widget, common, orientation);
                                parent.add(&container);
                            }
                            Err(err) => error!("{err:?}")
                        }


                    }};
                }

                let id = Ironbar::unique_id();
                match config {
                    #[cfg(feature = "clipboard")]
                    ModuleConfig::Clipboard(mut module) => add_module!(module, id),
                    #[cfg(feature = "clock")]
                    ModuleConfig::Clock(mut module) => add_module!(module, id),
                    ModuleConfig::Custom(mut module) => add_module!(module, id),
                    #[cfg(feature = "focused")]
                    ModuleConfig::Focused(mut module) => add_module!(module, id),
                    ModuleConfig::Label(mut module) => add_module!(module, id),
                    #[cfg(feature = "launcher")]
                    ModuleConfig::Launcher(mut module) => add_module!(module, id),
                    #[cfg(feature = "lua")]
                    ModuleConfig::Lua(mut module) => add_module!(module, id),
                    #[cfg(feature = "music")]
                    ModuleConfig::Music(mut module) => add_module!(module, id),
                    #[cfg(feature = "notifications")]
                    ModuleConfig::Notifications(mut module) => add_module!(module, id),
                    ModuleConfig::Script(mut module) => add_module!(module, id),
                    #[cfg(feature = "sys_info")]
                    ModuleConfig::SysInfo(mut module) => add_module!(module, id),
                    #[cfg(feature = "tray")]
                    ModuleConfig::Tray(mut module) => add_module!(module, id),
                    #[cfg(feature = "upower")]
                    ModuleConfig::Upower(mut module) => add_module!(module, id),
                    #[cfg(feature = "volume")]
                    ModuleConfig::Volume(mut module) => add_module!(module, id),
                    #[cfg(feature = "workspaces")]
                    ModuleConfig::Workspaces(mut module) => add_module!(module, id),
                }
            }
        }
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
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
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
        let orientation = info.bar_position.orientation();
        let container = gtk::Box::builder().orientation(orientation).build();

        let popup_buttons = Rc::new(RefCell::new(Vec::new()));

        let custom_context = CustomWidgetContext {
            ironbar: context.ironbar.clone(),
            popup: context.popup.clone(),
            info,
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
            .into_popup(context.controller_tx.clone(), context.subscribe(), context, info)
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
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Option<gtk::Box>
    where
        Self: Sized,
    {
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        if let Some(popup) = self.popup {
            let custom_context = CustomWidgetContext {
                ironbar: context.ironbar.clone(),
                popup: context.popup,
                info,
                tx: &tx,
                bar_orientation: info.bar_position.orientation(),
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
