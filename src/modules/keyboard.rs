use std::collections::HashMap;

use color_eyre::Result;
use color_eyre::eyre::Report;
use gtk::{Button, prelude::*};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{debug, trace};

use super::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::clients::compositor::{self, KeyboardLayoutUpdate};
use crate::clients::libinput::{Event, Key, KeyEvent};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::IconLabel;
use crate::{glib_recv, module_impl, module_update, send_async, spawn, try_send};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KeyboardModule {
    /// Whether to show capslock indicator.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    show_caps: bool,

    /// Whether to show num lock indicator.
    ///
    ///  **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    show_num: bool,

    /// Whether to show scroll lock indicator.
    ///
    ///  **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    show_scroll: bool,

    /// Whether to show the current keyboard layout.
    ///
    ///  **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    show_layout: bool,

    /// Size to render the icons at, in pixels (image icons only).
    ///
    /// **Default** `32`
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    /// Player state icons.
    ///
    /// See [icons](#icons).
    #[serde(default)]
    icons: Icons,

    /// The Wayland seat to attach to.
    /// You almost certainly do not need to change this.
    ///
    /// **Default**: `seat0`
    #[serde(default = "default_seat")]
    seat: String,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
struct Icons {
    /// Icon to show when capslock is enabled.
    ///
    /// **Default**: `󰪛`
    #[serde(default = "default_icon_caps")]
    caps_on: String,

    /// Icon to show when capslock is disabled.
    ///
    /// **Default**: `""`
    #[serde(default)]
    caps_off: String,

    /// Icon to show when num lock is enabled.
    ///
    /// **Default**: ``
    #[serde(default = "default_icon_num")]
    num_on: String,

    /// Icon to show when num lock is disabled.
    ///
    /// **Default**: `""`
    #[serde(default)]
    num_off: String,

    /// Icon to show when scroll lock is enabled.
    ///
    /// **Default**: ``
    #[serde(default = "default_icon_scroll")]
    scroll_on: String,

    /// Icon to show when scroll lock is disabled.
    ///
    /// **Default**: `""`
    #[serde(default)]
    scroll_off: String,

    /// Map of icons or labels to show for a particular keyboard layout.
    ///
    /// If a layout is not present in the map,
    /// it will fall back to using its actual name.
    ///
    /// **Default**: `{}`
    ///
    /// # Example
    ///
    /// ```corn
    /// {
    ///   type = "keyboard"
    ///   show_layout = true
    ///   icons.layout_map.'English (US)' = "EN"
    ///   icons.layout_map.Ukrainian = "UA"
    /// }
    /// ```
    #[serde(default)]
    layout_map: HashMap<String, String>,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            caps_on: default_icon_caps(),
            caps_off: String::new(),
            num_on: default_icon_num(),
            num_off: String::new(),
            scroll_on: default_icon_scroll(),
            scroll_off: String::new(),
            layout_map: HashMap::new(),
        }
    }
}

const fn default_icon_size() -> i32 {
    32
}

fn default_seat() -> String {
    String::from("seat0")
}

fn default_icon_caps() -> String {
    String::from("󰪛")
}

fn default_icon_num() -> String {
    String::from("")
}

fn default_icon_scroll() -> String {
    String::from("")
}

#[derive(Debug, Clone)]
pub enum KeyboardUpdate {
    Key(KeyEvent),
    Layout(KeyboardLayoutUpdate),
}

impl Module<gtk::Box> for KeyboardModule {
    type SendMessage = KeyboardUpdate;
    type ReceiveMessage = ();

    module_impl!("keyboard");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let client = context.ironbar.clients.borrow_mut().libinput(&self.seat)?;

        let tx = context.tx.clone();
        spawn(async move {
            let mut rx = client.subscribe();
            while let Ok(ev) = rx.recv().await {
                match ev {
                    Event::Device => {
                        for key in [Key::Caps, Key::Num, Key::Scroll] {
                            let event = KeyEvent {
                                key,
                                state: client.get_state(key),
                            };
                            module_update!(tx, KeyboardUpdate::Key(event));
                        }
                    }
                    Event::Key(ev) => {
                        module_update!(tx, KeyboardUpdate::Key(ev));
                    }
                }
            }
        });

        let client = context.try_client::<dyn compositor::KeyboardLayoutClient>()?;
        {
            let client = client.clone();
            let tx = context.tx.clone();
            spawn(async move {
                let mut srx = client.subscribe();

                trace!("Set up keyboard_layout subscription");

                loop {
                    match srx.recv().await {
                        Ok(payload) => {
                            debug!("Received update: {payload:?}");
                            module_update!(tx, KeyboardUpdate::Layout(payload));
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                            tracing::warn!(
                                "Channel lagged behind by {count}, this may result in unexpected or broken behaviour"
                            );
                        }
                        Err(err) => {
                            tracing::error!("{err:?}");
                            break;
                        }
                    };
                }
            });
        }

        // Change keyboard layout
        spawn(async move {
            trace!("Setting up keyboard_layout UI event handler");

            while let Some(()) = rx.recv().await {
                client.set_next_active();
            }

            Ok::<(), Report>(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        let container = gtk::Box::new(info.bar_position.orientation(), 0);

        let caps = IconLabel::new(&self.icons.caps_off, info.icon_theme, self.icon_size);
        let num = IconLabel::new(&self.icons.num_off, info.icon_theme, self.icon_size);
        let scroll = IconLabel::new(&self.icons.scroll_off, info.icon_theme, self.icon_size);

        let layout_button = Button::new();
        let layout = IconLabel::new("", info.icon_theme, self.icon_size);
        layout_button.add(&*layout);

        if self.show_caps {
            caps.add_class("key");
            caps.add_class("caps");
            container.add(&*caps);
        }

        if self.show_num {
            num.add_class("key");
            num.add_class("num");
            container.add(&*num);
        }

        if self.show_scroll {
            scroll.add_class("key");
            scroll.add_class("scroll");
            container.add(&*scroll);
        }

        if self.show_layout {
            layout.add_class("layout");
            container.add(&layout_button);
        }

        {
            let tx = context.controller_tx.clone();
            layout_button.connect_clicked(move |_| {
                try_send!(tx, ());
            });
        }

        let icons = self.icons;
        let handle_event = move |ev: KeyboardUpdate| match ev {
            KeyboardUpdate::Key(ev) => {
                let parts = match (ev.key, ev.state) {
                    (Key::Caps, true) if self.show_caps => Some((&caps, icons.caps_on.as_str())),
                    (Key::Caps, false) if self.show_caps => Some((&caps, icons.caps_off.as_str())),
                    (Key::Num, true) if self.show_num => Some((&num, icons.num_on.as_str())),
                    (Key::Num, false) if self.show_num => Some((&num, icons.num_off.as_str())),
                    (Key::Scroll, true) if self.show_scroll => {
                        Some((&scroll, icons.scroll_on.as_str()))
                    }
                    (Key::Scroll, false) if self.show_scroll => {
                        Some((&scroll, icons.scroll_off.as_str()))
                    }
                    _ => None,
                };

                if let Some((label, input)) = parts {
                    label.set_label(Some(input));

                    if ev.state {
                        label.add_class("enabled");
                    } else {
                        label.remove_class("enabled");
                    }
                }
            }
            KeyboardUpdate::Layout(KeyboardLayoutUpdate(language)) => {
                let text = icons.layout_map.get(&language).unwrap_or(&language);
                layout.set_label(Some(text));
            }
        };

        glib_recv!(context.subscribe(), handle_event);
        Ok(ModuleParts::new(container, None))
    }
}
