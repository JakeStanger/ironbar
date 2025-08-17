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
        // Must be done here synchronously to avoid race condition
        let mut client_rx = client.subscribe();
        spawn(async move {
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

        // Must be done here synchronously to avoid race condition
        let rx = context.subscribe();
        // We cannot use recv_glib_async here because the lifetimes don't work out
        spawn_future_local(handle_update_events(
            rx,
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
        match event {
            Event::DeviceAdded { interface, .. } => {
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
        _ => None,
    }
}
