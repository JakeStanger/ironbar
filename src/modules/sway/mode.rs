use crate::config::{CommonConfig, TruncateMode};
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{await_sync, glib_recv, module_impl, try_send};
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
                .add_listener::<swayipc_async::ModeEvent>(move |mode| {
                    trace!("mode: {:?}", mode);
                    try_send!(tx, ModuleUpdateEvent::Update(mode.clone()));
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

        {
            let label = label.clone();

            if let Some(truncate) = self.truncate {
                truncate.truncate_label(&label);
            }

            let on_mode = move |mode: ModeEvent| {
                trace!("mode: {:?}", mode);
                label.set_use_markup(mode.pango_markup);
                if mode.change != "default" {
                    label.set_markup(&mode.change)
                } else {
                    label.set_markup("");
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
