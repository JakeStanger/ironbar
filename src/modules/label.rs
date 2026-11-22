use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::config::{CommonConfig, LayoutConfig, TruncateMode};
use crate::dynamic_value::dynamic_string;
use crate::gtk_helpers::IronbarLabelExt;
use crate::module_impl;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use color_eyre::Result;
use gtk::Label;
use serde::Deserialize;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize, Clone, PartialEq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
pub struct LabelModule {
    /// The text to show on the label.
    /// This is a [Dynamic String](dynamic-values#dynamic-string).
    label: String,

    // -- Common --

    /// The truncation options to use for the label text.
    truncate: Option<TruncateMode>,

    #[serde(default, flatten)]
    layout: LayoutConfig,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
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
        dynamic_string(&self.label, &context.tx, move |tx, string| {
            tx.send_update_spawn(string);
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Result<ModuleParts<Label>> {
        let label = Label::builder()
            .use_markup(true)
            .justify(self.layout.justify.into())
            .build();

        if let Some(truncate) = self.truncate {
            label.truncate(truncate);
        }

        context.subscribe().recv_glib(&label, move |label, string| {
            label.set_label_escaped(&string);
        });

        Ok(ModuleParts {
            widget: label,
            popup: None,
        })
    }
}
