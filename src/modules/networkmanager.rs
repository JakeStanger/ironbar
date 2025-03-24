use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::networkmanager::{Client, ClientState};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};
use color_eyre::Result;
use futures_lite::StreamExt;
use futures_signals::signal::SignalExt;
use gtk::prelude::ContainerExt;
use gtk::{Box as GtkBox, Image};
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NetworkManagerModule {
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

const fn default_icon_size() -> i32 {
    24
}

impl Module<GtkBox> for NetworkManagerModule {
    type SendMessage = ClientState;
    type ReceiveMessage = ();

    module_impl!("network_manager");

    fn spawn_controller(
        &self,
        _: &ModuleInfo,
        context: &WidgetContext<ClientState, ()>,
        _: Receiver<()>,
    ) -> Result<()> {
        let client = context.try_client::<Client>()?;
        let mut client_signal = client.subscribe().to_stream();
        let tx = context.tx.clone();

        spawn(async move {
            while let Some(state) = client_signal.next().await {
                tx.send_update(state).await;
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<ClientState, ()>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<GtkBox>> {
        const INITIAL_ICON_NAME: &str = "content-loading-symbolic";

        let container = GtkBox::new(info.bar_position.orientation(), 0);
        let icon = Image::new();
        icon.add_css_class("icon");
        container.add(&icon);

        let image_provider = context.ironbar.image_provider();

        glib::spawn_future_local({
            let image_provider = image_provider.clone();
            let icon = icon.clone();

            async move {
                image_provider
                    .load_into_image_silent(INITIAL_ICON_NAME, self.icon_size, false, &icon)
                    .await;
            }
        });

        context.subscribe().recv_glib_async((), move |(), state| {
            let image_provider = image_provider.clone();
            let icon = icon.clone();

            let icon_name = match state {
                ClientState::WiredConnected => "network-wired-symbolic",
                ClientState::WifiConnected => "network-wireless-symbolic",
                ClientState::CellularConnected => "network-cellular-symbolic",
                ClientState::VpnConnected => "network-vpn-symbolic",
                ClientState::WifiDisconnected => "network-wireless-acquiring-symbolic",
                ClientState::Offline => "network-wireless-disabled-symbolic",
                ClientState::Unknown => "dialog-question-symbolic",
            };

            async move {
                image_provider
                    .load_into_image_silent(icon_name, self.icon_size, false, &icon)
                    .await;
            }
        });

        Ok(ModuleParts::new(container, None))
    }
}
