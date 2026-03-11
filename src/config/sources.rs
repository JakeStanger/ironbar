use cfg_if::cfg_if;
use config::FileFormat;
use std::path::{Path, PathBuf};
use tracing::error;

use super::ConfigLocation;

#[derive(Debug, Clone)]
pub enum Builtin {
    Minimal,
    Desktop,
}

impl Builtin {
    pub fn css(&self) -> &'static str {
        match self {
            Self::Minimal => include_str!("../../examples/minimal/style.css"),
            Self::Desktop => include_str!("../../examples/desktop/style.css"),
        }
    }
}

#[cfg(feature = "config")]
impl Builtin {
    pub fn config(&self) -> (&'static str, config::FileFormat) {
        cfg_if! {
            if #[cfg(feature = "config+corn")] {
                match self {
                    Self::Minimal => (include_str!("../../examples/minimal/config.corn"), FileFormat::Corn),
                    Self::Desktop => (include_str!("../../examples/desktop/config.corn"), FileFormat::Corn)
                }
            } else if #[cfg(feature = "config+json")] {
                match self {
                    Self::Minimal => (include_str!("../../examples/minimal/config.json"), FileFormat::Json),
                    Self::Desktop => (include_str!("../../examples/desktop/config.json"), FileFormat::Json)
                }
            } else if #[cfg(feature = "config+yaml")] {
                match self {
                    Self::Minimal => (include_str!("../../examples/minimal/config.yaml"), FileFormat::Yaml),
                    Self::Desktop => (include_str!("../../examples/desktop/config.yaml"), FileFormat::Yaml)
                }
            } else if #[cfg(feature = "config+toml")] {
                match self {
                    Self::Minimal => (include_str!("../../examples/minimal/config.toml"), FileFormat::Toml),
                    Self::Desktop => (include_str!("../../examples/desktop/config.toml"), FileFormat::Toml)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConfigSource {
    Builtin(Builtin),
    File(PathBuf),
}

impl ConfigSource {
    pub fn xdg_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_default()
            .to_path_buf()
            .join("ironbar/config")
    }
}

#[derive(Debug, Clone)]
pub enum CssSource {
    Builtin(Builtin),
    File(PathBuf),
}

#[cfg(feature = "config")]
pub fn resolve_sources(
    cli_config: Option<ConfigLocation>,
    cli_css: Option<ConfigLocation>,
    env_config: Option<String>,
    env_css: Option<String>,
) -> (ConfigSource, CssSource) {
    let config = resolve_config(cli_config, env_config);
    let css = resolve_css(cli_css, env_css, &config);
    (config, css)
}

#[cfg(not(feature = "config"))]
pub fn resolve_sources(
    cli_config: Option<ConfigLocation>,
    cli_css: Option<ConfigLocation>,
    env_config: Option<String>,
    env_css: Option<String>,
) -> (ConfigSource, CssSource) {
    panic!(
        "Ironbar has been configured without config support. This won't work. Please reconfigure with at least one `config` feature flag enabled."
    )
}

fn resolve_config(cli: Option<ConfigLocation>, env: Option<String>) -> ConfigSource {
    // IRONBAR_CONFIG env var
    if let Some(path) = env {
        return ConfigSource::File(PathBuf::from(path));
    }

    // --config CLI arg
    if let Some(loc) = cli {
        return match loc {
            ConfigLocation::Minimal => ConfigSource::Builtin(Builtin::Minimal),
            ConfigLocation::Desktop => ConfigSource::Builtin(Builtin::Desktop),
            ConfigLocation::Custom(path) => ConfigSource::File(path),
        };
    }

    // XDG_CONFIG_DIR user config
    let xdg = dirs::config_dir()
        .unwrap_or_default()
        .join("ironbar/config");
    if config_source_exists(&xdg) {
        return ConfigSource::File(xdg);
    }

    // /etc/ system config
    let etc = Path::new("/etc/ironbar/config");
    if config_source_exists(etc) {
        return ConfigSource::File(etc.to_path_buf());
    }

    // Fallback: minimal config
    ConfigSource::Builtin(Builtin::Minimal)
}

fn resolve_css(
    cli: Option<ConfigLocation>,
    env: Option<String>,
    config: &ConfigSource,
) -> CssSource {
    // IRONBAR_CSS env var
    if let Some(path) = env {
        return CssSource::File(css_path(PathBuf::from(path)));
    }

    // --theme CLI arg
    if let Some(loc) = cli {
        return match loc {
            ConfigLocation::Minimal => CssSource::Builtin(Builtin::Minimal),
            ConfigLocation::Desktop => CssSource::Builtin(Builtin::Desktop),
            ConfigLocation::Custom(path) => {
                let style = css_path(path);
                if style.exists() {
                    CssSource::File(style)
                } else {
                    error!(
                        "styles at '{}' not found, falling back to minimal theme",
                        style.display()
                    );
                    CssSource::Builtin(Builtin::Minimal)
                }
            }
        };
    }

    // CSS from Config dir
    match config {
        ConfigSource::Builtin(b) => CssSource::Builtin(b.clone()),
        ConfigSource::File(path) => {
            let style = css_path(path.clone());
            if style.exists() {
                CssSource::File(style)
            } else {
                error!(
                    "styles at '{}' not found, falling back to minimal theme",
                    style.display()
                );
                CssSource::Builtin(Builtin::Minimal)
            }
        }
    }
}

fn config_source_exists(base: &Path) -> bool {
    config::Config::builder()
        .add_source(config::File::from(base))
        .build()
        .is_ok()
}

fn css_path(path: PathBuf) -> PathBuf {
    if path.is_dir() {
        path.join("style.css")
    } else if path.extension().is_none_or(|ext| ext != "css") {
        path.parent().unwrap_or(&path).join("style.css")
    } else {
        path
    }
}
