use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::wayland::{self, ToplevelEvent};
use crate::config::{CommonConfig, LayoutConfig, TruncateMode};
use crate::gtk_helpers::IronbarGtkExt;
use crate::gtk_helpers::IronbarLabelExt;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};
use color_eyre::Result;
use gtk::Label;
use gtk::prelude::*;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::debug;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FocusedModule {
    /// Whether to show icon on the bar.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    show_icon: bool,
    /// Whether to show app name on the bar.
    ///
    /// **Default**: `true`
    #[serde(default = "crate::config::default_true")]
    show_title: bool,

    /// Icon size in pixels.
    ///
    /// **Default**: `32`
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    // -- common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    truncate: Option<TruncateMode>,

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
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
            layout: LayoutConfig::default(),
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

    module_impl!("focused");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
        let wl = context.client::<wayland::Client>();

        spawn(async move {
            let mut current = None;

            let mut wlrx = wl.subscribe_toplevels();
            let handles = wl.toplevel_info_all();

            let focused = handles.into_iter().find(|info| info.focused);

            if let Some(focused) = focused {
                current = Some(focused.id);

                tx.send_update(Some((focused.title.clone(), focused.app_id)))
                    .await;
            }

            while let Ok(event) = wlrx.recv().await {
                match event {
                    ToplevelEvent::Update(info) => {
                        if info.focused {
                            debug!("Changing focus");

                            current = Some(info.id);

                            tx.send_update(Some((info.title.clone(), info.app_id)))
                                .await;
                        } else if info.id == current.unwrap_or_default() {
                            debug!("Clearing focus");
                            current = None;
                            tx.send_update(None).await;
                        }
                    }
                    ToplevelEvent::Remove(info) => {
                        if info.focused {
                            debug!("Clearing focus");
                            current = None;
                            tx.send_update(None).await;
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
        let container = gtk::Box::new(self.layout.orientation(info), 5);

        let icon = gtk::Image::new();
        if self.show_icon {
            icon.add_class("icon");
            container.add(&icon);
        }

        let label = Label::builder()
            .angle(self.layout.angle(info))
            .justify(self.layout.justify.into())
            .build();

        label.add_class("label");

        if let Some(truncate) = self.truncate {
            label.truncate(truncate);
        }

        container.add(&label);

        {
            let image_provider = context.ironbar.image_provider();

            context.subscribe().recv_glib_async((), move |(), data| {
                let icon = icon.clone();
                let label = label.clone();
                let image_provider = image_provider.clone();

                async move {
                    if let Some((name, id)) = data {
                        if self.show_icon {
                            match image_provider
                                .load_into_image(&id, self.icon_size, true, &icon)
                                .await
                            {
                                Ok(true) => icon.show(),
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
                }
            });
        }

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}
