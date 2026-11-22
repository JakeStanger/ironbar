use crate::config::{ModuleJustification, ModuleOrientation};
use crate::modules::ModuleInfo;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "extras", derive(serde::Serialize, schemars::JsonSchema))]
pub struct LayoutConfig {
    /// The orientation to display the widget contents.
    orientation: Option<ModuleOrientation>,

    /// The justification (alignment) of the widget text shown on the bar.
    #[serde(default)]
    pub justify: ModuleJustification,
}

impl LayoutConfig {
    pub fn orientation(&self, info: &ModuleInfo) -> gtk::Orientation {
        self.orientation
            .map_or(info.bar_position.orientation(), ModuleOrientation::into)
    }
}
