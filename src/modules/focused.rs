use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::wayland::{ToplevelChange};
use crate::{await_sync, icon, wayland};
use color_eyre::Result;
use glib::Continue;
use gtk::prelude::*;
use gtk::{IconTheme, Image, Label, Orientation};
use serde::Deserialize;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender};

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
            let wl = wayland::get_client().await;
            let toplevels = wl
                .toplevels
                .read()
                .expect("Failed to get read lock on toplevels")
                .clone();

            toplevels.into_iter().find(|top| top.active)
        });

        if let Some(top) = focused {
            tx.try_send(ModuleUpdateEvent::Update((
                top.title.clone(),
                top.app_id
            )))?;
        }

        spawn(async move {
            let mut wlrx = {
                let wl = wayland::get_client().await;
                wl.subscribe_toplevels()
            };

            while let Ok(event) = wlrx.recv().await {
                let update = match event.change {
                    ToplevelChange::Focus(focus) => focus,
                    ToplevelChange::Title(_) => event.toplevel.active,
                    _ => false
                };

                if update {
                    tx.send(ModuleUpdateEvent::Update((
                        event.toplevel.title,
                        event.toplevel.app_id,
                    )))
                    .await
                    .expect("Failed to send focus update");
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Result<ModuleWidget<gtk::Box>> {
        let icon_theme = IconTheme::new();

        if let Some(theme) = self.icon_theme {
            icon_theme.set_custom_theme(Some(&theme));
        }

        let container = gtk::Box::new(Orientation::Horizontal, 5);

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
