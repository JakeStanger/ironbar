use crate::clients::wayland::{self, ToplevelEvent};
use crate::config::{CommonConfig, TruncateMode};
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::ImageProvider;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{glib_recv, lock, send_async, spawn, try_send};
use color_eyre::Result;
use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::debug;

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

    truncate: Option<TruncateMode>,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for FocusedModule {
    fn default() -> Self {
        Self {
            show_icon: crate::config::default_true(),
            show_title: crate::config::default_true(),
            icon_size: default_icon_size(),
            truncate: None,
            common: Some(CommonConfig::default()),
        }
    }
}

const fn default_icon_size() -> i32 {
    32
}

impl Module<gtk::Box> for FocusedModule {
    type SendMessage = Option<(String, String)>;
    type ReceiveMessage = ();

    fn name() -> &'static str {
        "focused"
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: Sender<ModuleUpdateEvent<Self::SendMessage>>,
        _rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        spawn(async move {
            let (mut wlrx, handles) = {
                let wl = wayland::get_client();
                let wl = lock!(wl);
                wl.subscribe_toplevels()
            };

            let focused = handles.values().find_map(|handle| {
                handle
                    .info()
                    .and_then(|info| if info.focused { Some(info) } else { None })
            });

            if let Some(focused) = focused {
                try_send!(
                    tx,
                    ModuleUpdateEvent::Update(Some((focused.title.clone(), focused.app_id)))
                );
            };

            while let Ok(event) = wlrx.recv().await {
                match event {
                    ToplevelEvent::Update(handle) => {
                        let info = handle.info().unwrap_or_default();

                        if info.focused {
                            debug!("Changing focus");
                            send_async!(
                                tx,
                                ModuleUpdateEvent::Update(Some((
                                    info.title.clone(),
                                    info.app_id.clone()
                                )))
                            );
                        } else {
                            send_async!(tx, ModuleUpdateEvent::Update(None));
                        }
                    }
                    ToplevelEvent::Remove(handle) => {
                        let info = handle.info().unwrap_or_default();
                        if info.focused {
                            debug!("Clearing focus");
                            send_async!(tx, ModuleUpdateEvent::Update(None));
                        }
                    }
                    ToplevelEvent::New(_) => {}
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
        let icon_theme = info.icon_theme;

        let container = gtk::Box::new(info.bar_position.get_orientation(), 5);

        let icon = gtk::Image::new();
        if self.show_icon {
            icon.add_class("icon");
            container.add(&icon);
        }

        let label = Label::new(None);
        label.add_class("label");

        if let Some(truncate) = self.truncate {
            truncate.truncate_label(&label);
        }

        container.add(&label);

        {
            let icon_theme = icon_theme.clone();
            glib_recv!(context.subscribe(), data => {
                if let Some((name, id)) = data {
                    if self.show_icon {
                        match ImageProvider::parse(&id, &icon_theme, true, self.icon_size)
                            .map(|image| image.load_into_image(icon.clone()))
                        {
                            Some(Ok(())) => icon.show(),
                            _ => icon.hide(),
                        }
                    }

                    if self.show_title {
                        label.show();
                        label.set_label(&name);
                    }
                } else {
                    icon.hide();
                    label.hide();
                }
            });
        }

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}
