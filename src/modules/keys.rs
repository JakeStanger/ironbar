use color_eyre::Result;
use gtk::prelude::*;
use serde::Deserialize;
use tokio::sync::mpsc;

use super::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::clients::libinput::{Event, Key, KeyEvent};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::new_icon_label;
use crate::{glib_recv, module_impl, module_update, send_async, spawn};

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
    caps: String,

    /// Icon to show when num lock is enabled.
    ///
    /// **Default**: ``
    #[serde(default = "default_icon_num")]
    num: String,

    /// Icon to show when scroll lock is enabled.
    ///
    /// **Default**: ``
    #[serde(default = "default_icon_scroll")]
    scroll: String,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            caps: default_icon_caps(),
            num: default_icon_num(),
            scroll: default_icon_scroll(),
        }
    }
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
                        let caps_state = client.get_state(Key::Caps);
                        module_update!(
                            tx,
                            KeyEvent {
                                key: Key::Caps,
                                state: caps_state
                            }
                        );

                        let num_state = client.get_state(Key::Num);
                        module_update!(
                            tx,
                            KeyEvent {
                                key: Key::Num,
                                state: num_state
                            }
                        );

                        let scroll_state = client.get_state(Key::Scroll);
                        module_update!(
                            tx,
                            KeyEvent {
                                key: Key::Scroll,
                                state: scroll_state
                            }
                        );
                    }
                    Event::Key(ev) => {
                        send_async!(tx, ModuleUpdateEvent::Update(ev));
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

        let caps = new_icon_label(&self.icons.caps, info.icon_theme, 32);
        let num = new_icon_label(&self.icons.num, info.icon_theme, 32);
        let scroll = new_icon_label(&self.icons.scroll, info.icon_theme, 32);

        if self.show_caps {
            caps.add_class("caps");
            container.add(&caps);
        }

        if self.show_num {
            caps.add_class("num");
            container.add(&num);
        }

        if self.show_scroll {
            caps.add_class("scroll");
            container.add(&scroll);
        }

        let handle_event = move |ev: KeyEvent| {
            let label = match ev.key {
                Key::Caps if self.show_caps => Some(&caps),
                Key::Num if self.show_num => Some(&num),
                Key::Scroll if self.show_scroll => Some(&scroll),
                _ => None,
            };

            if let Some(label) = label {
                label.set_visible(ev.state);
            }
        };

        glib_recv!(context.subscribe(), handle_event);

        Ok(ModuleParts::new(container, None))
    }
}
