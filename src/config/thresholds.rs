use serde::Deserialize;
use std::collections::HashMap;
use std::ops::Deref;

/// A representation of numeric thresholds
/// mapped to values of type `T`.
///
/// This is most useful for icon configuration,
/// allowing users to define the icons that should appear
/// as a value passes various thresholds.
/// For example, showing low/medium/high volume icons as volume changes.
///
/// Any number of thresholds can be defined.
///
/// Internally this is a `HashMap<i32, T>`
/// and can be dereferenced as such.
///
/// # Example
///
/// ```corn
/// {
///     thresholds.0 = "icon:volume-low"
///     thresholds.33 = "icon:battery-medium"
///     thresholds.67 = "icon:battery-high"
/// }
/// ```
#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct Thresholds<T>(HashMap<i32, T>);

impl<T> Thresholds<T> {
    /// Creates a new `Thresholds` instance with low/medium/high values
    /// split into equal thirds `(0, 33, 67)`.
    pub fn new_low_med_high(low: T, med: T, high: T) -> Self {
        let mut map = HashMap::new();

        map.insert(0, low);
        map.insert(33, med);
        map.insert(67, high);

        Self(map)
    }

    /// Gets the defined threshold value for the given numeric value.
    ///
    /// This is a 'floor' operation,
    /// meaning the nearest threshold *below* the provided value is used.
    pub fn for_value(&self, value: i32) -> Option<&T> {
        let mut keys = self.keys().collect::<Vec<_>>();
        keys.sort();

        keys.into_iter()
            .rfind(|k| **k <= value)
            .and_then(|key| self.get(key))
    }
}

impl<T> Deref for Thresholds<T> {
    type Target = HashMap<i32, T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Deserialize)]
    #[allow(unused)]
    struct Test {
        foo: Thresholds<String>,
    }

    fn create() -> Thresholds<&'static str> {
        let mut map = HashMap::new();
        map.insert(0, "low");
        map.insert(33, "medium");
        map.insert(67, "high");

        Thresholds(map)
    }

    #[test]
    fn zero() {
        let levels = create();
        assert_eq!(levels.for_value(0), Some(&"low"));
    }

    #[test]
    fn low() {
        let levels = create();
        assert_eq!(levels.for_value(25), Some(&"low"));
    }

    #[test]
    fn medium() {
        let levels = create();
        assert_eq!(levels.for_value(50), Some(&"medium"));
    }

    #[test]
    fn high() {
        let levels = create();
        assert_eq!(levels.for_value(75), Some(&"high"));
    }

    #[test]
    fn max() {
        let levels = create();
        assert_eq!(levels.for_value(100), Some(&"high"));
    }

    #[test]
    fn deserialize_json() {
        let json = "{\"foo\": {\"0\": \"low\", \"33\": \"medium\", \"67\": \"high\"}}";

        let test = serde_json::from_str::<Test>(json);
        assert!(test.is_ok());
    }

    #[test]
    fn deserialize_yaml() {
        let yaml = "
        foo:
          0: low
          33: medium
          67: high
        ";

        let test = serde_norway::from_str::<Test>(yaml);
        assert!(test.is_ok());
    }

    #[test]
    fn deserialize_toml() {
        let toml = "
        [foo]
        0 = \"low\"
        33 = \"medium\"
        67 = \"high\"\
        ";

        let test = toml::from_str::<Test>(toml);
        assert!(test.is_ok());
    }

    #[test]
    fn deserialize_corn() {
        let corn = "{ foo = { 0 = \"low\" 33 = \"medium\" 67 = \"high\" } }";

        let test = corn::from_str::<Test>(corn);
        assert!(test.is_ok());
    }
}
