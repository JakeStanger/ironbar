use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::config::{CommonConfig, TruncateMode};
use crate::gtk_helpers::IronbarLabelExt;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{await_sync, module_impl};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;
use swayipc_async::ModeEvent;
use tokio::sync::mpsc;
use tracing::{info, trace};

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SwayModeModule {
    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub truncate: Option<TruncateMode>,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Module<Label> for SwayModeModule {
    type SendMessage = ModeEvent;
    type ReceiveMessage = ();

    module_impl!("sway_mode");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        info!("Sway Mode module started");
        let tx = context.tx.clone();

        await_sync(async move {
            let client = context.ironbar.clients.borrow_mut().sway()?;
            client
                .add_listener::<ModeEvent>(move |mode| {
                    trace!("mode: {:?}", mode);
                    tx.send_update_spawn(mode.clone());
                })
                .await?;

            Ok::<(), Report>(())
        })?;

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Result<ModuleParts<Label>> {
        let label = Label::new(None);
        label.set_use_markup(true);

        {
            let label = label.clone();

            if let Some(truncate) = self.truncate {
                label.truncate(truncate);
            }

            context.subscribe().recv_glib(move |mode| {
                trace!("mode: {:?}", mode);
                label.set_use_markup(mode.pango_markup);
                if mode.change == "default" {
                    label.set_label_escaped("");
                } else {
                    label.set_label_escaped(&mode.change);
                }
            });
        }

        Ok(ModuleParts {
            widget: label,
            popup: None,
        })
    }
}
