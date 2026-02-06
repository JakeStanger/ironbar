use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::brightness::{self, brightness, default_resource_name, max_brightness};
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
use tokio::time::sleep;

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BrightnessDataSource {
    /// using the keyboard dbus. Note: this only works for keyboards, not for screen brightness.
    Keyboard,
    /// using the login1 dbus and fs for reading. This works for keyboard and screen brightness, but needs the filesystem for reading the data and dbus for adjusting.
    Systemd {
        /// The subsystem to read the data from, e.g. `backlight` or `leds`. Subsystem refers to the directory within `/sys/class/`.
        ///
        /// **Default**: `backlight`
        subsystem: String,

        /// The name of the resource, see `/sys/class/<subsystem>` for available resources. If empty, ironbar will try to resolve it via a hardcoded list of common resources
        ///
        /// **Default**: `None`
        name: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Deserialize)]
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

    /// A multiplier from to control the speed of smooth scrolling on trackpad.
    /// Choose a negative number to swap scrolling direction.
    ///
    /// **Default**: `1.0`
    smooth_scroll_speed: f64,

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
            subsystem: "backlight".to_string(),
            name: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BrightnessProperties {
    screen_brightness: f64,
}

struct BrightnessData {
    percent: f64,
    current: i32,
    max: i32,
}

#[derive(Debug)]
enum SystemdError {
    NoResourceName,
}

impl std::fmt::Display for SystemdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SystemdError::NoResourceName => write!(
                f,
                "No Resource name was provided and no resource name could be resolved for your subsystem"
            ),
        }
    }
}

impl std::error::Error for SystemdError {}

impl BrightnessModule {
    async fn get_brightness(
        client: &brightness::Client,
        datasource: &BrightnessDataSource,
        default_resource_name: &Option<String>,
    ) -> Result<BrightnessData> {
        let (current, max): (i32, i32) = match &datasource {
            BrightnessDataSource::Keyboard => {
                let brightness_kbd = client.keyboard().get_brightness().await?;
                let max_brightness_kbd = client.keyboard().get_max_brightness().await?;
                (brightness_kbd, max_brightness_kbd)
            }
            BrightnessDataSource::Systemd { subsystem, name } => {
                let name = name
                    .clone()
                    .or_else(|| default_resource_name.clone())
                    .ok_or(SystemdError::NoResourceName)?;
                let brightness_screen = brightness(subsystem, &name)?;
                let max_brightness_screen = max_brightness(subsystem, &name)?;
                (brightness_screen, max_brightness_screen)
            }
        };

        let percent: f64 = current as f64 / (max as f64) * 100.0;

        Ok(BrightnessData {
            percent,
            current,
            max,
        })
    }

    async fn set_brightness(
        client: &brightness::Client,
        datasource: &BrightnessDataSource,
        default_resource_name: &Option<String>,
        brightness: i32,
    ) -> Result<()> {
        match &datasource {
            BrightnessDataSource::Keyboard => {
                client.keyboard().set_brightness(brightness).await?;
            }
            BrightnessDataSource::Systemd { subsystem, name } => {
                let name = name
                    .clone()
                    .or_else(|| default_resource_name.clone())
                    .ok_or(SystemdError::NoResourceName)?;
                client
                    .screen_writer()
                    .set_brightness(subsystem.to_string(), name, brightness as u32)
                    .await?;
            }
        };
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum UiEvent {
    AdjustBrightnessScroll(f64),
    Refresh,
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
        const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);
        let tx = context.tx.clone();
        let controller_tx = context.controller_tx.clone();
        let client = context.try_client::<brightness::Client>()?;
        let datasource = self.mode.clone();
        let scroll_speed = self.smooth_scroll_speed;
        let default_resource_name =
            if let BrightnessDataSource::Systemd { subsystem, .. } = &datasource {
                default_resource_name(subsystem)
            } else {
                None
            };

        spawn(async move {
            // make sure we have a value on startup and not have to wait for 1 interval
            controller_tx.send_expect(UiEvent::Refresh).await;

            let mut partial_scroll: f64 = 0.0;

            loop {
                let event = tokio::select! {
                    v = rx.recv() => v,
                    _ = sleep(POLL_INTERVAL) => None,
                };

                let BrightnessData {
                    mut percent,
                    current,
                    max,
                } = match Self::get_brightness(&client, &datasource, &default_resource_name).await {
                    Ok(d) => d,
                    Err(err) => match err.downcast::<SystemdError>() {
                        Ok(err) => {
                            tracing::error!(
                                ?err,
                                "Could not retrieve brightness levels. Error is unrecoverable, fix the config! Stopping brightness module"
                            );
                            break;
                        }
                        Err(err) => {
                            tracing::error!(?err, "Could not retrieve brightness levels");
                            continue;
                        }
                    },
                };

                if let Some(UiEvent::AdjustBrightnessScroll(dy)) = event {
                    partial_scroll += dy * scroll_speed;

                    if partial_scroll.abs() >= 1.0 {
                        let num_steps = partial_scroll.floor() as i32;
                        partial_scroll -= partial_scroll.floor();

                        let step_len = (max / 100).max(1); // ensure step_len is at least 1, otherwise the div by i32 might produce a step_len of 0 due to truncing

                        let new_brightness = (current - num_steps * step_len).max(0).min(max); // not using .clamp to avoid panic in case max is ever < 0
                        percent = new_brightness as f64 / (max as f64) * 100.0;

                        if let Err(err) = Self::set_brightness(
                            &client,
                            &datasource,
                            &default_resource_name,
                            new_brightness,
                        )
                        .await
                        {
                            tracing::error!(?err, "Could not change brightness");
                        };
                    }
                }

                tx.send_update(BrightnessProperties {
                    screen_brightness: percent,
                })
                .await;
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
            let percentage = properties.screen_brightness;
            manager.update(percentage, ());
        });

        Ok(ModuleParts::new(button, None))
    }
}
