use color_eyre::Result;
use color_eyre::eyre::Report;
use gtk::prelude::*;
use indexmap::IndexMap;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

use super::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::compositor::{self, KeyboardLayoutUpdate};
use crate::clients::libinput::{Event, Key, KeyEvent};
use crate::config::{CommonConfig, LayoutConfig};
use crate::image::{IconButton, IconLabel};
use crate::{module_impl, spawn};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct KeyboardModule {
    /// Whether to show capslock indicator.
    ///
    /// **Default**: `true`
    show_caps: bool,

    /// Whether to show num lock indicator.
    ///
    ///  **Default**: `true`
    show_num: bool,

    /// Whether to show scroll lock indicator.
    ///
    ///  **Default**: `true`
    show_scroll: bool,

    /// Whether to show the current keyboard layout.
    ///
    ///  **Default**: `true`
    show_layout: bool,

    /// Size to render the icons at, in pixels (image icons only).
    ///
    /// **Default** `32`
    icon_size: i32,

    /// Player state icons.
    ///
    /// See [icons](#icons).
    icons: Icons,

    /// The Wayland seat to attach to.
    /// You almost certainly do not need to change this.
    ///
    /// **Default**: `seat0`
    seat: String,

    // -- common --
    /// See [layout options](module-level-options#layout)
    #[serde(flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for KeyboardModule {
    fn default() -> Self {
        Self {
            show_caps: true,
            show_num: true,
            show_scroll: true,
            show_layout: true,
            icon_size: 32,
            icons: Icons::default(),
            seat: "seat0".to_string(),
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
struct Icons {
    /// Icon to show when capslock is enabled.
    ///
    /// **Default**: `󰪛`
    caps_on: String,

    /// Icon to show when capslock is disabled.
    ///
    /// **Default**: `""`
    caps_off: String,

    /// Icon to show when num lock is enabled.
    ///
    /// **Default**: ``
    num_on: String,

    /// Icon to show when num lock is disabled.
    ///
    /// **Default**: `""`
    num_off: String,

    /// Icon to show when scroll lock is enabled.
    ///
    /// **Default**: ``
    scroll_on: String,

    /// Icon to show when scroll lock is disabled.
    ///
    /// **Default**: `""`
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
    layout_map: IndexMap<String, String>,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            caps_on: "󰪛".to_string(),
            caps_off: String::new(),
            num_on: "".to_string(),
            num_off: String::new(),
            scroll_on: "".to_string(),
            scroll_off: String::new(),
            layout_map: IndexMap::new(),
        }
    }
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
        let client = context.ironbar.clients.borrow_mut().libinput(&self.seat);

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
                            tx.send_update(KeyboardUpdate::Key(event)).await;
                        }
                    }
                    Event::Key(ev) => {
                        tx.send_update(KeyboardUpdate::Key(ev)).await;
                    }
                }
            }
        });

        match context.try_client::<dyn compositor::KeyboardLayoutClient>() {
            Ok(client) => {
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
                                    tx.send_update(KeyboardUpdate::Layout(payload)).await;
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                                    tracing::warn!(
                                        "Channel lagged behind by {count}, this may result in unexpected or broken behaviour"
                                    );
                                }
                                Err(err) => {
                                    error!("{err:?}");
                                    break;
                                }
                            }
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
            }
            Err(err) => error!("Failed to spawn keyboard layout client: {err:?}"),
        }

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        let container = gtk::Box::new(self.layout.orientation(info), 0);

        let image_provider = context.ironbar.image_provider();

        let caps = IconLabel::new(&self.icons.caps_off, self.icon_size, &image_provider);
        let num = IconLabel::new(&self.icons.num_off, self.icon_size, &image_provider);
        let scroll = IconLabel::new(&self.icons.scroll_off, self.icon_size, &image_provider);

        caps.label().set_justify(self.layout.justify.into());
        num.label().set_justify(self.layout.justify.into());
        scroll.label().set_justify(self.layout.justify.into());

        let layout_button = IconButton::new("", self.icon_size, image_provider);

        if self.show_caps {
            caps.add_css_class("key");
            caps.add_css_class("caps");
            container.append(&*caps);
        }

        if self.show_num {
            num.add_css_class("key");
            num.add_css_class("num");
            container.append(&*num);
        }

        if self.show_scroll {
            scroll.add_css_class("key");
            scroll.add_css_class("scroll");
            container.append(&*scroll);
        }

        if self.show_layout {
            layout_button.add_css_class("layout");
            container.append(&*layout_button);
        }

        {
            let tx = context.controller_tx.clone();
            layout_button.connect_clicked(move |_| {
                tx.send_spawn(());
            });
        }

        let icons = self.icons;
        context
            .subscribe()
            .recv_glib((), move |(), ev: KeyboardUpdate| match ev {
                KeyboardUpdate::Key(ev) => {
                    let parts = match (ev.key, ev.state) {
                        (Key::Caps, true) if self.show_caps => {
                            Some((&caps, icons.caps_on.as_str()))
                        }
                        (Key::Caps, false) if self.show_caps => {
                            Some((&caps, icons.caps_off.as_str()))
                        }
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
                            label.add_css_class("enabled");
                        } else {
                            label.remove_css_class("enabled");
                        }
                    }
                }
                KeyboardUpdate::Layout(KeyboardLayoutUpdate(language)) => {
                    let text = icons
                        .layout_map
                        .iter()
                        .find_map(|(pattern, display_text)| {
                            let is_match = if pattern.ends_with('*') {
                                language.starts_with(&pattern[..pattern.len() - 1])
                            } else {
                                pattern == &language
                            };

                            is_match.then_some(display_text)
                        })
                        .unwrap_or(&language);
                    layout_button.set_label(text);
                }
            });
        Ok(ModuleParts::new(container, None))
    }
}
