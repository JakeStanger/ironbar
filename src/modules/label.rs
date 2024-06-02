use crate::config::CommonConfig;
use crate::dynamic_value::dynamic_string;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{glib_recv, module_impl, try_send};
use color_eyre::Result;
use gtk::prelude::*;
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
            try_send!(tx, ModuleUpdateEvent::Update(string));
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

        {
            let label = label.clone();
            glib_recv!(context.subscribe(), string => label.set_markup(&string));
        }

        Ok(ModuleParts {
            widget: label,
            popup: None,
        })
    }
}
