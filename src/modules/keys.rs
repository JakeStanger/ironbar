use color_eyre::Result;
use gtk::prelude::*;
use serde::Deserialize;
use std::ops::Deref;
use tokio::sync::mpsc;

use super::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::libinput::{Event, Key, KeyEvent};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::IconLabel;
use crate::{module_impl, spawn};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct KeysModule {
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

impl Module<gtk::Box> for KeysModule {
    type SendMessage = KeyEvent;
    type ReceiveMessage = ();

    module_impl!("keys");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let client = context.ironbar.clients.borrow_mut().libinput(&self.seat);

        let tx = context.tx.clone();
        spawn(async move {
            let mut rx = client.subscribe();
            while let Ok(ev) = rx.recv().await {
                match ev {
                    Event::Device => {
                        for key in [Key::Caps, Key::Num, Key::Scroll] {
                            tx.send_update(KeyEvent {
                                key: Key::Caps,
                                state: client.get_state(key),
                            })
                            .await;
                        }
                    }
                    Event::Key(ev) => {
                        tx.send_update(ev).await;
                    }
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
        let container = gtk::Box::new(info.bar_position.orientation(), 5);

        let caps = IconLabel::new(&self.icons.caps_off, info.icon_theme, self.icon_size);
        let num = IconLabel::new(&self.icons.num_off, info.icon_theme, self.icon_size);
        let scroll = IconLabel::new(&self.icons.scroll_off, info.icon_theme, self.icon_size);

        if self.show_caps {
            caps.add_class("key");
            caps.add_class("caps");
            container.add(caps.deref());
        }

        if self.show_num {
            num.add_class("key");
            num.add_class("num");
            container.add(num.deref());
        }

        if self.show_scroll {
            scroll.add_class("key");
            scroll.add_class("scroll");
            container.add(scroll.deref());
        }

        let icons = self.icons;
        context.subscribe().recv_glib(move |ev| {
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
        });

        Ok(ModuleParts::new(container, None))
    }
}
