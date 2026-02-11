mod config;

use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::networkmanager::{Client, NetworkManagerUpdate};
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::Provider;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};

use color_eyre::Result;
use gtk::prelude::WidgetExt;
use gtk::prelude::*;
use gtk::{Box as GtkBox, ContentFit, Picture};
use tokio::sync::mpsc::Receiver;

pub use config::NetworkManagerModule;

impl Module<GtkBox> for NetworkManagerModule {
    type SendMessage = NetworkManagerUpdate;
    type ReceiveMessage = ();

    module_impl!("network_manager");

    fn on_create(&mut self) {
        self.profiles.setup_defaults(config::default_profiles());
    }

    fn spawn_controller(
        &self,
        _: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, ()>,
        _: Receiver<()>,
    ) -> Result<()> {
        let client = context.try_client::<Client>()?;
        let tx = context.tx.clone();

        spawn(async move {
            let mut client_signal = client.subscribe().await;
            while let Ok(state) = client_signal.recv().await {
                tx.send_update(state).await;
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, ()>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<GtkBox>> {
        let container = GtkBox::new(info.bar_position.orientation(), 0);

        let image_provider = context.ironbar.image_provider();

        let icon_size = self.icon_size;
        let mut manager = self.profiles.attach(&container, move |_, event| {
            let (widget, image_provider): (gtk::Widget, Provider) = event.data;
            let icon_name = event.profile.icon.clone();
            tracing::debug!("profiles update: icon_name={icon_name}");
            if icon_name.is_empty() {
                widget.set_visible(false);
                return;
            }

            glib::spawn_future_local(async move {
                image_provider
                    .load_into_picture_silent(
                        &icon_name,
                        icon_size,
                        false,
                        widget.downcast_ref::<Picture>().expect("should be Picture"),
                    )
                    .await;
                widget.set_visible(true)
            });
        });

        let container_clone = container.clone();
        context.subscribe().recv_glib((), move |(), update| {
            match update {
                NetworkManagerUpdate::Devices(devices) => {
                    tracing::debug!("NetworkManager devices updated");
                    tracing::trace!("NetworkManager devices updated: {devices:#?}");

                    // resize the container's children to match the number of devices
                    if container.children().count() > devices.len() {
                        for child in container.children().skip(devices.len()) {
                            container.remove(&child);
                        }
                    } else {
                        while container.children().count() < devices.len() {
                            let icon = Picture::builder()
                                .content_fit(ContentFit::ScaleDown)
                                .css_classes(["icon"])
                                .build();
                            container.append(&icon);
                        }
                    }

                    // update each icon to match the device state
                    for (device, widget) in devices.iter().zip(container.children()) {
                        match self.get_profile_state(device) {
                            Some(state) => {
                                let tooltip = self.get_tooltip(device);
                                widget.set_tooltip_text(Some(&tooltip));
                                manager.update(state, (widget, image_provider.clone()));
                            }
                            _ => {
                                widget.set_visible(false);
                                continue;
                            }
                        };
                    }
                }
                NetworkManagerUpdate::Device(idx, device) => {
                    tracing::debug!("NetworkManager device {idx} updated: {}", device.interface);
                    tracing::trace!("NetworkManager device {idx} updated: {device:#?}");
                    if let Some(widget) = container.children().nth(idx) {
                        match self.get_profile_state(&device) {
                            Some(state) => {
                                let tooltip = self.get_tooltip(&device);
                                widget.set_tooltip_text(Some(&tooltip));
                                manager.update(state, (widget, image_provider.clone()));
                            }
                            _ => {
                                widget.set_visible(false);
                            }
                        };
                    } else {
                        tracing::warn!("No widget found for device index {idx}");
                    }
                }
            }
        });

        Ok(ModuleParts::new(container_clone, None))
    }
}
