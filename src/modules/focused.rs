use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::sway::node::{get_node_id, get_open_windows};
use crate::sway::{get_client, get_sub_client};
use crate::{await_sync, icon};
use color_eyre::Result;
use glib::Continue;
use gtk::prelude::*;
use gtk::{IconTheme, Image, Label};
use serde::Deserialize;
use swayipc_async::WindowChange;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::trace;

#[derive(Debug, Deserialize, Clone)]
pub struct FocusedModule {
    /// Whether to show icon on the bar.
    #[serde(default = "crate::config::default_true")]
    show_icon: bool,
    /// Whether to show app name on the bar.
    #[serde(default = "crate::config::default_true")]
    show_title: bool,

    /// Icon size in pixels.
    #[serde(default = "default_icon_size")]
    icon_size: i32,
    /// GTK icon theme to use.
    icon_theme: Option<String>,
}

const fn default_icon_size() -> i32 {
    32
}

impl Module<gtk::Box> for FocusedModule {
    type SendMessage = (String, String);
    type ReceiveMessage = ();

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        _rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let focused = await_sync(async {
            let sway = get_client().await;
            let mut sway = sway.lock().await;
            get_open_windows(&mut sway)
                .await
                .expect("Failed to get open windows")
                .into_iter()
                .find(|node| node.focused)
        });

        if let Some(node) = focused {
            let id = get_node_id(&node);
            let name = node.name.as_deref().unwrap_or(id);

            tx.try_send(ModuleUpdateEvent::Update((
                name.to_string(),
                id.to_string(),
            )))?;
        }

        spawn(async move {
            let mut srx = {
                let sway = get_sub_client();
                sway.subscribe_window()
            };

            trace!("Set up Sway window subscription");

            while let Ok(payload) = srx.recv().await {
                let update = match payload.change {
                    WindowChange::Focus => true,
                    WindowChange::Title => payload.container.focused,
                    _ => false,
                };

                if update {
                    let node = payload.container;

                    let id = get_node_id(&node);
                    let name = node.name.as_deref().unwrap_or(id);

                    tx.try_send(ModuleUpdateEvent::Update((
                        name.to_string(),
                        id.to_string(),
                    )))
                    .expect("Failed to send focus update");
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleWidget<gtk::Box>> {
        let icon_theme = IconTheme::new();

        if let Some(theme) = self.icon_theme {
            icon_theme.set_custom_theme(Some(&theme));
        }

        let container = gtk::Box::new(info.bar_position.get_orientation(), 5);

        let icon = Image::builder().name("icon").build();
        let label = Label::builder().name("label").build();

        container.add(&icon);
        container.add(&label);

        {
            context.widget_rx.attach(None, move |(name, id)| {
                let pixbuf = icon::get_icon(&icon_theme, &id, self.icon_size);

                if self.show_icon {
                    icon.set_pixbuf(pixbuf.as_ref());
                }

                if self.show_title {
                    label.set_label(&name);
                }

                Continue(true)
            });
        }

        Ok(ModuleWidget {
            widget: container,
            popup: None,
        })
    }
}
