use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::brightness::{self, brightness, default_resource_name, max_brightness};
use crate::config::{CommonConfig, LayoutConfig, TruncateMode};
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
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
    #[serde(rename = "login1")]
    /// using the login1 dbus and fs for reading. This works for keyboard and screen brightness, but needs the filesystem for reading the data and dbus for adjusting.
    Login1Fs {
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
pub struct BrightnessModule {
    /// The format string to use for the widget button label.
    /// For available tokens, see [below](#formatting-tokens).
    ///
    /// **Default**: `{icon} {percentage}%`
    format: String,

    /// Brightness state icons.
    ///
    /// See [icons](#icons).
    icons: Icons,

    /// Where to get the brightness data from
    ///
    /// See [BrightnessDataSource].
    mode: BrightnessDataSource,

    /// The number of milliseconds between refreshing memory data.
    ///
    /// **Default**: `1000`
    interval: u64,

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
            format: "{icon} {percentage}%".to_string(),
            icons: Icons::default(),
            mode: BrightnessDataSource::default(),
            interval: 1000,
            smooth_scroll_speed: 1.0,
            truncate: None,
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

impl Default for BrightnessDataSource {
    fn default() -> Self {
        BrightnessDataSource::Login1Fs {
            subsystem: "backlight".to_string(),
            name: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct Icons {
    /// Icon to show for respective brightness levels. Needs to be sorted.
    ///
    /// **Default**: `[(0, ""), (12, ""), (24, ""), (36, ""), (48, ""), (60,""), (72, ""), (84, ""), (100, "")]`
    brightness: Vec<(u32, String)>,
}

impl Default for Icons {
    fn default() -> Self {
        Self {
            brightness: [
                (0, ""),
                (12, ""),
                (24, ""),
                (36, ""),
                (48, ""),
                (60, ""),
                (72, ""),
                (84, ""),
                (100, ""),
            ]
            .into_iter()
            .map(|(b, i)| (b, i.to_string()))
            .collect(),
        }
    }
}

impl Icons {
    fn brightness_icon(&self, percent: f64) -> String {
        let percent = percent as u32;
        self.brightness
            .iter()
            .rev()
            .find(|&&(v, _)| percent >= v)
            .map(|(_, label)| label.clone())
            .unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BrightnessProperties {
    screen_brightness: f64,
    icon_name: String,
}

struct BrightnessData {
    percent: f64,
    current: i32,
    max: i32,
}

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
            BrightnessDataSource::Login1Fs { subsystem, name } => {
                let name = name
                    .clone()
                    .or_else(|| default_resource_name.clone())
                    .expect(
                        "Could not get resource name, consider explicit setting datasource.name",
                    );
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
    ) {
        match &datasource {
            BrightnessDataSource::Keyboard => {
                if let Err(e) = client.keyboard().set_brightness(brightness).await {
                    tracing::error!(?e, "Could not change brightness");
                }
            }
            BrightnessDataSource::Login1Fs { subsystem, name } => {
                let name = name
                    .clone()
                    .or_else(|| default_resource_name.clone())
                    .expect(
                        "Could not get resource name, consider explicit setting datasource.name",
                    );
                if let Err(e) = client
                    .screen_writer()
                    .set_brightness(subsystem.to_string(), name, brightness as u32)
                    .await
                {
                    tracing::error!(?e, "Could not change brightness");
                }
            }
        };
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

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
        let controller_tx = context.controller_tx.clone();
        let client = context.try_client::<brightness::Client>()?;
        let icons = self.icons.clone();
        let datasource = self.mode.clone();
        let scroll_speed = self.smooth_scroll_speed;
        let duration = tokio::time::Duration::from_millis(self.interval);
        let default_resource_name =
            if let BrightnessDataSource::Login1Fs { subsystem, .. } = &datasource {
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
                    _ = sleep(duration) => None,
                };

                let BrightnessData {
                    mut percent,
                    current,
                    max,
                } = match Self::get_brightness(&client, &datasource, &default_resource_name).await {
                    Ok(d) => d,
                    Err(e) => {
                        tracing::error!(?e, "Could not retrieve brightness levels");
                        continue;
                    }
                };

                if let Some(UiEvent::AdjustBrightnessScroll(dy)) = event {
                    partial_scroll += dy * scroll_speed;

                    if partial_scroll.abs() >= 1.0 {
                        let num_steps = partial_scroll.floor() as i32;
                        partial_scroll -= partial_scroll.floor();

                        let step_len = (max / 100).max(1); // ensure step_len is at least 1, otherwise the div by i32 might produce a step_len of 0 due to truncing

                        let new_brightness = (current - num_steps * step_len).max(0).min(max); // not using .clamp to avoid panic in case max is ever < 0
                        percent = new_brightness as f64 / (max as f64) * 100.0;

                        Self::set_brightness(
                            &client,
                            &datasource,
                            &default_resource_name,
                            new_brightness,
                        )
                        .await;
                    }
                }

                tx.send_update(BrightnessProperties {
                    icon_name: icons.brightness_icon(percent).to_string(),
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
        _: &ModuleInfo,
    ) -> color_eyre::Result<ModuleParts<Button>>
    where
        <Self as Module<Button>>::SendMessage: Clone,
    {
        let button_label = Label::builder()
            .use_markup(true)
            .justify(self.layout.justify.into())
            .build();

        let button = Button::new();
        button.set_child(Some(&button_label));

        let controller_tx = context.controller_tx.clone();

        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll_controller.connect_scroll(move |_, _, dy| {
            let ctx = controller_tx.clone();
            ctx.send_spawn(UiEvent::AdjustBrightnessScroll(dy));

            glib::Propagation::Proceed
        });
        button.add_controller(scroll_controller);

        let rx = context.subscribe();

        rx.recv_glib(&self.format, move |format, properties| {
            let percentage = properties.screen_brightness;
            let format = format
                .replace("{icon}", &properties.icon_name)
                .replace("{percentage}", &percentage.round().to_string());

            button_label.set_label(&format);
        });

        Ok(ModuleParts::new(button, None))
    }
}
