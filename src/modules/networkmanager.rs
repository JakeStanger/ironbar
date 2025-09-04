use crate::clients::networkmanager::Client;
use crate::clients::networkmanager::dbus::{DeviceState, DeviceType};
use crate::clients::networkmanager::event::{ClientToModuleEvent, ModuleToClientEvent};
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
use tracing::debug;

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
    type SendMessage = ClientToModuleEvent;
    type ReceiveMessage = ();

    module_impl!("network_manager");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<ClientToModuleEvent, ()>,
        _widget_receiver: mpsc::Receiver<()>,
    ) -> Result<()> {
        let client = context.try_client::<Client>()?;
        // Should we be using context.tx with ModuleUpdateEvent::Update instead?
        let widget_sender = context.update_tx.clone();

        // Must be done here otherwise we miss the response to our `NewController` event
        let mut client_receiver = client.subscribe();

        client
            .get_sender()
            .send(ModuleToClientEvent::NewController)?;

        spawn(async move {
            while let Result::Ok(event) = client_receiver.recv().await {
                widget_sender.send(event)?;
            }
            Ok(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<ClientToModuleEvent, ()>,
        _info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        // Must be done here otherwise we miss the response to our `NewController` event
        let receiver = context.subscribe();

        let container = gtk::Box::new(Orientation::Horizontal, 0);

        // We cannot use recv_glib_async here because the lifetimes don't work out
        spawn_future_local(handle_update_events(
            receiver,
            container.clone(),
            self.icon_size,
            context.ironbar.image_provider(),
        ));

        Ok(ModuleParts::new(container, None))
    }
}

async fn handle_update_events(
    mut widget_receiver: broadcast::Receiver<ClientToModuleEvent>,
    container: gtk::Box,
    icon_size: i32,
    image_provider: Provider,
) -> Result<()> {
    // TODO: Ensure the visible icons are always in the same order
    let mut icons = HashMap::<u32, Image>::new();

    while let Result::Ok(event) = widget_receiver.recv().await {
        match event {
            ClientToModuleEvent::DeviceChanged {
                number,
                r#type,
                new_state,
            } => {
                debug!(
                    "Module widget received DeviceChanged event for number {}",
                    number
                );

                let icon: &_ = icons.entry(number).or_insert_with(|| {
                    debug!("Adding icon for device {}", number);

                    let icon = Image::new();
                    icon.add_class("icon");
                    container.add(&icon);
                    icon
                });

                // TODO: Make this configurable at runtime
                let icon_name = get_icon_for_device_state(&r#type, &new_state);
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
            ClientToModuleEvent::DeviceRemoved { number } => {
                debug!(
                    "Module widget received DeviceRemoved event for number {}",
                    number
                );

                let icon = icons
                    .get(&number)
                    .expect("The icon for {} was about to be removed but was not present");
                container.remove(icon);
                icons.remove(&number);
            }
        }
    }

    Ok(())
}

fn get_icon_for_device_state(r#type: &DeviceType, state: &DeviceState) -> Option<&'static str> {
    match r#type {
        DeviceType::Ethernet => match state {
            DeviceState::Unavailable
            | DeviceState::Disconnected
            | DeviceState::Prepare
            | DeviceState::Config
            | DeviceState::NeedAuth
            | DeviceState::IpConfig
            | DeviceState::IpCheck
            | DeviceState::Secondaries
            | DeviceState::Deactivating
            | DeviceState::Failed => Some("icon:network-wired-disconnected-symbolic"),
            DeviceState::Activated => Some("icon:network-wired-symbolic"),
            _ => None,
        },
        DeviceType::Wifi => match state {
            DeviceState::Unavailable => Some("icon:network-wireless-hardware-disabled-symbolic"),
            DeviceState::Disconnected
            | DeviceState::Prepare
            | DeviceState::Config
            | DeviceState::NeedAuth
            | DeviceState::IpConfig
            | DeviceState::IpCheck
            | DeviceState::Secondaries
            | DeviceState::Deactivating
            | DeviceState::Failed => Some("icon:network-wireless-offline-symbolic"),
            DeviceState::Activated => Some("icon:network-wireless-connected-symbolic"),
            _ => None,
        },
        DeviceType::Tun | DeviceType::Wireguard => match state {
            DeviceState::Activated => Some("icon:network-vpn-symbolic"),
            _ => None,
        },
        _ => None,
    }
}
