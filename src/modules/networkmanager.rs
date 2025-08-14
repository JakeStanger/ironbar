use color_eyre::Result;
use futures_lite::StreamExt;
use gtk::prelude::{ContainerExt, WidgetExt};
use gtk::{Box as GtkBox, Image, Orientation};
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::networkmanager::Client;
use crate::clients::networkmanager::event::Event;
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
    type SendMessage = Event;
    type ReceiveMessage = ();

    module_impl!("network_manager");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Event, ()>,
        _rx: Receiver<()>,
    ) -> Result<()> {
        let client = context.try_client::<Client>()?;
        // let mut client_signal = client.subscribe().to_stream();
        // let widget_transmitter = context.tx.clone();

        // spawn(async move {
        // while let Some(state) = client_signal.next().await {
        //     widget_transmitter
        //         .send_expect(ModuleUpdateEvent::Update(state))
        //         .await;
        // }
        // });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Event, ()>,
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

        context
            .subscribe()
            .recv_glib_async((), move |(), event| async {});

        Ok(ModuleParts::new(container, None))
    }
}
