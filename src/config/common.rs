use crate::dynamic_value::{DynamicBool, dynamic_string};
use crate::script::{Script, ScriptInput};
use glib::Propagation;
use gtk::gdk::ScrollDirection;
use gtk::prelude::*;
use gtk::{EventBox, Justification, Orientation, Revealer, RevealerTransitionType};
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
    pub fn install_events(mut self, container: &EventBox, revealer: &Revealer) {
        self.install_show_if(container, revealer);

        let left_click_script = self.on_click_left.map(Script::new_polling);
        let middle_click_script = self.on_click_middle.map(Script::new_polling);
        let right_click_script = self.on_click_right.map(Script::new_polling);

        container.connect_button_press_event(move |_, event| {
            let script = match event.button() {
                1 => left_click_script.as_ref(),
                2 => middle_click_script.as_ref(),
                3 => right_click_script.as_ref(),
                _ => None,
            };

            if let Some(script) = script {
                trace!("Running on-click script: {}", event.button());
                script.run_as_oneshot(None);
            }

            Propagation::Proceed
        });

        let scroll_up_script = self.on_scroll_up.map(Script::new_polling);
        let scroll_down_script = self.on_scroll_down.map(Script::new_polling);

        container.connect_scroll_event(move |_, event| {
            let script = match event.direction() {
                ScrollDirection::Up => scroll_up_script.as_ref(),
                ScrollDirection::Down => scroll_down_script.as_ref(),
                ScrollDirection::Smooth => {
                    if event.scroll_deltas().unwrap_or_default().1 > 0.0 {
                        scroll_down_script.as_ref()
                    } else {
                        scroll_up_script.as_ref()
                    }
                }
                _ => None,
            };

            if let Some(script) = script {
                trace!("Running on-scroll script: {}", event.direction());
                script.run_as_oneshot(None);
            }

            Propagation::Proceed
        });

        macro_rules! install_oneshot {
            ($option:expr, $method:ident) => {
                $option.map(Script::new_polling).map(|script| {
                    container.$method(move |_, _| {
                        script.run_as_oneshot(None);
                        Propagation::Proceed
                    });
                })
            };
        }

        install_oneshot!(self.on_mouse_enter, connect_enter_notify_event);
        install_oneshot!(self.on_mouse_exit, connect_leave_notify_event);

        if let Some(tooltip) = self.tooltip {
            dynamic_string(&tooltip, container, move |container, string| {
                container.set_tooltip_text(Some(&string));
            });
        }
    }

    fn install_show_if(&mut self, container: &EventBox, revealer: &Revealer) {
        self.show_if.take().map_or_else(
            || {
                container.show_all();
            },
            |show_if| {
                // need to keep clone here for the notify callback
                let container = container.clone();

                show_if.subscribe((revealer, &container), |(revealer, container), success| {
                    if success {
                        container.show_all();
                    }
                    revealer.set_reveal_child(success);
                });

                revealer.connect_child_revealed_notify(move |revealer| {
                    if !revealer.reveals_child() {
                        container.hide();
                    }
                });
            },
        );
    }
}
