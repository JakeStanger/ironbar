use crate::clients::compositor::Visibility;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OpenState {
    /// A favourite workspace, which is not currently open
    Closed,
    /// A workspace which is open but not visible on any monitors.
    Hidden,
    /// A workspace which is visible, but not focused.
    Visible,
    /// The currently active workspace.
    Focused,
}

impl From<Visibility> for OpenState {
    fn from(value: Visibility) -> Self {
        match value {
            Visibility::Visible { focused: true } => Self::Focused,
            Visibility::Visible { focused: false } => Self::Visible,
            Visibility::Hidden => Self::Hidden,
        }
    }
}

impl OpenState {
    /// Whether the workspace is visible, including focused state.
    pub fn is_visible(self) -> bool {
        matches!(self, OpenState::Visible | OpenState::Focused)
    }
}
