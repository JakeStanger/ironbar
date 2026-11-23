use crate::dynamic_value::{DynamicBool, DynamicString};
use crate::gtk_helpers::{IronbarGtkExt, MouseButton};
use crate::script::{OneshotScriptInput, Script};
use glib::Propagation;
use gtk::prelude::*;
use gtk::{
    EventControllerMotion, EventControllerScroll, EventControllerScrollFlags, Justification,
    Orientation, Revealer, RevealerTransitionType, Widget,
};
use serde::Deserialize;
use std::cell::Cell;
use tracing::trace;

/// The following are module-level options which are present on **all** modules.
///
/// Each module also provides options specific to its type.
/// For details on those, check the relevant module documentation.
///
/// For information on the Script type, and embedding scripts in strings,
/// see [here](script).
/// For information on styling, please see the [styling guide](/guides/styling-guide).
#[derive(Debug, Default, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
pub struct CommonConfig {
    /// Sets the unique widget name,
    /// allowing you to target it in CSS using `#name`.
    ///
    /// It is best practise (although not required) to ensure that the value is
    /// globally unique throughout the Ironbar instance
    /// to avoid clashes.
    pub name: Option<String>,

    /// Sets one or more CSS classes,
    /// allowing you to target it in CSS using `.class`.
    ///
    /// Unlike [name](#name), the `class` property is not expected to be unique.
    pub class: Option<String>,

    /// Shows this text on hover.
    pub tooltip: Option<DynamicString>,

    /// Shows the module only if the dynamic boolean evaluates to true.
    ///
    /// This allows for modules to be dynamically shown or hidden
    /// based on custom events.
    ///
    /// # Example
    ///
    /// ```corn
    /// {
    ///     type = "battery"
    ///     show_if = "ls /sys/class/power_supply/ | grep --quiet '^BAT'"
    /// }
    /// ```
    pub show_if: Option<DynamicBool>,

    /// The transition animation to use when showing/hiding the widget.
    ///
    /// Note this has no effect if `show_if` is not configured.
    #[cfg_attr(feature = "extras", schemars(extend("default" = "slide_start")))]
    pub transition_type: Option<TransitionType>,

    /// The length in milliseconds
    /// of the transition animation to use when showing/hiding the widget.
    ///
    /// Note this has no effect if `show_if` is not configured.
    #[cfg_attr(feature = "extras", schemars(extend("default" = 250)))]
    pub transition_duration: Option<u32>,

    /// A oneshot [script](scripts) to run when the module is left-clicked.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_click_left = "echo 'event' >> log.txt" }
    /// ```
    pub on_click_left: Option<OneshotScriptInput>,

    /// A oneshot [script](scripts) to run when the module is right-clicked.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_click_right = "echo 'event' >> log.txt" }
    /// ```
    pub on_click_right: Option<OneshotScriptInput>,

    /// A oneshot [script](scripts) to run when the module is middle-clicked.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_click_middle = "echo 'event' >> log.txt" }
    /// ```
    pub on_click_middle: Option<OneshotScriptInput>,

    /// A oneshot [script](scripts) to run when the module is double-left-clicked.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_click_left_double = "echo 'double click' >> log.txt" }
    /// ```
    pub on_click_left_double: Option<OneshotScriptInput>,

    /// A oneshot [script](scripts) to run when the module is double-right-clicked.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_click_right_double = "echo 'double click' >> log.txt" }
    /// ```
    pub on_click_right_double: Option<OneshotScriptInput>,

    /// A oneshot [script](scripts) to run when the module is double-middle-clicked.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_click_middle_double = "echo 'double click' >> log.txt" }
    /// ```
    pub on_click_middle_double: Option<OneshotScriptInput>,

    /// A oneshot [script](scripts) to run when the module is scrolled up on.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_scroll_up = "echo 'event' >> log.txt" }
    /// ```
    pub on_scroll_up: Option<OneshotScriptInput>,

    /// A oneshot [script](scripts) to run when the module is scrolled down on.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_scroll_down = "echo 'event' >> log.txt" }
    /// ```
    pub on_scroll_down: Option<OneshotScriptInput>,

    /// A multiplier from `0.0` - `10.0` to control the speed
    /// of smooth scrolling on trackpad.
    #[serde(default = "default_smooth_scroll_speed")]
    pub smooth_scroll_speed: f64,

    /// A oneshot [script](scripts) to run when the cursor begins hovering over the module.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_mouse_enter = "echo 'event' >> log.txt" }
    /// ```
    pub on_mouse_enter: Option<OneshotScriptInput>,

    /// A oneshot [script](scripts) to run when the cursor stops hovering over the module.
    ///
    /// # Example
    ///
    /// ```corn
    /// { on_mouse_exit = "echo 'event' >> log.txt" }
    /// ```
    pub on_mouse_exit: Option<OneshotScriptInput>,

    /// Prevents the popup from opening on-click for this widget.
    #[serde(default)]
    pub disable_popup: bool,
}

fn default_smooth_scroll_speed() -> f64 {
    1.0
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
pub enum TransitionType {
    None,
    Crossfade,
    SlideStart,
    SlideEnd,
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
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

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
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
        const SMOOTH_SCROLL_REQUIRED_DELTA: f64 = 10.0;

        self.install_show_if(container, revealer);

        // Helper to install click handlers with optional double-click support
        let install_click_handler =
            |button: MouseButton,
             single: Option<OneshotScriptInput>,
             double: Option<OneshotScriptInput>,
             button_name: &'static str| {
                let single = single.map(Script::new_oneshot);
                let double = double.map(Script::new_oneshot);

                if single.is_some() || double.is_some() {
                    container.connect_pressed_with_double_click(
                        button,
                        move || {
                            if let Some(script) = &single {
                                trace!("Running on-click script: {}", button_name);
                                script.run_as_oneshot(None);
                            }
                        },
                        double.map(|script| {
                            move || {
                                trace!("Running on-double-click script: {}", button_name);
                                script.run_as_oneshot(None);
                            }
                        }),
                    );
                }
            };

        install_click_handler(
            MouseButton::Primary,
            self.on_click_left,
            self.on_click_left_double,
            "left",
        );
        install_click_handler(
            MouseButton::Middle,
            self.on_click_middle,
            self.on_click_middle_double,
            "middle",
        );
        install_click_handler(
            MouseButton::Secondary,
            self.on_click_right,
            self.on_click_right_double,
            "right",
        );

        let event_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);

        let scroll_up_script = self.on_scroll_up.map(Script::new_oneshot);
        let scroll_down_script = self.on_scroll_down.map(Script::new_oneshot);

        let scroll_speed = self.smooth_scroll_speed;
        let curr_scroll = Cell::new(0.0);

        event_controller.connect_scroll(move |_, _dx, dy| {
            let script = if dy > 0.0 {
                scroll_down_script.as_ref()
            } else {
                scroll_up_script.as_ref()
            };

            let is_smooth_scroll = dy.fract() != 0.0;

            let should_run = if is_smooth_scroll {
                let delta = dy * scroll_speed;
                curr_scroll.set(curr_scroll.get() + delta);

                if curr_scroll.get() >= SMOOTH_SCROLL_REQUIRED_DELTA {
                    curr_scroll.set(curr_scroll.get() - SMOOTH_SCROLL_REQUIRED_DELTA);
                    true
                } else if curr_scroll.get() <= -SMOOTH_SCROLL_REQUIRED_DELTA {
                    curr_scroll.set(curr_scroll.get() + SMOOTH_SCROLL_REQUIRED_DELTA);
                    true
                } else {
                    false
                }
            } else {
                true
            };

            if let Some(script) = script
                && should_run
            {
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

        if let Some(script) = self.on_mouse_enter.map(Script::new_oneshot) {
            event_controller.connect_enter(move |_, _, _| {
                script.run_as_oneshot(None);
            });
        }

        if let Some(script) = self.on_mouse_exit.map(Script::new_oneshot) {
            event_controller.connect_leave(move |_| {
                script.run_as_oneshot(None);
            });
        }

        if let Some(tooltip) = self.tooltip {
            tooltip.subscribe(container, move |container, string| {
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
            // need to keep clone here for the notify callback
            let container = container.clone();

            show_if.subscribe((revealer, &container), |(revealer, container), success| {
                if success {
                    container.set_visible(true);
                }
                revealer.set_reveal_child(success);
            });

            revealer.connect_child_revealed_notify(move |revealer| {
                if !revealer.reveals_child() {
                    container.set_visible(false);
                }
            });
        } else {
            revealer.set_visible(true);
        }
    }
}
