use crate::config::{CommonConfig, LayoutConfig, TruncateMode};
use crate::dynamic_value::dynamic_string;
use crate::gtk_helpers::IronbarLabelExt;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{glib_recv, module_impl, try_send};
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

    // -- Common --
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

impl LabelModule {
    pub(crate) fn new(label: String) -> Self {
        Self {
            label,
            truncate: None,
            layout: LayoutConfig::default(),
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
        info: &ModuleInfo,
    ) -> Result<ModuleParts<Label>> {
        let label = Label::builder()
            .use_markup(true)
            // .angle(self.layout.angle(info))
            .justify(self.layout.justify.into())
            .build();

        if let Some(truncate) = self.truncate {
            label.truncate(truncate);
        }

        {
            let label = label.clone();
            glib_recv!(context.subscribe(), string => label.set_label_escaped(&string));
        }

        Ok(ModuleParts {
            widget: label,
            popup: None,
        })
    }
}
