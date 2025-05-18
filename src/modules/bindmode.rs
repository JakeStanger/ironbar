use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::compositor::BindModeUpdate;
use crate::config::{CommonConfig, LayoutConfig, TruncateMode};
use crate::gtk_helpers::IronbarLabelExt;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{module_impl, spawn};
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

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

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

        let client = context.try_client::<dyn crate::clients::compositor::BindModeClient>()?;

        let tx = context.tx.clone();

        let mut rx = client.subscribe()?;
        spawn(async move {
            while let Ok(ev) = rx.recv().await {
                tx.send_update(ev).await;
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<Label>> {
        let label = Label::builder()
            .use_markup(true)
            .angle(self.layout.angle(info))
            .justify(self.layout.justify.into())
            .build();

        if let Some(truncate) = self.truncate {
            label.truncate(truncate);
        }

        // Send a dummy event on init so that the widget starts hidden
        {
            let tx = context.tx.clone();
            tx.send_spawn(ModuleUpdateEvent::Update(BindModeUpdate {
                name: String::new(),
                pango_markup: true,
            }));
        }

        {
            let label = label.clone();

            let on_mode = move |mode: BindModeUpdate| {
                trace!("mode: {:?}", mode);
                label.set_use_markup(mode.pango_markup);
                label.set_label_escaped(&mode.name);

                if mode.name.is_empty() {
                    label.hide();
                } else {
                    label.show();
                }
            };

            context.subscribe().recv_glib(on_mode);
        }

        Ok(ModuleParts {
            widget: label,
            popup: None,
        })
    }
}
