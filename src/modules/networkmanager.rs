use color_eyre::Result;
use futures_lite::StreamExt;
use futures_signals::signal::SignalExt;
use gtk::prelude::ContainerExt;
use gtk::{Box as GtkBox, Image, Orientation};
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;

use crate::clients::networkmanager::{Client, ClientState};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::ImageProvider;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{glib_recv, module_impl, send_async, spawn};

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
        let widget_transmitter = context.tx.clone();

        spawn(async move {
            while let Some(state) = client_signal.next().await {
                send_async!(widget_transmitter, ModuleUpdateEvent::Update(state));
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<ClientState, ()>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<GtkBox>> {
        let container = GtkBox::new(Orientation::Horizontal, 0);
        let icon = Image::new();
        icon.add_class("icon");
        container.add(&icon);

        let icon_theme = info.icon_theme.clone();

        let initial_icon_name = "content-loading-symbolic";
        ImageProvider::parse(initial_icon_name, &icon_theme, false, self.icon_size)
            .map(|provider| provider.load_into_image(icon.clone()));

        let widget_receiver = context.subscribe();
        glib_recv!(widget_receiver, state => {
            let icon_name = match state {
                ClientState::WiredConnected => "network-wired-symbolic",
                ClientState::WifiConnected => "network-wireless-symbolic",
                ClientState::CellularConnected => "network-cellular-symbolic",
                ClientState::VpnConnected => "network-vpn-symbolic",
                ClientState::WifiDisconnected => "network-wireless-acquiring-symbolic",
                ClientState::Offline => "network-wireless-disabled-symbolic",
                ClientState::Unknown => "dialog-question-symbolic",
            };
            ImageProvider::parse(icon_name, &icon_theme, false, self.icon_size)
                .map(|provider| provider.load_into_image(icon.clone()));
        });

        Ok(ModuleParts::new(container, None))
    }
}
