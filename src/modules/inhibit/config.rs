use crate::config::{CommonConfig, LayoutConfig};
use chrono::Timelike;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use std::time::Duration;

/// Command to control inhibit state.
///
/// **Valid options**: `toggle`, `cycle`
#[derive(Debug, Clone, Copy, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum InhibitCommand {
    /// Toggle inhibit on/off.
    Toggle,
    /// Cycle to next duration.
    Cycle,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct InhibitModule {
    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,

    /// See [DurationSpec]
    #[serde(flatten)]
    pub(super) duration_spec: DurationSpec,

    /// Command to execute on left click.
    ///
    /// **Valid options**: `toggle`, `cycle`
    /// <br>
    /// **Default**: `toggle`
    #[cfg_attr(feature = "extras", schemars(extend("default" = "'toggle'")))]
    pub(super) on_click_left: Option<InhibitCommand>,

    /// Command to execute on right click.
    ///
    /// **Valid options**: `toggle`, `cycle`
    /// <br>
    /// **Default**: `cycle`
    #[cfg_attr(feature = "extras", schemars(extend("default" = "'cycle'")))]
    pub(super) on_click_right: Option<InhibitCommand>,

    /// Command to execute on middle click.
    ///
    /// **Valid options**: `toggle`, `cycle`
    /// <br>
    /// **Default**: `null`
    pub(super) on_click_middle: Option<InhibitCommand>,

    /// Format string when inhibit is active.
    /// `{duration}` token shows remaining/selected time.
    ///
    /// Available tokens:
    ///
    /// | Token        | Description       |
    /// |--------------|-------------------|
    /// | `{duration}` | Current duration. |
    ///
    /// **Default**: `"☕ {duration}"`
    pub(super) format_on: String,

    /// Format string when inhibit is inactive.
    /// `{duration}` token shows selected duration.
    ///
    /// See [format_on](#format_on) for available tokens.
    ///
    /// **Default**: `"💤 {duration}"`
    pub(super) format_off: String,

    /// See [layout options](module-level-options#layout).
    #[serde(flatten)]
    pub(super) layout: LayoutConfig,
}

impl Default for InhibitModule {
    fn default() -> Self {
        Self {
            duration_spec: DurationSpec::default(),
            on_click_left: Some(InhibitCommand::Toggle),
            on_click_right: Some(InhibitCommand::Cycle),
            on_click_middle: None,
            format_on: "☕ {duration}".to_string(),
            format_off: "💤 {duration}".to_string(),
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    if matches!(s, "0" | "00:00:00" | "inf") {
        return Ok(Duration::MAX);
    }
    chrono::NaiveTime::parse_from_str(s, "%H:%M:%S")
        .map_err(|_| format!("invalid duration: {s}"))
        .map(|time| Duration::from_secs(time.num_seconds_from_midnight() as u64))
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub(super) struct DurationSpec {
    /// List of durations to cycle through.
    /// Use `0` for infinite inhibit. Format: `HH:MM:SS` (e.g., `01:30:00`).
    ///
    /// If `default_duration` is not specified, the first item in `durations` is used.
    #[cfg_attr(feature = "extras", schemars(with = "Vec<String>"))]
    #[cfg_attr(feature = "extras", schemars(extend("default" = ["00:30:00", "01:00:00", "01:30:00", "02:00:00", "inf"])))]
    pub(super) durations: Vec<Duration>,

    /// Starting duration. See [durations](#durations) above for information.
    #[cfg_attr(feature = "extras", schemars(with = "String"))]
    #[cfg_attr(feature = "extras", schemars(extend("default" = "00:30:00")))]
    pub(super) default_duration: Duration,
}

impl Default for DurationSpec {
    fn default() -> Self {
        let durations: Vec<Duration> = ["00:30:00", "01:00:00", "01:30:00", "02:00:00", "inf"]
            .iter()
            .map(|s| parse_duration(s).expect("default duration strings are valid"))
            .collect();

        Self {
            default_duration: durations[0],
            durations,
        }
    }
}

impl<'de> Deserialize<'de> for DurationSpec {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize, Default)]
        #[serde(default)]
        struct Raw {
            durations: Option<Vec<String>>,
            default_duration: Option<String>,
        }

        let r = Raw::deserialize(d)?;

        let durations = match r.durations {
            Some(strs) if !strs.is_empty() => strs
                .iter()
                .map(|s| parse_duration(s).map_err(Error::custom))
                .collect::<Result<Vec<_>, _>>()?,
            Some(_) => return Err(Error::custom("durations list cannot be empty")),
            None => Self::default().durations,
        };

        let default_duration = match r.default_duration {
            Some(ref s) => parse_duration(s).map_err(Error::custom)?,
            None => durations[0],
        };

        if !durations.contains(&default_duration) {
            return Err(Error::custom("default_duration must be in durations list"));
        }

        Ok(Self {
            durations,
            default_duration,
        })
    }
}
