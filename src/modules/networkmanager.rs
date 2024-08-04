use color_eyre::Result;
use futures_lite::StreamExt;
use futures_signals::signal::SignalExt;
use gtk::prelude::{ContainerExt, WidgetExt};
use gtk::{Box as GtkBox, Image, Orientation};
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;

use crate::clients::networkmanager::state::{
    CellularState, State, VpnState, WifiState, WiredState,
};
use crate::clients::networkmanager::Client;
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
    type SendMessage = State;
    type ReceiveMessage = ();

    module_impl!("network_manager");

    fn spawn_controller(
        &self,
        _: &ModuleInfo,
        context: &WidgetContext<State, ()>,
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
        context: WidgetContext<State, ()>,
        info: &ModuleInfo,
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

        let icon_theme = info.icon_theme.clone();
        glib_recv!(context.subscribe(), state => {
            macro_rules! update_icon {
                (
                    $icon_var:expr,
                    $state_type:ident,
                    {$($state:pat => $icon_name:expr,)+}
                ) => {
                    let icon_name = match state.$state_type {
                        $($state => $icon_name,)+
                    };
                    if icon_name.is_empty() {
                        $icon_var.hide();
                    } else {
                        ImageProvider::parse(icon_name, &icon_theme, false, self.icon_size)
                            .map(|provider| provider.load_into_image($icon_var.clone()));
                        $icon_var.show();
                    }
                };
            }

            update_icon!(wired_icon, wired, {
                WiredState::Connected => "icon:network-wired-symbolic",
                WiredState::Disconnected => "icon:network-wired-disconnected-symbolic",
                WiredState::NotPresent | WiredState::Unknown => "",
            });
            update_icon!(wifi_icon, wifi, {
                WifiState::Connected(_) => "icon:network-wireless-connected-symbolic",
                WifiState::Disconnected => "icon:network-wireless-offline-symbolic",
                WifiState::Disabled => "icon:network-wireless-hardware-disabled-symbolic",
                WifiState::NotPresent | WifiState::Unknown => "",
            });
            update_icon!(cellular_icon, cellular, {
                CellularState::Connected => "icon:network-cellular-connected-symbolic",
                CellularState::Disconnected => "icon:network-cellular-offline-symbolic",
                CellularState::Disabled => "icon:network-cellular-hardware-disabled-symbolic",
                CellularState::NotPresent | CellularState::Unknown => "",
            });
            update_icon!(vpn_icon, vpn, {
                VpnState::Connected(_) => "icon:network-vpn-symbolic",
                VpnState::Disconnected | VpnState::Unknown => "",
            });
        });

        Ok(ModuleParts::new(container, None))
    }
}
