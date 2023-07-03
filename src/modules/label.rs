use crate::config::CommonConfig;
use crate::dynamic_value::dynamic_string;
use crate::modules::{Module, ModuleInfo, ModuleUpdateEvent, ModuleWidget, WidgetContext};
use crate::try_send;
use color_eyre::Result;
use glib::Continue;
use gtk::prelude::*;
use gtk::Label;
use serde::Deserialize;
use tokio::sync::mpsc;

#[derive(Debug, Deserialize, Clone)]
pub struct LabelModule {
    label: String,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Module<Label> for LabelModule {
    type SendMessage = String;
    type ReceiveMessage = ();

    fn name() -> &'static str {
        "label"
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        tx: mpsc::Sender<ModuleUpdateEvent<Self::SendMessage>>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        dynamic_string(&self.label, move |string| {
            try_send!(tx, ModuleUpdateEvent::Update(string));
            Continue(true)
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> Result<ModuleWidget<Label>> {
        let label = Label::new(None);
        label.set_use_markup(true);

        {
            let label = label.clone();
            context.widget_rx.attach(None, move |string| {
                label.set_markup(&string);
                Continue(true)
            });
        }

        Ok(ModuleWidget {
            widget: label,
            popup: None,
        })
    }
}
