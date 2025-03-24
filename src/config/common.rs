use crate::dynamic_value::{DynamicBool, dynamic_string};
use crate::script::{Script, ScriptInput};
use glib::Propagation;
use gtk::gdk::{BUTTON_MIDDLE, BUTTON_PRIMARY, BUTTON_SECONDARY, ScrollDirection};
use gtk::prelude::*;
use gtk::{
    EventControllerMotion, EventControllerScroll, EventControllerScrollFlags, GestureClick,
    Justification, Orientation, Revealer, RevealerTransitionType, Widget,
};
use serde::Deserialize;
use tracing::trace;

/// The following are module-level options which are present on **all** modules.
///
/// Each module also provides options specific to its type.
/// For details on those, check the relevant module documentation.
///
/// For information on the Script type, and embedding scripts in strings,
/// see [here](script).
/// For information on styling, please see the [styling guide](styling-guide).
#[derive(Debug, Default, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CommonConfig {
    /// Sets the unique widget name,
    /// allowing you to target it in CSS using `#name`.
    ///
    /// It is best practise (although not required) to ensure that the value is
    /// globally unique throughout the Ironbar instance
    /// to avoid clashes.
    ///
    /// **Default**: `null`
    pub name: Option<String>,

    /// Sets one or more CSS classes,
    /// allowing you to target it in CSS using `.class`.
    ///
    /// Unlike [name](#name), the `class` property is not expected to be unique.
    ///
    /// **Default**: `null`
    pub class: Option<String>,

    /// Shows this text on hover.
    /// Supports embedding scripts between `{{double braces}}`.
    ///
    /// Note that full dynamic string support is not currently supported.
    ///
    /// **Default**: `null`
    pub tooltip: Option<String>,

    /// Shows the module only if the dynamic boolean evaluates to true.
    ///
    /// This allows for modules to be dynamically shown or hidden
    /// based on custom events.
    ///
    /// **Default**: `null`
    pub show_if: Option<DynamicBool>,

    /// The transition animation to use when showing/hiding the widget.
    ///
    /// Note this has no effect if `show_if` is not configured.
    ///
    /// **Valid options**: `slide_start`, `slide_end`, `crossfade`, `none`
    /// <br>
    /// **Default**: `slide_start`
    pub transition_type: Option<TransitionType>,

    /// The length in milliseconds
    /// of the transition animation to use when showing/hiding the widget.
    ///
    /// Note this has no effect if `show_if` is not configured.
    ///
    /// **Default**: `250`
    pub transition_duration: Option<u32>,

    /// A [script](scripts) to run when the module is left-clicked.
    ///
    /// **Supported script types**: `oneshot`.
    /// <br>
    /// **Default**: `null`
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_click_left = "echo 'event' >> log.txt" }
    /// ```
    pub on_click_left: Option<ScriptInput>,

    /// A [script](scripts) to run when the module is right-clicked.
    ///
    /// **Supported script types**: `oneshot`.
    /// <br>
    /// **Default**: `null`
    /// /// # Example
    ///
    /// ```corn
    /// { on_click_right = "echo 'event' >> log.txt" }
    /// ```
    pub on_click_right: Option<ScriptInput>,

    /// A [script](scripts) to run when the module is middle-clicked.
    ///
    /// **Supported script types**: `oneshot`.
    /// <br>
    /// **Default**: `null`
    /// # Example
    ///
    /// ```corn
    /// { on_click_middle = "echo 'event' >> log.txt" }
    /// ```
    pub on_click_middle: Option<ScriptInput>,

    /// A [script](scripts) to run when the module is scrolled up on.
    ///
    /// **Supported script types**: `oneshot`.
    /// <br>
    /// **Default**: `null`
    /// # Example
    ///
    /// ```corn
    /// { on_scroll_up = "echo 'event' >> log.txt" }
    /// ```
    pub on_scroll_up: Option<ScriptInput>,

    /// A [script](scripts) to run when the module is scrolled down on.
    ///
    /// **Supported script types**: `oneshot`.
    /// <br>
    /// **Default**: `null`
    /// # Example
    ///
    /// ```corn
    /// { on_scroll_down = "echo 'event' >> log.txt" }
    /// ```
    pub on_scroll_down: Option<ScriptInput>,

    /// A [script](scripts) to run when the cursor begins hovering over the module.
    ///
    /// **Supported script types**: `oneshot`.
    /// <br>
    /// **Default**: `null`
    /// # Example
    ///
    /// ```corn
    /// { on_mouse_enter = "echo 'event' >> log.txt" }
    /// ```
    pub on_mouse_enter: Option<ScriptInput>,

    /// A [script](scripts) to run when the cursor stops hovering over the module.
    ///
    /// **Supported script types**: `oneshot`.
    /// <br>
    /// **Default**: `null`
    /// # Example
    ///
    /// ```corn
    /// { on_mouse_exit = "echo 'event' >> log.txt" }
    /// ```
    pub on_mouse_exit: Option<ScriptInput>,

    /// Prevents the popup from opening on-click for this widget.
    #[serde(default)]
    pub disable_popup: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum TransitionType {
    None,
    Crossfade,
    SlideStart,
    SlideEnd,
}

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ModuleOrientation {
    #[default]
    #[serde(alias = "h")]
    Horizontal,
    #[serde(alias = "v")]
    Vertical,
}

impl ModuleOrientation {
    pub const fn to_angle(self) -> f64 {
        match self {
            Self::Horizontal => 0.0,
            Self::Vertical => 90.0,
        }
    }
}

impl From<ModuleOrientation> for Orientation {
    fn from(o: ModuleOrientation) -> Self {
        match o {
            ModuleOrientation::Horizontal => Self::Horizontal,
            ModuleOrientation::Vertical => Self::Vertical,
        }
    }
}

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ModuleJustification {
    #[default]
    Left,
    Right,
    Center,
    Fill,
}

impl From<ModuleJustification> for Justification {
    fn from(o: ModuleJustification) -> Self {
        match o {
            ModuleJustification::Left => Self::Left,
            ModuleJustification::Right => Self::Right,
            ModuleJustification::Center => Self::Center,
            ModuleJustification::Fill => Self::Fill,
        }
    }
}

impl TransitionType {
    pub const fn to_revealer_transition_type(
        &self,
        orientation: Orientation,
    ) -> RevealerTransitionType {
        match (self, orientation) {
            (Self::SlideStart, Orientation::Horizontal) => RevealerTransitionType::SlideLeft,
            (Self::SlideStart, Orientation::Vertical) => RevealerTransitionType::SlideUp,
            (Self::SlideEnd, Orientation::Horizontal) => RevealerTransitionType::SlideRight,
            (Self::SlideEnd, Orientation::Vertical) => RevealerTransitionType::SlideDown,
            (Self::Crossfade, _) => RevealerTransitionType::Crossfade,
            _ => RevealerTransitionType::None,
        }
    }
}

impl CommonConfig {
    /// Configures the module's container according to the common config options.
    pub fn install_events<W>(mut self, container: &W, revealer: &Revealer)
    where
        W: IsA<Widget>,
    {
        self.install_show_if(container, revealer);

        if let Some(script) = self.on_click_left.map(Script::new_polling) {
            let event_controller = GestureClick::new();
            event_controller.set_button(BUTTON_PRIMARY);

            event_controller.connect_pressed(move |_, _, _, _| {
                trace!("Running on-click script: left");
                script.run_as_oneshot(None);
            });
        };

        if let Some(script) = self.on_click_middle.map(Script::new_polling) {
            let event_controller = GestureClick::new();
            event_controller.set_button(BUTTON_MIDDLE);

            event_controller.connect_pressed(move |_, _, _, _| {
                trace!("Running on-click script: middle");
                script.run_as_oneshot(None);
            });
        };

        if let Some(script) = self.on_click_right.map(Script::new_polling) {
            let event_controller = GestureClick::new();
            event_controller.set_button(BUTTON_SECONDARY);

            event_controller.connect_pressed(move |_, _, _, _| {
                trace!("Running on-click script: right");
                script.run_as_oneshot(None);
            });
        };

        let event_controller = EventControllerScroll::new(EventControllerScrollFlags::all());

        let scroll_up_script = self.on_scroll_up.map(Script::new_polling);
        let scroll_down_script = self.on_scroll_down.map(Script::new_polling);

        event_controller.connect_scroll(move |_, _dx, dy| {
            let script = if dy > 0.0 {
                scroll_down_script.as_ref()
            } else {
                scroll_up_script.as_ref()
            };

            if let Some(script) = script {
                trace!(
                    "Running on-scroll script: {}",
                    if dy > 0.0 { "down" } else { "up" }
                );
                script.run_as_oneshot(None);
            }

            Propagation::Proceed
        });

        container.add_controller(event_controller);

        let event_controller = EventControllerMotion::new();

        if let Some(script) = self.on_mouse_enter.map(Script::new_polling) {
            event_controller.connect_enter(move |_, _, _| {
                script.run_as_oneshot(None);
            });
        }

        if let Some(script) = self.on_mouse_exit.map(Script::new_polling) {
            event_controller.connect_leave(move |_| {
                script.run_as_oneshot(None);
            });
        }

        if let Some(tooltip) = self.tooltip {
            let container = container.clone();
            dynamic_string(&tooltip, move |string| {
                container.set_tooltip_text(Some(&string));
            });
        }

        container.add_controller(event_controller);
    }

    fn install_show_if<W>(&mut self, container: &W, revealer: &Revealer)
    where
        W: IsA<Widget>,
    {
        if let Some(show_if) = self.show_if.take() {
            let container = container.clone();

            {
                let revealer = revealer.clone();
                let container = container.clone();

                show_if.subscribe(move |success| {
                    if success {
                        container.show();
                    }
                    revealer.set_reveal_child(success);
                });
            }

            revealer.connect_child_revealed_notify(move |revealer| {
                if !revealer.reveals_child() {
                    container.hide();
                }
            });
        } else {
            revealer.show();
        }
    }
}
