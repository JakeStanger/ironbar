use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::brightness::{self, default_resource_name};
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
pub enum BrightnessDataSource {
    /// using the keyboard dbus. Note: this only works for keyboards, not for screen brightness.
    Keyboard,
    /// using the login1 dbus and fs for reading. This works for keyboard and screen brightness, but needs the filesystem for reading the data and dbus for adjusting.
    Login1Fs {
        /// The subsystem to take the data from, e.g. `backlight` or `leds`.
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
    datasource: BrightnessDataSource,

    /// The number of milliseconds between refreshing memory data.
    ///
    /// **Default**: `5000`
    interval: u64,

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
            datasource: BrightnessDataSource::default(),
            interval: 5000,
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
            .rev() // iterate from the end
            .find(|&&(v, _)| percent >= v) // find the first where value >= num
            .map(|(_, label)| label.clone())
            .unwrap_or("".to_string())
    }
}

#[derive(Clone, Debug, Default)]
pub struct BrightnessProperties {
    screen_brightness: f64,
    icon_name: String,
}

impl BrightnessModule {
    async fn read_percentage(
        client: &brightness::Client,
        datasource: &BrightnessDataSource,
        default_resource_name: &Option<String>,
    ) -> Result<(f64, i32, i32)> {
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
                let brightness_screen = client.screen_reader().brightness(subsystem, &name)?;
                let max_brightness_screen =
                    client.screen_reader().max_brightness(subsystem, &name)?;
                (brightness_screen, max_brightness_screen)
            }
        };

        let percent: f64 = current as f64 / (max as f64) * 100.0;

        Ok((percent, current, max))
    }

    async fn write_brightness(
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
    /// Does nothing more than force a fresh of the brightness
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
        let ctx = context.controller_tx.clone();
        let client = context.try_client::<brightness::Client>()?;
        let icons = self.icons.clone();
        let datasource = self.datasource.clone();
        let duration = tokio::time::Duration::from_millis(self.interval);
        let default_resource_name =
            if let BrightnessDataSource::Login1Fs { subsystem, .. } = &datasource {
                default_resource_name(subsystem)
            } else {
                None
            };

        spawn(async move {
            // make sure we have a value on startup and not have to wait for 1 interval
            let _ = ctx.send(UiEvent::Refresh).await;

            loop {
                let event = tokio::select! {
                    v = rx.recv() => v,
                    _ = sleep(duration) => None,
                };

                let (mut percent, cur, max) =
                    match Self::read_percentage(&client, &datasource, &default_resource_name).await
                    {
                        Ok(d) => d,
                        Err(e) => {
                            tracing::error!(?e, "Could not retrieve brightness levels");
                            continue;
                        }
                    };

                if let Some(UiEvent::AdjustBrightnessScroll(dy)) = event {
                    let step = ((max as f64 / 100.0) as i32).max(1); // at least modify by 1 if max < 100
                    let new_cur = if dy > 0.0 {
                        cur - step
                    } else if dy < 0.0 {
                        cur + step
                    } else {
                        continue;
                    };

                    let new_cur = new_cur.max(0).min(max); // not using .clamp to avoid panic in case max is ever < 0
                    percent = new_cur as f64 / (max as f64) * 100.0;

                    Self::write_brightness(&client, &datasource, &default_resource_name, new_cur)
                        .await;
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

        let ctx = context.controller_tx.clone();

        let scroll_controller = EventControllerScroll::new(EventControllerScrollFlags::VERTICAL);
        scroll_controller.connect_scroll(move |_, _, dy| {
            let ctx = ctx.clone();
            spawn(async move { ctx.send(UiEvent::AdjustBrightnessScroll(dy)).await });

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
