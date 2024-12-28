use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::config::CommonConfig;
use crate::dynamic_value::dynamic_string;
use crate::gtk_helpers::IronbarLabelExt;
use crate::module_impl;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use color_eyre::Result;
use gtk::Label;
use serde::Deserialize;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LabelModule {
    /// The text to show on the label.
    /// This is a [Dynamic String](dynamic-values#dynamic-string).
    ///
    /// **Required**
    label: String,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl LabelModule {
    pub(crate) fn new(label: String) -> Self {
        Self {
            label,
            common: Some(CommonConfig::default()),
        }
    }
}

impl Module<Label> for LabelModule {
    type SendMessage = String;
    type ReceiveMessage = ();

    module_impl!("label");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
        dynamic_string(&self.label, move |string| {
            tx.send_update_spawn(string);
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Result<ModuleParts<Label>> {
        let label = Label::builder().use_markup(true).build();

        {
            let label = label.clone();
            context
                .subscribe()
                .recv_glib(move |string| label.set_label_escaped(&string));
        }

        Ok(ModuleParts {
            widget: label,
            popup: None,
        })
    }
}
