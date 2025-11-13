use super::fs_brightness;
use crate::await_sync;
use std::path::PathBuf;
use std::sync::Arc;

const KDB_NAMESPACE: &str = "kbd_backlight";
const KDB_KEY_MAX_BRIGHTNESS: &str = "max";
const KDB_KEY_BRIGHTNESS: &str = "current";
const BACKLIGHT_CURRENT_NAMESPACE: &str = "backlight_current";
const BACKLIGHT_MAX_NAMESPACE: &str = "backlight_max";
const LEDS_CURRENT_NAMESPACE: &str = "leds_current";
const LEDS_MAX_NAMESPACE: &str = "leds_max";

#[derive(Debug)]
pub(super) struct KbdBacklight {
    keyboard: Arc<super::KbdBacklightProxy<'static>>,
}

#[derive(Debug)]
pub(super) struct FsBacklight {
    screen_reader: Arc<super::FsLogin1Session>,
    subsystem: String,
    property: Property,
}

#[derive(Debug)]
enum Property {
    Brightness,
    MaxBrightness,
}

impl crate::ironvar::Namespace for KbdBacklight {
    fn get(&self, key: &str) -> Option<String> {
        match key {
            KDB_KEY_MAX_BRIGHTNESS => await_sync(self.keyboard.get_max_brightness())
                .ok()
                .map(|v| v.to_string()),

            KDB_KEY_BRIGHTNESS => await_sync(self.keyboard.get_max_brightness())
                .ok()
                .map(|v| v.to_string()),
            _ => None,
        }
    }

    fn list(&self) -> Vec<String> {
        [KDB_KEY_MAX_BRIGHTNESS, KDB_KEY_BRIGHTNESS]
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn namespaces(&self) -> Vec<String> {
        vec![]
    }

    fn get_namespace(&self, _key: &str) -> Option<crate::ironvar::NamespaceTrait> {
        None
    }
}

impl crate::ironvar::Namespace for FsBacklight {
    fn get(&self, key: &str) -> Option<String> {
        match self.property {
            Property::Brightness => self.screen_reader.brightness(&self.subsystem, key),
            Property::MaxBrightness => self.screen_reader.max_brightness(&self.subsystem, key),
        }
        .ok()
        .map(|v| v.to_string())
    }

    fn list(&self) -> Vec<String> {
        let mut entries = Vec::new();

        let mut subsystem_path = PathBuf::from(fs_brightness::SYS_PATH);
        subsystem_path.push(&self.subsystem);

        if let Ok(resource_entries) = std::fs::read_dir(&subsystem_path) {
            for resource_entry in resource_entries.flatten() {
                let resource_path = resource_entry.path();
                let name = resource_path
                    .file_name()
                    .and_then(|p| p.to_str())
                    .map(|p| p.to_string());
                if let Some(name) = name {
                    entries.push(name);
                }
            }
        }

        entries
    }

    fn namespaces(&self) -> Vec<String> {
        vec![]
    }

    fn get_namespace(&self, _key: &str) -> Option<crate::ironvar::NamespaceTrait> {
        None
    }
}

impl crate::ironvar::Namespace for super::Client {
    fn get(&self, _: &str) -> Option<String> {
        None
    }

    fn list(&self) -> Vec<String> {
        Vec::new()
    }

    fn namespaces(&self) -> Vec<String> {
        vec![
            KDB_NAMESPACE.to_string(),
            BACKLIGHT_CURRENT_NAMESPACE.to_string(),
            BACKLIGHT_MAX_NAMESPACE.to_string(),
            LEDS_CURRENT_NAMESPACE.to_string(),
            LEDS_MAX_NAMESPACE.to_string(),
        ]
    }

    fn get_namespace(&self, key: &str) -> Option<crate::ironvar::NamespaceTrait> {
        match key {
            KDB_NAMESPACE => Some(Arc::new(KbdBacklight {
                keyboard: self.keyboard.clone(),
            })),
            BACKLIGHT_CURRENT_NAMESPACE => Some(Arc::new(FsBacklight {
                screen_reader: self.screen_reader.clone(),
                subsystem: "backlight".to_string(),
                property: Property::Brightness,
            })),
            BACKLIGHT_MAX_NAMESPACE => Some(Arc::new(FsBacklight {
                screen_reader: self.screen_reader.clone(),
                subsystem: "backlight".to_string(),
                property: Property::MaxBrightness,
            })),
            LEDS_CURRENT_NAMESPACE => Some(Arc::new(FsBacklight {
                screen_reader: self.screen_reader.clone(),
                subsystem: "leds".to_string(),
                property: Property::Brightness,
            })),
            LEDS_MAX_NAMESPACE => Some(Arc::new(FsBacklight {
                screen_reader: self.screen_reader.clone(),
                subsystem: "leds".to_string(),
                property: Property::MaxBrightness,
            })),
            _ => None,
        }
    }
}
