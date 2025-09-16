use super::{BarConfig, BarPosition, MonitorConfig};
use color_eyre::{Help, Report};
use gtk::Orientation;
use serde::de::value::{MapAccessDeserializer, SeqAccessDeserializer};
use serde::{Deserialize, Deserializer, de};
use std::fmt;

// Manually implement for better untagged enum error handling:
// currently open pr: https://github.com/serde-rs/serde/pull/1544
impl<'de> Deserialize<'de> for MonitorConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V;
        impl<'de> de::Visitor<'de> for V {
            type Value = MonitorConfig;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("single bar config or array of bar configs")
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                let map_de = MapAccessDeserializer::new(map);
                let single_err = match BarConfig::deserialize(map_de) {
                    Ok(config) => return Ok(MonitorConfig::Single(config)),
                    Err(e) => e,
                };
                // Map can't be array, so create error with both attempts
                let r = Report::msg(" multi-bar (c): expected an array".to_string())
                    .wrap_err(format!("single-bar (b): {single_err}"))
                    .wrap_err("An invalid config was found. The following errors were encountered:")
                    .note("Both the single-bar (type b / error 1) and multi-bar (type c / error 2) config variants were tried. You can likely ignore whichever of these is not relevant to you.")
                    .suggestion("Please see https://github.com/JakeStanger/ironbar/wiki/configuration-guide#2-pick-your-use-case for more info on the above");
                Err(de::Error::custom(format!("{r:?}")))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
                let seq_de = SeqAccessDeserializer::new(seq);
                let multi_err = match Vec::<BarConfig>::deserialize(seq_de) {
                    Ok(config) => return Ok(MonitorConfig::Multiple(config)),
                    Err(e) => e,
                };
                // Seq can't be single bar, so create error with both attempts
                let r = Report::msg(format!(" multi-bar (c): {multi_err}"))
                    .wrap_err("single-bar (b): expected an object, got array")
                    .wrap_err("An invalid config was found. The following errors were encountered:")
                    .note("Both the single-bar (type b / error 1) and multi-bar (type c / error 2) config variants were tried. You can likely ignore whichever of these is not relevant to you.")
                    .suggestion("Please see https://github.com/JakeStanger/ironbar/wiki/configuration-guide#2-pick-your-use-case for more info on the above");
                Err(de::Error::custom(format!("{r:?}")))
            }
        }
        deserializer.deserialize_any(V)
    }
}

pub fn deserialize_layer<'de, D>(deserializer: D) -> Result<gtk_layer_shell::Layer, D::Error>
where
    D: Deserializer<'de>,
{
    use gtk_layer_shell::Layer;

    let value = Option::<String>::deserialize(deserializer)?;
    value.map_or(Ok(Layer::Top), |v| match v.as_str() {
        "background" => Ok(Layer::Background),
        "bottom" => Ok(Layer::Bottom),
        "top" => Ok(Layer::Top),
        "overlay" => Ok(Layer::Overlay),
        _ => Err(serde::de::Error::custom("invalid value for orientation")),
    })
}

#[cfg(feature = "schema")]
pub fn schema_layer(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "string",
        "enum": ["background", "bottom", "top", "overlay"],
    })
}

impl BarPosition {
    /// Gets the orientation the bar and widgets should use
    /// based on this position.
    pub fn orientation(self) -> Orientation {
        if self == Self::Top || self == Self::Bottom {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        }
    }

    /// Gets the angle that label text should be displayed at
    /// based on this position.
    pub const fn angle(self) -> f64 {
        match self {
            Self::Top | Self::Bottom => 0.0,
            Self::Left => 90.0,
            Self::Right => 270.0,
        }
    }
}
