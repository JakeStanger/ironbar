use std::sync::Arc;

pub(crate) use self::config::BrightnessModule;
use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::Client;
use crate::clients::brightness::{self, Key, Scale};
use crate::config::{ProfileUpdateEvent, default};
use crate::image::IconLabel;
use crate::modules::brightness::config::{BrightnessProfile, default_profiles};
use crate::modules::brightness::utils::{
    MAX_SETTABLE_PERCENTAGE, MIN_PERCENTAGE, brightness_from, percent_from,
};
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn};
use color_eyre::Result;
use gtk::{Button, Label};
use gtk::{EventControllerScroll, EventControllerScrollFlags, prelude::*};
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

mod config;
mod utils;

#[derive(Clone, Debug, Default)]
pub struct BrightnessProperties {
    screen_brightness: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    AdjustBrightnessScroll(f64),
}

/// Get the initial state for the tracked device.
///
/// The device is identified with its [Key] matching the `key`.
/// The device must have already been registered to the [brightness::Client].
///
/// Returns the maximum brightness for the device and its reported [Scale].
/// If no state could be found for the `key` returns [None].
async fn initial_values(client: &Arc<brightness::Client>, key: &Key) -> Option<(i32, Scale)> {
    let state = match client.state().await {
        Ok(state) => state,
        Err(err) => {
            error!("could not read client's state: {err}");
            return None;
        }
    };

    let device_state = state
        .tracked
        .iter()
        .find(|&device_state| device_state.key() == key.get())?;

    Some(match device_state {
        brightness::BrightnessDeviceState::DBus { max_brightness, .. } => {
            (*max_brightness, Scale::Unknown)
        }
        brightness::BrightnessDeviceState::SysFs {
            max_brightness,
            scale,
            ..
        } => (*max_brightness, *scale),
    })
}

impl Module<Button> for BrightnessModule {
    type SendMessage = BrightnessProperties;
    type ReceiveMessage = UiEvent;

    module_impl!("brightness");

    fn on_create(&mut self) {
        self.profiles.setup_defaults(default_profiles());
    }

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
        let client = context.try_client::<brightness::Client>()?;
        let mut brightness_events = client.subscribe();
        let strategy = self.scale.clone();
        let scroll_speed = self.smooth_scroll_speed;

        let key = {
            let mut data_source = self.mode.clone();
            match data_source.resolve() {
                Ok(_) => (),
                Err(_) => {
                    return Err(color_eyre::Report::msg(
                        "no default could be resolved for brightness module",
                    ));
                }
            }
            data_source.key()
        };

        spawn(async move {
            let mut brightness = match &key {
                Key::Kbd { path } => match client.track_new_kbd(path).await {
                    Ok(brightness) => brightness,
                    Err(err) => {
                        error!("could not track {path} over D-Bus: {err}");
                        return;
                    }
                },
                Key::SysFs { subsystem, device } => {
                    match client.track_new_kernel_device(subsystem, device) {
                        Ok(brightness) => brightness,
                        Err(err) => {
                            error!("could not track device /sys/class/{subsystem}/{device}: {err}");
                            return;
                        }
                    }
                }
            };
            let Some((max_brightness, scale)) = initial_values(&client, &key).await else {
                error!("could not find initial values for device {}", key.get());
                return;
            };

            let mut percent = percent_from(brightness, max_brightness, &strategy, &scale);
            tx.send_update(BrightnessProperties {
                screen_brightness: percent,
            })
            .await;

            let mut partial_scroll: f32 = 0.0;

            loop {
                #[derive(Debug)]
                enum Event {
                    Ui(UiEvent),
                    Client(brightness::Event),
                }

                let event = tokio::select! {
                    Some(ui) = rx.recv() => Event::Ui(ui),
                    Ok(client) = brightness_events.recv() => Event::Client(client),
                };

                trace!("got event: {event:?}");

                let mut new_percent = percent;
                match event {
                    Event::Ui(ui_event) => {
                        let UiEvent::AdjustBrightnessScroll(dy) = ui_event;

                        debug!("processing scroll event (dy={dy})");

                        partial_scroll += dy as f32 * scroll_speed;

                        if partial_scroll.abs() >= 1.0 {
                            let offset = partial_scroll.floor();
                            partial_scroll -= offset;

                            new_percent = (percent + offset / 100f32)
                                .clamp(MIN_PERCENTAGE, MAX_SETTABLE_PERCENTAGE);
                            brightness =
                                brightness_from(new_percent, max_brightness, &strategy, &scale);

                            match client.set_brightness(&key, brightness).await {
                                Ok(_) => (),
                                Err(err) => {
                                    error!("could not set brightness: {err}");
                                    continue;
                                }
                            }
                        }
                    }
                    Event::Client(event) => {
                        if event.cmp_key(&key) {
                            brightness = event.brightness();
                            new_percent =
                                percent_from(brightness, max_brightness, &strategy, &scale);
                        }
                    }
                }

                if new_percent != percent {
                    percent = new_percent;
                    tx.send_update(BrightnessProperties {
                        screen_brightness: percent,
                    })
                    .await;
                }
            }
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> color_eyre::Result<ModuleParts<Button>>
    where
        <Self as Module<Button>>::SendMessage: Clone,
    {
        let button_label = Label::builder()
            .use_markup(true)
            .justify(self.layout.justify.into())
            .css_classes(["label"])
            .build();

        let button_icon = IconLabel::new(
            "",
            default::IconSize::Small as i32,
            &context.ironbar.image_provider(),
        );
        button_icon.add_css_class("icon");

        let container = gtk::Box::new(self.layout.orientation(info), 5);
        container.add_css_class("contents");
        container.append(&*button_icon);
        container.append(&button_label);

        let button = Button::new();
        button.set_child(Some(&container));

        let controller_tx = context.controller_tx.clone();

        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll_controller.connect_scroll(move |_, _, dy| {
            debug!("scrolling by {dy}");

            let ctx = controller_tx.clone();
            ctx.send_spawn(UiEvent::AdjustBrightnessScroll(dy));

            glib::Propagation::Proceed
        });
        button.add_controller(scroll_controller);

        let rx = context.subscribe();

        let mut manager = self.profiles.attach(
            &button,
            move |_, event: ProfileUpdateEvent<f64, BrightnessProfile, ()>| {
                let percentage = event.state.round();
                let format = event
                    .profile
                    .format
                    .replace("{percentage}", &percentage.to_string());

                button_label.set_label(&format);
                button_icon.set_label(event.profile.icon_label.as_deref());
            },
        );

        rx.recv_glib((), move |(), properties| {
            // UI displays values from 0 to 100.
            let percentage = properties.screen_brightness * 100f32;
            manager.update(percentage.into(), ());
        });

        Ok(ModuleParts::new(button, None))
    }
}
