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
use crate::channels::AsyncSenderExt;
use crate::config::{CommonConfig, ModuleConfig};
use crate::modules::custom::button::ButtonWidget;
use crate::modules::custom::progress::ProgressWidget;
use crate::modules::{
    wrap_widget, AnyModuleFactory, BarModuleFactory, Module, ModuleInfo, ModuleParts, ModulePopup,
    ModuleUpdateEvent, PopupButton, PopupModuleFactory, WidgetContext,
};
use crate::script::Script;
use crate::{module_impl, spawn};
use color_eyre::Result;
use gtk::prelude::*;
use gtk::{Button, IconTheme, Orientation};
use serde::Deserialize;
use std::cell::RefCell;
use std::rc::Rc;
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CustomModule {
    /// Modules and widgets to add to the bar container.
    ///
    /// **Default**: `[]`
    bar: Vec<WidgetConfig>,

    /// Modules and widgets to add to the popup container.
    ///
    /// **Default**: `null`
    popup: Option<Vec<WidgetConfig>>,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WidgetConfig {
    /// One of a custom module native Ironbar module.
    #[serde(flatten)]
    widget: WidgetOrModule,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    common: CommonConfig,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum WidgetOrModule {
    /// A custom-module specific basic widget
    Widget(Widget),
    /// A native Ironbar module, such as `clock` or `focused`.
    /// All widgets are supported, including their popups.
    Module(ModuleConfig),
}

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Widget {
    /// A container to place nested widgets inside.
    Box(BoxWidget),
    /// A text label. Pango markup is supported.
    Label(LabelWidget),
    /// A clickable button, which can run a command when clicked.
    Button(ButtonWidget),
    /// An image or icon from disk or http.
    Image(ImageWidget),
    /// A draggable slider.
    Slider(SliderWidget),
    /// A progress bar.
    Progress(ProgressWidget),
}

#[derive(Clone)]
struct CustomWidgetContext<'a> {
    info: &'a ModuleInfo<'a>,
    tx: &'a mpsc::Sender<ExecEvent>,
    bar_orientation: Orientation,
    icon_theme: &'a IconTheme,
    popup_buttons: Rc<RefCell<Vec<Button>>>,
    module_factory: AnyModuleFactory,
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

impl WidgetOrModule {
    fn add_to(self, parent: &gtk::Box, context: &CustomWidgetContext, common: CommonConfig) {
        match self {
            WidgetOrModule::Widget(widget) => widget.add_to(parent, context, common),
            WidgetOrModule::Module(config) => {
                if let Err(err) = config.create(&context.module_factory, parent, context.info) {
                    error!("{err:?}");
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

    module_impl!("custom");

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
                    tx.send_expect(ModuleUpdateEvent::TogglePopup(event.id))
                        .await;
                } else if event.cmd == "popup:open" {
                    tx.send_expect(ModuleUpdateEvent::OpenPopup(event.id)).await;
                } else if event.cmd == "popup:close" {
                    tx.send_expect(ModuleUpdateEvent::ClosePopup).await;
                } else {
                    error!("Received invalid command: '{}'", event.cmd);
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        mut context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        let orientation = info.bar_position.orientation();
        let container = gtk::Box::builder().orientation(orientation).build();

        let popup_buttons = Rc::new(RefCell::new(Vec::new()));

        let custom_context = CustomWidgetContext {
            info,
            tx: &context.controller_tx,
            bar_orientation: orientation,
            icon_theme: info.icon_theme,
            popup_buttons: popup_buttons.clone(),
            module_factory: BarModuleFactory::new(context.ironbar.clone(), context.popup.clone())
                .into(),
        };

        self.bar.clone().into_iter().for_each(|widget| {
            widget
                .widget
                .add_to(&container, &custom_context, widget.common);
        });

        for button in popup_buttons.borrow().iter() {
            button.ensure_popup_id();
        }

        context.button_id = popup_buttons
            .borrow()
            .first()
            .map_or(usize::MAX, PopupButton::popup_id);

        let popup = self
            .into_popup(
                context.controller_tx.clone(),
                context.subscribe(),
                context,
                info,
            )
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
                info,
                tx: &tx,
                bar_orientation: info.bar_position.orientation(),
                icon_theme: info.icon_theme,
                popup_buttons: Rc::new(RefCell::new(vec![])),
                module_factory: PopupModuleFactory::new(
                    context.ironbar,
                    context.popup,
                    context.button_id,
                )
                .into(),
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
