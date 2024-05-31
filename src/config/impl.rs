use super::{BarConfig, BarPosition, MonitorConfig};
use color_eyre::{Help, Report};
use gtk::Orientation;
use serde::{Deserialize, Deserializer};

// Manually implement for better untagged enum error handling:
// currently open pr: https://github.com/serde-rs/serde/pull/1544
impl<'de> Deserialize<'de> for MonitorConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let content =
            <serde::__private::de::Content as serde::Deserialize>::deserialize(deserializer)?;

        match <BarConfig as serde::Deserialize>::deserialize(
            serde::__private::de::ContentRefDeserializer::<D::Error>::new(&content),
        ) {
            Ok(config) => Ok(Self::Single(config)),
            Err(outer) => match <Vec<BarConfig> as serde::Deserialize>::deserialize(
                serde::__private::de::ContentRefDeserializer::<D::Error>::new(&content),
            ) {
                Ok(config) => Ok(Self::Multiple(config)),
                Err(inner) => {
                    let report = Report::msg(format!(" multi-bar (c): {inner}").replace("An error occurred when deserializing: ", ""))
                        .wrap_err(format!("single-bar (b): {outer}").replace("An error occurred when deserializing: ", ""))
                        .wrap_err("An invalid config was found. The following errors were encountered:")
                        .note("Both the single-bar (type b / error 1) and multi-bar (type c / error 2) config variants were tried. You can likely ignore whichever of these is not relevant to you.")
                        .suggestion("Please see https://github.com/JakeStanger/ironbar/wiki/configuration-guide#2-pick-your-use-case for more info on the above");

                    Err(serde::de::Error::custom(format!("{report:?}")))
                }
            },
        }
    }
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
    pub const fn get_angle(self) -> f64 {
        match self {
            Self::Top | Self::Bottom => 0.0,
            Self::Left => 90.0,
            Self::Right => 270.0,
        }
    }
}
