use super::{BarPosition, Config, MonitorConfig};
use color_eyre::eyre::Result;
use color_eyre::eyre::{ContextCompat, WrapErr};
use color_eyre::{Help, Report};
use dirs::config_dir;
use gtk::Orientation;
use serde::{Deserialize, Deserializer};
use std::path::{Path, PathBuf};
use std::{env, fs};
use tracing::instrument;

// Manually implement for better untagged enum error handling:
// currently open pr: https://github.com/serde-rs/serde/pull/1544
impl<'de> Deserialize<'de> for MonitorConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let content =
            <serde::__private::de::Content as serde::Deserialize>::deserialize(deserializer)?;

        match <Config as serde::Deserialize>::deserialize(
            serde::__private::de::ContentRefDeserializer::<D::Error>::new(&content),
        ) {
            Ok(config) => Ok(Self::Single(config)),
            Err(outer) => match <Vec<Config> as serde::Deserialize>::deserialize(
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
    pub fn get_orientation(self) -> Orientation {
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

impl Config {
    /// Attempts to load the config file from file,
    /// parse it and return a new instance of `Self`.
    pub fn load() -> Result<Self> {
        let config_path = env::var("IRONBAR_CONFIG").map_or_else(
            |_| Self::try_find_config(),
            |config_path| {
                let path = PathBuf::from(config_path);
                if path.exists() {
                    Ok(path)
                } else {
                    Err(Report::msg(format!(
                        "Specified config file does not exist: {}",
                        path.display()
                    ))
                    .note("Config file was specified using `IRONBAR_CONFIG` environment variable"))
                }
            },
        )?;

        Self::load_file(&config_path)
    }

    /// Attempts to discover the location of the config file
    /// by checking each valid format's extension.
    ///
    /// Returns the path of the first valid match, if any.
    #[instrument]
    fn try_find_config() -> Result<PathBuf> {
        let config_dir = config_dir().wrap_err("Failed to locate user config dir")?;

        let extensions = vec!["json", "toml", "yaml", "yml", "corn"];

        let file = extensions.into_iter().find_map(|extension| {
            let full_path = config_dir
                .join("ironbar")
                .join(format!("config.{extension}"));

            if Path::exists(&full_path) {
                Some(full_path)
            } else {
                None
            }
        });

        file.map_or_else(
            || {
                Err(Report::msg("Could not find config file")
                    .suggestion("Ironbar does not include a configuration out of the box")
                    .suggestion("A guide on writing a config can be found on the wiki:")
                    .suggestion("https://github.com/JakeStanger/ironbar/wiki/configuration-guide"))
            },
            Ok,
        )
    }

    /// Loads the config file at the specified path
    /// and parses it into `Self` based on its extension.
    fn load_file(path: &Path) -> Result<Self> {
        let file = fs::read(path).wrap_err("Failed to read config file")?;
        let extension = path
            .extension()
            .unwrap_or_default()
            .to_str()
            .unwrap_or_default();

        match extension {
            "json" => serde_json::from_slice(&file).wrap_err("Invalid JSON config"),
            "toml" => toml::from_slice(&file).wrap_err("Invalid TOML config"),
            "yaml" | "yml" => serde_yaml::from_slice(&file).wrap_err("Invalid YAML config"),
            "corn" => libcorn::from_slice(&file).wrap_err("Invalid Corn config"),
            _ => unreachable!(),
        }
    }
}
