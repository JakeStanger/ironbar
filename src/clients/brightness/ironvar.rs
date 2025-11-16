use super::{brightness, fs_reader::available_resource_names, max_brightness};
use crate::await_sync;
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
            Property::Brightness => brightness(&self.subsystem, key),
            Property::MaxBrightness => max_brightness(&self.subsystem, key),
        }
        .ok()
        .map(|v| v.to_string())
    }

    fn list(&self) -> Vec<String> {
        available_resource_names(&self.subsystem)
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
                subsystem: "backlight".to_string(),
                property: Property::Brightness,
            })),
            BACKLIGHT_MAX_NAMESPACE => Some(Arc::new(FsBacklight {
                subsystem: "backlight".to_string(),
                property: Property::MaxBrightness,
            })),
            LEDS_CURRENT_NAMESPACE => Some(Arc::new(FsBacklight {
                subsystem: "leds".to_string(),
                property: Property::Brightness,
            })),
            LEDS_MAX_NAMESPACE => Some(Arc::new(FsBacklight {
                subsystem: "leds".to_string(),
                property: Property::MaxBrightness,
            })),
            _ => None,
        }
    }
}
