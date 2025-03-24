use crate::clients::wayland::{self, ToplevelEvent};
use crate::config::{CommonConfig, LayoutConfig, TruncateMode};
use crate::gtk_helpers::IronbarGtkExt;
use crate::gtk_helpers::IronbarLabelExt;
use crate::image::ImageProvider;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{glib_recv, module_impl, send_async, spawn, try_send};
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

                try_send!(
                    tx,
                    ModuleUpdateEvent::Update(Some((focused.title.clone(), focused.app_id)))
                );
            };

            while let Ok(event) = wlrx.recv().await {
                match event {
                    ToplevelEvent::Update(info) => {
                        if info.focused {
                            debug!("Changing focus");

                            current = Some(info.id);

                            send_async!(
                                tx,
                                ModuleUpdateEvent::Update(Some((
                                    info.title.clone(),
                                    info.app_id.clone()
                                )))
                            );
                        } else if info.id == current.unwrap_or_default() {
                            debug!("Clearing focus");
                            current = None;
                            send_async!(tx, ModuleUpdateEvent::Update(None));
                        }
                    }
                    ToplevelEvent::Remove(info) => {
                        if info.focused {
                            debug!("Clearing focus");
                            current = None;
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
        let container = gtk::Box::new(self.layout.orientation(info), 5);

        let icon = gtk::Image::new();
        if self.show_icon {
            icon.add_class("icon");
            container.append(&icon);
        }

        let label = Label::builder()
            // .angle(self.layout.angle(info))
            .justify(self.layout.justify.into())
            .build();

        label.add_class("label");

        if let Some(truncate) = self.truncate {
            label.truncate(truncate);
        }

        container.append(&label);

        {
            let icon_overrides = info.icon_overrides.clone();
            let icon_theme = info.icon_theme.clone();

            glib_recv!(context.subscribe(), data => {
                if let Some((name, mut id)) = data {
                    if self.show_icon {

                        if let Some(icon) = icon_overrides.get(&id) {
                            id = icon.clone();
                        }

                        match ImageProvider::parse(&id, &icon_theme, true, self.icon_size)
                            .map(|image| image.load_into_image(&icon))
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
