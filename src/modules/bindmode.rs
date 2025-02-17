use crate::config::{CommonConfig, TruncateMode};
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{await_sync, glib_recv, module_impl, try_send};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{info, trace};

#[derive(Clone, Debug)]
pub struct ModeEvent {
    /// The binding mode that became active.
    pub name: String,
    /// Whether the mode should be parsed as pango markup.
    pub pango_markup: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Bindmode {
    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub truncate: Option<TruncateMode>,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Module<Label> for Bindmode {
    type SendMessage = ModeEvent;
    type ReceiveMessage = ();

    module_impl!("bindmode");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        info!("Bindmode module started");

        #[cfg(feature = "sway")]
        {
            let tx = context.tx.clone();
            let result = await_sync(async move {
                let client = context.ironbar.clients.borrow_mut().sway()?;
                client
                    .add_listener::<swayipc_async::ModeEvent>(move |mode| {
                        trace!("mode: {:?}", mode);
                        try_send!(
                            tx,
                            ModuleUpdateEvent::Update(ModeEvent {
                                name: mode.change.clone(),
                                pango_markup: mode.pango_markup,
                            })
                        );
                    })
                    .await?;

                Ok::<(), Report>(())
            });
            if result.is_ok() {
                return Ok(());
            }
        }

        #[cfg(feature = "hyprland")]
        {
            let tx = context.tx.clone();
            let result = await_sync(async move {
                let client = context.ironbar.clients.borrow_mut().hyprland()?;
                client.listen_submap_events(move |submap| {
                    try_send!(
                        tx,
                        ModuleUpdateEvent::Update(ModeEvent {
                            name: submap.clone(),
                            pango_markup: false,
                        })
                    );
                });

                Ok::<(), Report>(())
            });
            if result.is_ok() {
                return Ok(());
            }
        }

        Err(Report::msg("No supported backend found"))
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Result<ModuleParts<Label>> {
        let label = Label::new(None);
        label.set_use_markup(true);

        label.add_class("sway_mode");

        {
            let label = label.clone();

            if let Some(truncate) = self.truncate {
                label.truncate(truncate);
            }

            let on_mode = move |mode: ModeEvent| {
                trace!("mode: {:?}", mode);
                label.set_use_markup(mode.pango_markup);
                if mode.name == "default" {
                    label.set_label_escaped("");
                } else {
                    label.set_label_escaped(&mode.name);
                }
            };

            glib_recv!(context.subscribe(), mode => on_mode(mode));
        }

        Ok(ModuleParts {
            widget: label,
            popup: None,
        })
    }
}
