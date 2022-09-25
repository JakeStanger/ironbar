use swayipc_async::Node;

/// Open state for a launcher item, or item window.
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum OpenState {
    Closed,
    Open { focused: bool, urgent: bool },
}

impl OpenState {
    /// Creates from `SwayNode`
    pub const fn from_node(node: &Node) -> Self {
        Self::Open {
            focused: node.focused,
            urgent: node.urgent,
        }
    }

    /// Creates open with focused
    pub const fn focused(focused: bool) -> Self {
        Self::Open {
            focused,
            urgent: false,
        }
    }

    /// Creates open with urgent
    pub const fn urgent(urgent: bool) -> Self {
        Self::Open {
            focused: false,
            urgent,
        }
    }

    /// Checks if open
    pub fn is_open(self) -> bool {
        self != Self::Closed
    }

    /// Checks if open with focus
    pub const fn is_focused(self) -> bool {
        matches!(self, Self::Open { focused: true, .. })
    }

    /// check if open with urgent
    pub const fn is_urgent(self) -> bool {
        matches!(self, Self::Open { urgent: true, .. })
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
                    urgent: merged.is_urgent() || current.is_urgent(),
                }
            } else {
                Self::Closed
            }
        })
    }
}
