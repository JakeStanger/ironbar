use crate::clients::wayland::ToplevelInfo;

/// Open state for a launcher item, or item window.
#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum OpenState {
    Closed,
    Open { focused: bool },
}

impl From<&ToplevelInfo> for OpenState {
    fn from(info: &ToplevelInfo) -> Self {
        Self::Open {
            focused: info.focused,
        }
    }
}

impl OpenState {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_states_empty() {
        let states: Vec<&OpenState> = vec![];
        assert_eq!(OpenState::merge_states(&states), OpenState::Closed);
    }

    #[test]
    fn test_merge_states_single_closed() {
        let states = vec![&OpenState::Closed];
        assert_eq!(OpenState::merge_states(&states), OpenState::Closed);
    }

    #[test]
    fn test_merge_states_single_open_unfocused() {
        let states = vec![&OpenState::Open { focused: false }];
        assert_eq!(
            OpenState::merge_states(&states),
            OpenState::Open { focused: false }
        );
    }

    #[test]
    fn test_merge_states_single_open_focused() {
        let states = vec![&OpenState::Open { focused: true }];
        assert_eq!(
            OpenState::merge_states(&states),
            OpenState::Open { focused: true }
        );
    }

    #[test]
    fn test_merge_states_unfocused_and_focused() {
        let states = vec![
            &OpenState::Open { focused: false },
            &OpenState::Open { focused: true },
        ];
        assert_eq!(
            OpenState::merge_states(&states),
            OpenState::Open { focused: true }
        );
    }
}
