use crate::config::{ModuleJustification, ModuleOrientation};
use crate::modules::ModuleInfo;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Default)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub struct LayoutConfig {
    /// The orientation to display the widget contents.
    /// Setting to vertical will rotate text 90 degrees.
    ///
    /// **Valid options**: `horizontal`, `vertical`
    /// <br>
    /// **Default**: `horizontal`
    orientation: Option<ModuleOrientation>,

    /// The justification (alignment) of the widget text shown on the bar.
    ///
    /// **Valid options**: `left`, `right`, `center`, `fill`
    /// <br>
    /// **Default**: `left`
    #[serde(default)]
    pub justify: ModuleJustification,
}

impl LayoutConfig {
    pub fn orientation(&self, info: &ModuleInfo) -> gtk::Orientation {
        self.orientation
            .map_or(info.bar_position.orientation(), ModuleOrientation::into)
    }
}
