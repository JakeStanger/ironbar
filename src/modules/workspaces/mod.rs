mod button;
mod button_map;
mod open_state;

use self::button::Button;
use crate::clients::compositor::{Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::config::CommonConfig;
use crate::modules::workspaces::button_map::{ButtonMap, Identifier};
use crate::modules::workspaces::open_state::OpenState;
use crate::modules::{Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, WidgetContext};
use crate::{glib_recv, module_impl, send_async, spawn};
use color_eyre::{Report, Result};
use gtk::IconTheme;
use gtk::prelude::*;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

#[derive(Debug, Deserialize, Default, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SortOrder {
    /// Shows workspaces in the order they're added
    Added,

    /// Shows workspaces in the order of their displayed labels,
    /// accounting for any mappings supplied in `name_map`.
    /// In most cases, this is likely their number.
    ///
    /// Workspaces are sorted numerically first,
    /// and named workspaces are added to the end in alphabetical order.
    #[default]
    Label,

    /// Shows workspaces in the order of their real names,
    /// as supplied by the compositor.
    /// In most cases, this is likely their number.
    ///
    /// Workspaces are sorted numerically first,
    /// and named workspaces are added to the end in alphabetical order.
    Name,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Favorites {
    ByMonitor(HashMap<String, Vec<String>>),
    Global(Vec<String>),
}

impl Default for Favorites {
    fn default() -> Self {
        Self::Global(vec![])
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct WorkspacesModule {
    /// Map of actual workspace names to custom names.
    ///
    /// Custom names can be [images](images).
    ///
    /// If a workspace is not present in the map,
    /// it will fall back to using its actual name.
    #[serde(default)]
    name_map: HashMap<String, String>,

    /// Workspaces which should always be shown.
    /// This can either be an array of workspace names,
    /// or a map of monitor names to arrays of workspace names.
    ///
    /// **Default**: `{}`
    ///
    /// # Example
    ///
    /// ```corn
    /// // array format
    /// {
    ///   type = "workspaces"
    ///   favorites = ["1", "2", "3"]
    /// }
    ///
    /// // map format
    /// {
    ///   type = "workspaces"
    ///   favorites.DP-1 = ["1", "2", "3"]
    ///   favorites.DP-2 = ["4", "5", "6"]
    /// }
    /// ```
    #[serde(default)]
    favorites: Favorites,

    /// A list of workspace names to never show.
    ///
    /// This may be useful for scratchpad/special workspaces, for example.
    ///
    /// **Default**: `[]`
    #[serde(default)]
    hidden: Vec<String>,

    /// Whether to display workspaces from all monitors.
    /// When false, only shows workspaces on the current monitor.
    ///
    /// **Default**: `false`
    #[serde(default = "crate::config::default_false")]
    all_monitors: bool,

    /// The method used for sorting workspaces.
    ///
    /// - `added` always appends to the end.
    /// - `label` sorts by displayed value.
    /// - `name` sorts by workspace name.
    ///
    /// **Valid options**: `added`, `label`, `name`.
    /// <br>
    /// **Default**: `label`
    #[serde(default)]
    sort: SortOrder,

    /// The size to render icons at (image icons only).
    ///
    /// **Default**: `32`
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

const fn default_icon_size() -> i32 {
    32
}

#[derive(Debug, Clone)]
pub struct WorkspaceItemContext {
    name_map: HashMap<String, String>,
    icon_theme: IconTheme,
    icon_size: i32,
    tx: mpsc::Sender<i64>,
}

/// Re-orders the container children alphabetically,
/// using their widget names.
///
/// Named workspaces are always sorted before numbered ones.
fn reorder_workspaces(container: &gtk::Box, sort_order: SortOrder) {
    let mut buttons = container
        .children()
        .into_iter()
        .map(|child| {
            let label = if sort_order == SortOrder::Label {
                child
                    .downcast_ref::<gtk::Button>()
                    .and_then(ButtonExt::label)
                    .unwrap_or_else(|| child.widget_name())
            } else {
                child.widget_name()
            }
            .to_string();

            (label, child)
        })
        .collect::<Vec<_>>();

    buttons.sort_by(|(label_a, _), (label_b, _a)| {
        match (label_a.parse::<i32>(), label_b.parse::<i32>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => label_a.cmp(label_b),
        }
    });

    for (i, (_, button)) in buttons.into_iter().enumerate() {
        container.reorder_child(&button, i as i32);
    }
}

impl Module<gtk::Box> for WorkspacesModule {
    type SendMessage = WorkspaceUpdate;
    type ReceiveMessage = i64;

    module_impl!("workspaces");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
        let client = context.ironbar.clients.borrow_mut().workspaces()?;
        // Subscribe & send events
        spawn(async move {
            let mut srx = client.subscribe();

            trace!("Set up workspace subscription");

            while let Ok(payload) = srx.recv().await {
                debug!("Received update: {payload:?}");
                send_async!(tx, ModuleUpdateEvent::Update(payload));
            }
        });

        let client = context.try_client::<dyn WorkspaceClient>()?;

        // Change workspace focus
        spawn(async move {
            trace!("Setting up UI event handler");

            while let Some(id) = rx.recv().await {
                client.focus(id);
            }

            Ok::<(), Report>(())
        });

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        info: &ModuleInfo,
    ) -> Result<ModuleParts<gtk::Box>> {
        let container = gtk::Box::new(info.bar_position.orientation(), 0);

        let mut button_map = ButtonMap::new();

        let item_context = WorkspaceItemContext {
            name_map: self.name_map.clone(),
            icon_theme: info.icon_theme.clone(),
            icon_size: self.icon_size,
            tx: context.controller_tx.clone(),
        };

        // setup favorites
        let favorites = match self.favorites {
            Favorites::ByMonitor(map) => map.get(info.output_name).cloned(),
            Favorites::Global(vec) => Some(vec),
        }
        .unwrap_or_default();

        for favorite in &favorites {
            let btn = Button::new(-1, favorite, OpenState::Closed, &item_context);
            container.add(btn.button());
            button_map.insert(Identifier::Name(favorite.clone()), btn);
        }

        {
            let container = container.clone();
            let output_name = info.output_name.to_string();

            // keep track of whether init event has fired previously
            // since it fires for every workspace subscriber
            let mut has_initialized = false;

            let add_workspace = {
                let container = container.clone();
                move |workspace: Workspace, button_map: &mut ButtonMap| {
                    if favorites.contains(&workspace.name) {
                        let btn = button_map
                            .get_mut(&Identifier::Name(workspace.name))
                            .expect("favorite to exist");

                        // set an ID to track the open workspace for the favourite
                        btn.set_workspace_id(workspace.id);
                        btn.set_open_state(workspace.visibility.into());
                    } else {
                        let btn = Button::new(
                            workspace.id,
                            &workspace.name,
                            workspace.visibility.into(),
                            &item_context,
                        );
                        container.add(btn.button());
                        btn.button().show();

                        button_map.insert(Identifier::Id(workspace.id), btn);
                    }
                }
            };

            let remove_workspace = {
                let container = container.clone();
                move |id: i64, button_map: &mut ButtonMap| {
                    // since favourites use name identifiers,
                    // we can safely remove using ID here and favourites will remain
                    if let Some(button) = button_map.remove(&Identifier::Id(id)) {
                        container.remove(button.button());
                    } else {
                        // otherwise we do a deep search and use the button's cached ID
                        if let Some(button) = button_map.find_button_by_id_mut(id) {
                            button.set_workspace_id(-1);
                            button.set_open_state(OpenState::Closed);
                        }
                    }
                }
            };

            macro_rules! reorder {
                () => {
                    if self.sort != SortOrder::Added {
                        reorder_workspaces(&container, self.sort);
                    }
                };
            }

            let name_map = self.name_map;
            let mut handle_event = move |event: WorkspaceUpdate| match event {
                WorkspaceUpdate::Init(workspaces) => {
                    if has_initialized {
                        return;
                    }

                    trace!("Creating workspace buttons");

                    for workspace in workspaces
                        .into_iter()
                        .filter(|w| self.all_monitors || w.monitor == output_name)
                        .filter(|w| !self.hidden.contains(&w.name))
                    {
                        add_workspace(workspace, &mut button_map);
                    }

                    reorder!();

                    has_initialized = true;
                }
                WorkspaceUpdate::Add(workspace) => {
                    if !self.hidden.contains(&workspace.name)
                        && (self.all_monitors || workspace.monitor == output_name)
                    {
                        add_workspace(workspace, &mut button_map);
                    }

                    reorder!();
                }
                WorkspaceUpdate::Remove(id) => remove_workspace(id, &mut button_map),
                WorkspaceUpdate::Move(workspace) => {
                    if self.all_monitors {
                        return;
                    }

                    if workspace.monitor == output_name && !self.hidden.contains(&workspace.name) {
                        add_workspace(workspace, &mut button_map);
                        reorder!();
                    } else {
                        remove_workspace(workspace.id, &mut button_map);
                    }
                }
                WorkspaceUpdate::Focus { old, new } => {
                    // Open states are calculated here rather than using the workspace visibility
                    // as that seems to come back wrong, at least on Hyprland.
                    // Likely a deeper issue that needs exploring.

                    if let Some(old) = old {
                        if let Some(button) = button_map.find_button_mut(&old) {
                            let open_state = if new.monitor == old.monitor {
                                OpenState::Hidden
                            } else {
                                OpenState::Visible
                            };

                            button.set_open_state(open_state);
                        }
                    }

                    if let Some(button) = button_map.find_button_mut(&new) {
                        button.set_open_state(OpenState::Focused);
                    }
                }
                WorkspaceUpdate::Rename { id, name } => {
                    if let Some(button) = button_map
                        .get(&Identifier::Id(id))
                        .or_else(|| button_map.get(&Identifier::Name(name.clone())))
                        .map(Button::button)
                    {
                        let display_name = name_map.get(&name).unwrap_or(&name);

                        button.set_label(display_name);
                        button.set_widget_name(&name);
                    }
                }
                WorkspaceUpdate::Urgent { id, urgent } => {
                    if let Some(button) = button_map
                        .get(&Identifier::Id(id))
                        .or_else(|| button_map.find_button_by_id(id))
                    {
                        button.set_urgent(urgent);
                    }
                }
                WorkspaceUpdate::Unknown => warn!("received unknown type workspace event"),
            };

            glib_recv!(context.subscribe(), handle_event);
        }

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}
