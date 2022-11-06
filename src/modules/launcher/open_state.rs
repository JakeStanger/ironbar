use crate::clients::wayland::ToplevelInfo;

/// Open state for a launcher item, or item window.
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum OpenState {
    Closed,
    Open { focused: bool },
}

impl OpenState {
    /// Creates from `SwayNode`
    pub const fn from_toplevel(toplevel: &ToplevelInfo) -> Self {
        Self::Open {
            focused: toplevel.active,
        }
    }

    /// Creates open with focused
    pub const fn focused(focused: bool) -> Self {
        Self::Open { focused }
    }

    /// Checks if open
    pub fn is_open(self) -> bool {
        self != Self::Closed
    }

    /// Checks if open with focus
    pub const fn is_focused(self) -> bool {
        matches!(self, Self::Open { focused: true })
    }

    /// Merges states together to produce a single state.
    /// This is effectively an OR operation,
    /// so sets state to open and flags to true if any state is open
    /// or any instance of the flag is true.
    pub fn merge_states(states: &[&Self]) -> Self {
        states.iter().fold(Self::Closed, |merged, current| {
            if merged.is_open() || current.is_open() {
                Self::Open {
                    focused: merged.is_focused() || current.is_focused(),
                }
            } else {
                Self::Closed
            }
        })
    }
}
