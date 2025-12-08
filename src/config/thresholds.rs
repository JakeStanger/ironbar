use serde::Deserialize;
use std::collections::HashMap;

/// A representation of numeric thresholds
/// mapped in various forms to values of type `T`.
///
/// This is most useful for icon configuration,
/// allowing users to define the icons that should appear
/// as a value passes various thresholds.
/// For example, showing low/medium/high volume icons as volume changes.
#[derive(Debug, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
enum Thresholds<T> {
    /// Auto-calculated thresholds
    /// using pre-defined "low", "medium", "high" keys.
    ///
    /// # Example
    ///
    /// ```corn
    /// icons.low = "icon:volume_low"
    /// icons.medium = "icon:volume_medium"
    /// icons.high = "icon:volume_high"
    /// ```
    Basic { low: T, medium: T, high: T },

    /// Auto-calculated thresholds using an array
    /// where threshold boundaries are linearly separated
    /// based on the number of items.
    ///
    /// Values are rounded *down* to the nearest level.
    ///
    /// # Example
    ///
    /// ```corn
    /// icons = [ "icon:volume_low" "icon_volume_medium" "icon_volume_high" ]
    Dynamic(Vec<T>),

    /// Pre-defined thresholds using a map of levels to the values.
    /// This allows for non-linear behaviour.
    ///
    /// Values are rounded **down** to the nearest level.
    ///
    /// # Example
    ///
    /// ```corn
    /// icons.0 = "icon:volume_low"
    /// icons.33 = "icon:volume_medium"
    /// icons.66 = "icon_volume_high"
    Manual(HashMap<u32, T>),
}

impl<T> Thresholds<T> {
    fn threshold_for(&self, value: f64, max: f64) -> Option<&T> {
        match self {
            Thresholds::Basic { low, medium, high } => {
                let interval = max / 3.0;
                match value / interval {
                    0.0..1.0 => Some(low),
                    1.0..2.0 => Some(medium),
                    2.0..=3.0 => Some(high),
                    _ => unreachable!("interval should always be 0-3"),
                }
            }
            Thresholds::Dynamic(map) => {
                if value <= max {
                    // subtract a very small amount so that integers fall to prev bracket
                    // (ie to clamp to max)
                    let index = (value / max) * map.len() as f64 - 0.00001;
                    map.get(index.floor() as usize)
                } else {
                    map.last()
                }
            }
            Thresholds::Manual(map) => {
                let mut keys = map.keys().collect::<Vec<_>>();
                keys.sort();

                keys.into_iter()
                    .rfind(|k| **k <= value.floor() as u32)
                    .and_then(|key| map.get(key))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn basic() -> Thresholds<&'static str> {
        Thresholds::Basic {
            low: "low",
            medium: "medium",
            high: "high",
        }
    }

    fn dynamic() -> Thresholds<&'static str> {
        Thresholds::Dynamic(vec!["low", "medium", "high"])
    }

    fn manual() -> Thresholds<&'static str> {
        let mut map = HashMap::new();
        map.insert(0, "low");
        map.insert(33, "medium");
        map.insert(67, "high");

        Thresholds::Manual(map)
    }

    #[test]
    fn test_basic_zero() {
        let levels = basic();
        assert_eq!(levels.threshold_for(0.0, 100.0), Some(&"low"));
    }

    #[test]
    fn test_basic_low() {
        let levels = basic();
        assert_eq!(levels.threshold_for(25.0, 100.0), Some(&"low"));
    }

    #[test]
    fn test_basic_medium() {
        let levels = basic();
        assert_eq!(levels.threshold_for(50.0, 100.0), Some(&"medium"));
    }

    #[test]
    fn test_basic_high() {
        let levels = basic();
        assert_eq!(levels.threshold_for(75.0, 100.0), Some(&"high"));
    }

    #[test]
    fn test_basic_max() {
        let levels = basic();
        assert_eq!(levels.threshold_for(100.0, 100.0), Some(&"high"));
    }

    #[test]
    fn test_dynamic_zero() {
        let levels = dynamic();
        assert_eq!(levels.threshold_for(0.0, 100.0), Some(&"low"));
    }

    #[test]
    fn test_dynamic_low() {
        let levels = dynamic();
        assert_eq!(levels.threshold_for(25.0, 100.0), Some(&"low"));
    }

    #[test]
    fn test_dynamic_medium() {
        let levels = dynamic();
        assert_eq!(levels.threshold_for(50.0, 100.0), Some(&"medium"));
    }

    #[test]
    fn test_dynamic_high() {
        let levels = dynamic();
        assert_eq!(levels.threshold_for(75.0, 100.0), Some(&"high"));
    }

    #[test]
    fn test_dynamic_max() {
        let levels = dynamic();
        assert_eq!(levels.threshold_for(100.0, 100.0), Some(&"high"));
    }

    #[test]
    fn test_manual_zero() {
        let levels = manual();
        assert_eq!(levels.threshold_for(0.0, 100.0), Some(&"low"));
    }

    #[test]
    fn test_manual_low() {
        let levels = manual();
        assert_eq!(levels.threshold_for(25.0, 100.0), Some(&"low"));
    }

    #[test]
    fn test_manual_medium() {
        let levels = manual();
        assert_eq!(levels.threshold_for(50.0, 100.0), Some(&"medium"));
    }

    #[test]
    fn test_manual_high() {
        let levels = manual();
        assert_eq!(levels.threshold_for(75.0, 100.0), Some(&"high"));
    }

    #[test]
    fn test_manual_max() {
        let levels = manual();
        assert_eq!(levels.threshold_for(100.0, 100.0), Some(&"high"));
    }
}
