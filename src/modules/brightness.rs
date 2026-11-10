use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::Client;
use crate::clients::brightness::{self, Key, Scale, default_resource_name};
use crate::config::{
    CommonConfig, LayoutConfig, ProfileUpdateEvent, Profiles, TruncateMode, default,
};
use crate::image::IconLabel;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::profiles;
use crate::{module_impl, spawn};
use color_eyre::Result;
use gtk::{Button, Label};
use gtk::{EventControllerScroll, EventControllerScrollFlags, prelude::*};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{debug, error, trace};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrightnessDataSource {
    /// using the keyboard D-Bus. Use `busctl` to find the keyboard you want under interface `org.freedesktop.UPower` then check for the path `/org/freedesktop/UPower/KbdBacklight/<device>`.
    ///
    /// Note: this only works for keyboards, not for screen brightness.
    ///
    /// **Default**: `None`
    Keyboard { path: Option<Box<str>> },
    /// using the login1 D-Bus and fs for reading. This works for keyboard and screen brightness, but needs the filesystem for reading the data and D-Bus for adjusting.
    Systemd {
        /// The subsystem to read the data from, e.g. `backlight` or `leds`. Subsystem refers to the directory within `/sys/class/`.
        ///
        /// **Default**: `backlight`
        subsystem: Box<str>,

        /// The name of the resource, see `/sys/class/<subsystem>` for available resources. If empty, ironbar will try to resolve it via a hardcoded list of common resources
        ///
        /// **Default**: `None`
        name: Option<Box<str>>,
    },
}

const PATH_RESOLVED: &str = "path to be resolved";
const NAME_RESOLVED: &str = "name to be resolved";

impl BrightnessDataSource {
    /// Resolves empty field from [BrightnessDataSource].
    ///
    /// # Error
    ///
    /// When no default namespace could be resolved.
    fn resolve(&mut self) -> Result<(), ()> {
        match self {
            BrightnessDataSource::Keyboard { path } => {
                if path.is_some() {
                    return Ok(());
                }

                *path = Some(Box::from(
                    "/org/freedesktop/UPower/KbdBacklight/kdb_backlight",
                ));
            }
            BrightnessDataSource::Systemd { subsystem, name } => {
                if name.is_some() {
                    return Ok(());
                }

                *name = default_resource_name(subsystem);

                if name.is_none() {
                    return Err(());
                }
            }
        }

        Ok(())
    }

    /// Get the [Key] for the [BrightnessDataSource] in use.
    fn key(&self) -> Key {
        match self {
            BrightnessDataSource::Keyboard { path } => Key::Kbd {
                path: path.as_ref().expect(PATH_RESOLVED).clone(),
            },
            BrightnessDataSource::Systemd { subsystem, name } => Key::SysFs {
                subsystem: subsystem.clone(),
                device: name.as_ref().expect(NAME_RESOLVED).clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrightnessScaleControl {
    /// Let Ironbar decide what is best for your device.
    #[default]
    Automatic,
    /// Treat brightness data from your device as linear. This is the legacy behavior.
    Linear,
    /// Use a Gamma 2.2 curve to interpret brightness values.
    Gamma2_2,
    /// Use a Gamma 2.4 curve to interpret brightness values. see [wiki](https://en.wikipedia.org/wiki/SRGB#Transfer_function_(%22gamma%22))
    Gamma2_4,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BrightnessProfile {
    /// The format string to use for the widget button label.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Default**: `{percentage}%`
    format: String,

    /// The icon to show on the bar widget button.
    /// Supports [image](images) icons.
    ///
    /// **Default**: null
    icon_label: Option<String>,
}

impl Default for BrightnessProfile {
    fn default() -> Self {
        Self {
            format: "{percentage}%".to_string(),
            icon_label: None,
        }
    }
}

impl BrightnessProfile {
    fn with_icon(icon: &str) -> Self {
        Self {
            format: "{percentage}%".to_string(),
            icon_label: Some(icon.to_string()),
        }
    }
}

fn default_profiles() -> Profiles<f64, BrightnessProfile> {
    profiles!(
        "level0":5.0 => BrightnessProfile::with_icon(""),
        "level10":15.0 => BrightnessProfile::with_icon(""),
        "level20":25.0 => BrightnessProfile::with_icon(""),
        "level30":35.0 => BrightnessProfile::with_icon(""),
        "level40":45.0 => BrightnessProfile::with_icon(""),
        "level50":55.0 => BrightnessProfile::with_icon(""),
        "level60":65.0 => BrightnessProfile::with_icon(""),
        "level70":75.0 => BrightnessProfile::with_icon(""),
        "level80":85.0 => BrightnessProfile::with_icon(""),
        "level90":95.0 => BrightnessProfile::with_icon(""),
        "level100":100.0 => BrightnessProfile::with_icon("")
    )
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct BrightnessModule {
    /// See [profiles](profiles).
    #[serde(flatten)]
    profiles: Profiles<f64, BrightnessProfile>,

    /// Where to get the brightness data from
    ///
    /// See [BrightnessDataSource].
    mode: BrightnessDataSource,

    /// Control how to interpret brightness data from your device.
    ///
    /// See [BrightnessScaleControl].
    scale: BrightnessScaleControl,

    /// A multiplier from to control the speed of smooth scrolling on trackpad.
    /// Choose a negative number to swap scrolling direction.
    ///
    /// **Default**: `1.0`
    smooth_scroll_speed: f32,

    // -- Common --
    /// See [truncate options](module-level-options#truncate-mode).
    ///
    /// **Default**: `null`
    pub(crate) truncate: Option<TruncateMode>,

    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for BrightnessModule {
    fn default() -> Self {
        Self {
            profiles: Profiles::default(),
            mode: BrightnessDataSource::default(),
            scale: BrightnessScaleControl::default(),
            smooth_scroll_speed: 1.0,
            truncate: None,
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

impl Default for BrightnessDataSource {
    fn default() -> Self {
        BrightnessDataSource::Systemd {
            subsystem: Box::from("backlight"),
            name: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BrightnessProperties {
    screen_brightness: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    AdjustBrightnessScroll(f64),
}

const GAMMA_22: f32 = 2.2;
const GAMMA_24: f32 = 2.4;
const GAMMA_LOW_COEF: f32 = 12.92;
const GAMMA_HIGH_COEF: f32 = 1.055;
const GAMMA_OFFSET: f32 = 0.055;

const MIN_PERCENTAGE: f32 = 0.0;
const MAX_SETTABLE_PERCENTAGE: f32 = 1.0;

/// Convert `brightness` through a gamma curve.
fn gamma(brightness: i32, max_brightness: i32, gamma: f32) -> f32 {
    let linear = linear(brightness, max_brightness);

    if linear <= 0.0031308 {
        GAMMA_LOW_COEF * linear
    } else {
        GAMMA_HIGH_COEF * linear.powf(1f32 / gamma) - GAMMA_OFFSET
    }
    .max(MIN_PERCENTAGE)
}

/// Convert `percent` back to a brightness using a gamma curve.
fn revert_gamma(percent: f32, max_brightness: i32, gamma: f32) -> i32 {
    let linear = if percent <= 0.04045 {
        percent / GAMMA_LOW_COEF
    } else {
        ((percent + GAMMA_OFFSET) / GAMMA_HIGH_COEF).powf(gamma)
    };

    revert_linear(linear, max_brightness)
}

/// Convert `brightness` through a linear scale.
fn linear(brightness: i32, max_brightness: i32) -> f32 {
    (brightness as f32 / max_brightness as f32).max(MIN_PERCENTAGE)
}

/// Convert `percentage` back to a brightness values using a linear scale.
fn revert_linear(percentage: f32, max_brightness: i32) -> i32 {
    ((percentage * max_brightness as f32) as i32).clamp(0, max_brightness)
}

/// Convert the `brightness` to a percentage from 0.0 to 1.0.
///
/// `strategy` is used for the conversion strategy.
fn percent_from(
    brightness: i32,
    max_brightness: i32,
    strategy: &BrightnessScaleControl,
    scale: &Scale,
) -> f32 {
    match (strategy, scale) {
        (BrightnessScaleControl::Automatic, Scale::Unknown)
        | (BrightnessScaleControl::Linear, _) => linear(brightness, max_brightness),
        (BrightnessScaleControl::Gamma2_2, _) => gamma(brightness, max_brightness, GAMMA_22),
        (BrightnessScaleControl::Gamma2_4, _)
        | (BrightnessScaleControl::Automatic, Scale::Linear) => {
            gamma(brightness, max_brightness, GAMMA_24)
        }
    }
}

/// Convert the `percentage` value back into a brightness value.s
///
/// `strategy` is used for the conversion strategy.
fn brightness_from(
    percentage: f32,
    max_brightness: i32,
    strategy: &BrightnessScaleControl,
    scale: &Scale,
) -> i32 {
    match (strategy, scale) {
        (BrightnessScaleControl::Automatic, Scale::Unknown)
        | (BrightnessScaleControl::Linear, _) => revert_linear(percentage, max_brightness),
        (BrightnessScaleControl::Gamma2_2, _) => revert_gamma(percentage, max_brightness, GAMMA_22),
        (BrightnessScaleControl::Gamma2_4, _)
        | (BrightnessScaleControl::Automatic, Scale::Linear) => {
            revert_gamma(percentage, max_brightness, GAMMA_24)
        }
    }
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
            let Some((max_brightness, scale)) = ({
                let state = match client.state().await {
                    Ok(state) => state,
                    Err(err) => {
                        error!("could not read client's state: {err}");
                        return;
                    }
                };

                let mut init_values = None;
                for device in state.tracked {
                    if device.key() == key.get() {
                        init_values = Some(match device {
                            brightness::BrightnessDeviceState::DBus { max_brightness, .. } => {
                                (max_brightness, Scale::Unknown)
                            }
                            brightness::BrightnessDeviceState::SysFs {
                                max_brightness,
                                scale,
                                ..
                            } => (max_brightness, scale),
                        })
                    }
                }

                init_values
            }) else {
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
