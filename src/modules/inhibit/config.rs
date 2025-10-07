use crate::config::{CommonConfig, LayoutConfig};
use serde::{Deserialize, Deserializer};
use std::time::Duration;

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum BackendType {
    Systemd,
    Wayland,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum InhibitCommand {
    Toggle,
    Cycle,
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(default)]
pub struct InhibitModule {
    pub(super) backend: Option<BackendType>,
    #[serde(deserialize_with = "deserialize_durations")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<Vec<String>>"))]
    pub(super) durations: (Vec<Duration>, usize),
    pub(super) on_click_left: Option<InhibitCommand>,
    pub(super) on_click_right: Option<InhibitCommand>,
    pub(super) on_click_middle: Option<InhibitCommand>,
    pub(super) format_on: String,
    pub(super) format_off: String,
    #[serde(flatten)]
    pub(super) layout: LayoutConfig,
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

impl Default for InhibitModule {
    fn default() -> Self {
        Self {
            backend: Some(BackendType::Systemd),
            durations: parse_durations_with_default(Vec::new()).unwrap(),
            on_click_left: Some(InhibitCommand::Toggle),
            on_click_right: Some(InhibitCommand::Cycle),
            on_click_middle: None,
            format_on: "☕ {duration}".to_string(),
            format_off: "💤 {duration}".to_string(),
            layout: LayoutConfig::default(),
            common: None,
        }
    }
}

fn parse_duration(s: &str) -> color_eyre::Result<(Duration, bool)> {
    // "*" prefix marks which duration is the default selection
    let is_default = s.trim().starts_with('*');
    let s = s.trim().trim_start_matches('*').trim();
    let duration = match s.to_lowercase().as_str() {
        "inf" | "infinity" | "infinite" | "∞" | "" => Duration::MAX,
        _ => humantime::parse_duration(s)?,
    };
    Ok((duration, is_default))
}

fn parse_durations_with_default(
    strings: Vec<String>,
) -> color_eyre::Result<(Vec<Duration>, usize)> {
    let strings_to_parse: Vec<&str> = if strings.is_empty() {
        vec!["30m", "1h", "1h30m", "*2h", "inf"]
    } else {
        strings.iter().map(|s| s.as_str()).collect()
    };

    let (mut durations, mut default_idx) = (Vec::new(), strings_to_parse.len() - 1);

    for (i, s) in strings_to_parse.iter().enumerate() {
        let (duration, is_default) = parse_duration(s)?;
        if is_default {
            default_idx = i;
        }
        durations.push(duration);
    }

    Ok((durations, default_idx))
}

fn deserialize_durations<'de, D>(deserializer: D) -> Result<(Vec<Duration>, usize), D::Error>
where
    D: Deserializer<'de>,
{
    parse_durations_with_default(Vec::deserialize(deserializer)?).map_err(serde::de::Error::custom)
}
