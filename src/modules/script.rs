use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::config::{CommonConfig, LayoutConfig};
use crate::gtk_helpers::IronbarLabelExt;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::script::{OutputStream, Script, ScriptMode};
use crate::{module_impl, spawn};
use color_eyre::{Help, Report, Result};
use gtk::Label;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::error;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct ScriptModule {
    /// Path to script to execute.
    ///
    /// This can be an absolute path,
    /// or relative to the working directory.
    ///
    /// **Required**
    cmd: String,

    /// Script execution mode.
    /// See [modes](#modes) for more info.
    ///
    /// **Valid options**: `poll`, `watch`
    /// <br />
    /// **Default**: `poll`
    mode: ScriptMode,

    /// Time in milliseconds between executions.
    ///
    /// **Default**: `5000`
    interval: u64,

    // -- Common --
    /// See [layout options](module-level-options#layout)
    #[serde(flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for ScriptModule {
    fn default() -> Self {
        Self {
            cmd: String::new(),
            mode: ScriptMode::Poll,
            interval: 5000,
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

impl From<&ScriptModule> for Script {
    fn from(module: &ScriptModule) -> Self {
        Self {
            mode: module.mode,
            cmd: module.cmd.clone(),
            interval: module.interval,
        }
    }
}

impl Module<Label> for ScriptModule {
    type SendMessage = String;
    type ReceiveMessage = ();

    module_impl!("script");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let script: Script = self.into();

        let tx = context.tx.clone();
        spawn(async move {
            script.run(None, move |out, _| match out {
               OutputStream::Stdout(stdout) => {
                   tx.send_update_spawn(stdout);
               },
               OutputStream::Stderr(stderr) => {
                   error!("{:?}", Report::msg(stderr)
                                    .wrap_err("Watched script error:")
                                    .suggestion("Check the path to your script")
                                    .suggestion("Check the script for errors")
                                    .suggestion("If you expect the script to write to stderr, consider redirecting its output to /dev/null to suppress these messages"));
               }
           }).await;
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

        context
            .subscribe()
            .recv_glib(&label, |label, s| label.set_label_escaped(&s));

        Ok(ModuleParts {
            widget: label,
            popup: None,
        })
    }
}
