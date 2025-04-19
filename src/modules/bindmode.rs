use crate::clients::compositor::BindModeUpdate;
use crate::config::{CommonConfig, TruncateMode};
use crate::gtk_helpers::{IronbarGtkExt, IronbarLabelExt};
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{glib_recv, module_impl, module_update, send_async, spawn};
use color_eyre::Result;
use gtk::Label;
use gtk::prelude::*;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{info, trace};

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
    type SendMessage = BindModeUpdate;
    type ReceiveMessage = ();

    module_impl!("bindmode");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        info!("Bindmode module started");

        let client = context.ironbar.clients.borrow_mut().bindmode()?;

        let tx = context.tx.clone();

        let mut rx = client.subscribe()?;
        spawn(async move {
            while let Ok(ev) = rx.recv().await {
                module_update!(tx, ev);
            }
        });

        Ok(())
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

            let on_mode = move |mode: BindModeUpdate| {
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
