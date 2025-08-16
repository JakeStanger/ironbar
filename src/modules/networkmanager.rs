use crate::clients::networkmanager::Client;
use crate::clients::networkmanager::dbus::{DeviceState, DeviceType};
use crate::clients::networkmanager::event::Event;
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::Provider;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};
use color_eyre::{Result, eyre::Ok};
use glib::spawn_future_local;
use gtk::prelude::{ContainerExt, WidgetExt};
use gtk::{Image, Orientation};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc};

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

impl Module<gtk::Box> for NetworkManagerModule {
    type SendMessage = Event;
    type ReceiveMessage = ();

    module_impl!("network_manager");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Event, ()>,
        _rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        let client = context.try_client::<Client>()?;
        // Should we be using context.tx with ModuleUpdateEvent::Update instead?
        let tx = context.update_tx.clone();
        spawn(async move {
            let mut client_rx = client.subscribe();
            while let Result::Ok(event) = client_rx.recv().await {
                tx.send(event)?;
            }

            Ok(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Event, ()>,
        _info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        let container = gtk::Box::new(Orientation::Horizontal, 0);

        // TODO: Check if passing the widget context in its entirety here is possible
        // We cannot use recv_glib_async() here because the lifetimes don't work out
        spawn_future_local(handle_update_events(
            context.subscribe(),
            container.clone(),
            self.icon_size,
            context.ironbar.image_provider(),
        ));

        Ok(ModuleParts::new(container, None))
    }
}

async fn handle_update_events(
    mut rx: broadcast::Receiver<Event>,
    container: gtk::Box,
    icon_size: i32,
    image_provider: Provider,
) {
    let mut icons = HashMap::<String, Image>::new();

    while let Result::Ok(event) = rx.recv().await {
        println!("NM UI event: {:?}", event);

        match event {
            Event::DeviceAdded { interface, r#type } => {
                if !is_supported_device_type(&r#type) {
                    continue;
                }
                let icon = Image::new();
                icon.add_class("icon");
                container.add(&icon);
                icons.insert(interface, icon);
            }
            Event::DeviceStateChanged {
                interface,
                r#type,
                state,
            } => {
                if !is_supported_device_type(&r#type) {
                    continue;
                }
                let icon = icons
                    .get(&interface)
                    .expect("the icon for the interface to be present");
                let icon_name = get_icon_for_device_state(&r#type, &state);
                match icon_name {
                    Some(icon_name) => {
                        image_provider
                            .load_into_image_silent(icon_name, icon_size, false, icon)
                            .await;
                        icon.show();
                    }
                    None => {
                        icon.hide();
                    }
                }
            }
        };
    }
}

fn is_supported_device_type(r#type: &DeviceType) -> bool {
    matches!(
        r#type,
        DeviceType::Ethernet | DeviceType::Wifi | DeviceType::Tun | DeviceType::Wireguard
    )
}

fn get_icon_for_device_state(r#type: &DeviceType, state: &DeviceState) -> Option<&'static str> {
    match r#type {
        DeviceType::Ethernet => match state {
            DeviceState::Unavailable => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::Disconnected => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::Prepare => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::Config => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::NeedAuth => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::IpConfig => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::IpCheck => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::Secondaries => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::Activated => Some("icon:network-wired-symbolic"),
            DeviceState::Deactivating => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::Failed => Some("icon:network-wired-disconnected-symbolic"),
            _ => None,
        },
        DeviceType::Wifi => match state {
            DeviceState::Unavailable => Some("icon:network-wireless-hardware-disabled-symbolic"),
            DeviceState::Disconnected => Some("icon:network-wireless-offline-symbolic"),
            DeviceState::Prepare => Some("icon:network-wireless-offline-symbolic"),
            DeviceState::Config => Some("icon:network-wireless-offline-symbolic"),
            DeviceState::NeedAuth => Some("icon:network-wireless-offline-symbolic"),
            DeviceState::IpConfig => Some("icon:network-wireless-offline-symbolic"),
            DeviceState::IpCheck => Some("icon:network-wireless-offline-symbolic"),
            DeviceState::Secondaries => Some("icon:network-wireless-offline-symbolic"),
            DeviceState::Activated => Some("icon:network-wireless-connected-symbolic"),
            DeviceState::Deactivating => Some("icon:network-wireless-offline-symbolic"),
            DeviceState::Failed => Some("icon:network-wireless-offline-symbolic"),
            _ => None,
        },
        DeviceType::Tun => match state {
            DeviceState::Activated => Some("icon:network-vpn-symbolic"),
            _ => None,
        },
        DeviceType::Wireguard => match state {
            DeviceState::Activated => Some("icon:network-vpn-symbolic"),
            _ => None,
        },
        _ => panic!("Device type should be a supported one"),
    }
}
