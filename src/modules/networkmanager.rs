use color_eyre::Result;
use futures_lite::StreamExt;
use futures_signals::signal::SignalExt;
use gtk::prelude::{ContainerExt, WidgetExt};
use gtk::{Box as GtkBox, Image, Orientation};
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::networkmanager::Client;
use crate::clients::networkmanager::state::{
    CellularState, State, VpnState, WifiState, WiredState,
};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{module_impl, spawn};

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
    type SendMessage = State;
    type ReceiveMessage = ();

    module_impl!("network_manager");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<State, ()>,
        _rx: Receiver<()>,
    ) -> Result<()> {
        let client = context.try_client::<Client>()?;
        let mut client_signal = client.subscribe().to_stream();
        let widget_transmitter = context.tx.clone();

        spawn(async move {
            while let Some(state) = client_signal.next().await {
                widget_transmitter
                    .send_expect(ModuleUpdateEvent::Update(state))
                    .await;
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<State, ()>,
        _info: &ModuleInfo,
    ) -> Result<ModuleParts<GtkBox>> {
        let container = GtkBox::new(Orientation::Horizontal, 0);

        // Wired icon
        let wired_icon = Image::new();
        wired_icon.add_class("icon");
        wired_icon.add_class("wired-icon");
        container.add(&wired_icon);

        // Wifi icon
        let wifi_icon = Image::new();
        wifi_icon.add_class("icon");
        wifi_icon.add_class("wifi-icon");
        container.add(&wifi_icon);

        // Cellular icon
        let cellular_icon = Image::new();
        cellular_icon.add_class("icon");
        cellular_icon.add_class("cellular-icon");
        container.add(&cellular_icon);

        // VPN icon
        let vpn_icon = Image::new();
        vpn_icon.add_class("icon");
        vpn_icon.add_class("vpn-icon");
        container.add(&vpn_icon);

        context.subscribe().recv_glib_async((), move |(), state| {
            // TODO: Make this whole section less boneheaded

            let wired_icon_name = match state.wired {
                WiredState::Connected => "icon:network-wired-symbolic",
                WiredState::Disconnected => "icon:network-wired-disconnected-symbolic",
                WiredState::NotPresent | WiredState::Unknown => "",
            };
            let wifi_icon_name = match state.wifi {
                WifiState::Connected(_) => "icon:network-wireless-connected-symbolic",
                WifiState::Disconnected => "icon:network-wireless-offline-symbolic",
                WifiState::Disabled => "icon:network-wireless-hardware-disabled-symbolic",
                WifiState::NotPresent | WifiState::Unknown => "",
            };
            let cellular_icon_name = match state.cellular {
                CellularState::Connected => "icon:network-cellular-connected-symbolic",
                CellularState::Disconnected => "icon:network-cellular-offline-symbolic",
                CellularState::Disabled => "icon:network-cellular-hardware-disabled-symbolic",
                CellularState::NotPresent | CellularState::Unknown => "",
            };
            let vpn_icon_name = match state.vpn {
                VpnState::Connected(_) => "icon:network-vpn-symbolic",
                VpnState::Disconnected | VpnState::Unknown => "",
            };

            let wired_icon = wired_icon.clone();
            let wifi_icon = wifi_icon.clone();
            let cellular_icon = cellular_icon.clone();
            let vpn_icon = vpn_icon.clone();

            let image_provider = context.ironbar.image_provider();

            async move {
                if wired_icon_name.is_empty() {
                    wired_icon.hide();
                } else {
                    image_provider
                        .load_into_image_silent(wired_icon_name, self.icon_size, false, &wired_icon)
                        .await;
                    wired_icon.show();
                }

                if wifi_icon_name.is_empty() {
                    wifi_icon.hide();
                } else {
                    image_provider
                        .load_into_image_silent(wifi_icon_name, self.icon_size, false, &wifi_icon)
                        .await;
                    wifi_icon.show();
                }

                if cellular_icon_name.is_empty() {
                    cellular_icon.hide();
                } else {
                    image_provider
                        .load_into_image_silent(
                            cellular_icon_name,
                            self.icon_size,
                            false,
                            &cellular_icon,
                        )
                        .await;
                    cellular_icon.show();
                }

                if vpn_icon_name.is_empty() {
                    vpn_icon.hide();
                } else {
                    image_provider
                        .load_into_image_silent(vpn_icon_name, self.icon_size, false, &vpn_icon)
                        .await;
                    vpn_icon.show();
                }
            }
        });

        Ok(ModuleParts::new(container, None))
    }
}
