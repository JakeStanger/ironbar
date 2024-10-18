use super::button::Button;
use crate::clients::compositor::Workspace;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Identifier {
    Id(i64),
    Name(String),
}

/// Wrapper around a hashmap of workspace buttons,
/// which can be found using the workspace ID,
/// or their name for favourites.
pub struct ButtonMap {
    map: HashMap<Identifier, Button>,
}

impl ButtonMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Gets the button for a workspace,
    /// checking the map for both its ID and name.
    pub fn find_button_mut(&mut self, workspace: &Workspace) -> Option<&mut Button> {
        let id = Identifier::Id(workspace.id);

        if self.map.contains_key(&id) {
            self.map.get_mut(&id)
        } else {
            self.map.get_mut(&Identifier::Name(workspace.name.clone()))
        }
    }

    /// Gets the button for a workspace,
    /// performing a search of all keys for the button
    /// with the associated workspace ID.
    pub fn find_button_by_id_mut(&mut self, id: i64) -> Option<&mut Button> {
        self.map.iter_mut().find_map(|(_, button)| {
            if button.workspace_id() == id {
                Some(button)
            } else {
                None
            }
        })
    }
}

impl Deref for ButtonMap {
    type Target = HashMap<Identifier, Button>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl DerefMut for ButtonMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}
