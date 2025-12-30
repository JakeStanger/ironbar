mod button;
mod button_map;
mod open_state;

use self::button::Button;
use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::compositor::{Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::config::{CommonConfig, LayoutConfig, default};
use crate::gtk_helpers::IronbarGtkExt;
use crate::modules::workspaces::button_map::{ButtonMap, Identifier};
use crate::modules::workspaces::open_state::OpenState;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{image, module_impl, spawn};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, trace, warn};

#[derive(Debug, Deserialize, Default, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
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

    /// Shows workspaces in the order of their index within the compositor.
    Index,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(untagged)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
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
#[serde(untagged)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
pub enum Format {
    String(String),
    Pair {
        #[serde(default = "default_format")]
        named: String,
        #[serde(default = "default_format")]
        unnamed: String,
    },
}

impl Default for Format {
    fn default() -> Self {
        Self::String(default_format())
    }
}

impl Format {
    pub fn resolve(&self) -> (String, String) {
        match self {
            Self::String(s) => (s.clone(), s.clone()),
            Self::Pair { named, unnamed } => (named.clone(), unnamed.clone()),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
#[cfg_attr(feature = "extras", derive(schemars::JsonSchema))]
#[serde(default)]
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
    icon_size: i32,

    /// The format string for named workspaces.
    ///
    /// The following placeholders are supported:
    /// - `{label}`: The display label (from `name_map` or the workspace name).
    /// - `{name}`: The actual workspace name.
    /// - `{index}`: The workspace index.
    ///
    /// **Default**: `"{label}"`
    #[serde(default)]
    format: Format,

    // -- Common --
    /// See [layout options](module-level-options#layout)
    #[serde(default, flatten)]
    layout: LayoutConfig,

    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

fn default_format() -> String {
    "{label}".to_string()
}

impl Default for WorkspacesModule {
    fn default() -> Self {
        Self {
            name_map: HashMap::default(),
            favorites: Favorites::default(),
            hidden: vec![],
            all_monitors: false,
            sort: SortOrder::default(),
            icon_size: default::IconSize::Normal as i32,
            format: Format::default(),
            layout: LayoutConfig::default(),
            common: Some(CommonConfig::default()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkspaceItemContext {
    name_map: HashMap<String, String>,
    icon_size: i32,
    image_provider: image::Provider,
    tx: mpsc::Sender<i64>,
    format_named: String,
    format_unnamed: String,
}

impl WorkspaceItemContext {
    pub fn format_label(&self, name: &str, index: i64) -> String {
        let label = self.name_map.get(name).map_or(name, String::as_str);

        let is_named = name != index.to_string();
        let format = if is_named {
            &self.format_named
        } else {
            &self.format_unnamed
        };

        format
            .replace("{label}", label)
            .replace("{name}", name)
            .replace("{index}", &index.to_string())
    }
}

/// Re-orders the container children alphabetically,
/// using their widget names.
///
/// Named workspaces are always sorted before numbered ones.
fn reorder_workspaces(container: &gtk::Box, sort_order: SortOrder) {
    let mut buttons: Vec<(String, Option<gtk::Widget>)> = container
        .children()
        .map(|child| {
            let label = match sort_order {
                SortOrder::Label => child
                    .downcast_ref::<gtk::Button>()
                    .and_then(ButtonExt::label)
                    .unwrap_or_else(|| child.widget_name())
                    .to_string(),
                SortOrder::Index => {
                    let index = child.get_tag::<i64>("workspace_index").copied();

                    index
                        .map(|v| v.to_string())
                        .unwrap_or_else(|| i64::MAX.to_string())
                }
                _ => child.widget_name().to_string(),
            };

            (label, Some(child))
        })
        .collect();

    buttons.sort_by(|(label_a, _), (label_b, _)| {
        match (label_a.parse::<i64>(), label_b.parse::<i64>()) {
            (Ok(a), Ok(b)) => a.cmp(&b),
            (Ok(_), Err(_)) => Ordering::Less,
            (Err(_), Ok(_)) => Ordering::Greater,
            (Err(_), Err(_)) => label_a.cmp(label_b),
        }
    });

    // Ensure we have an even number of elements for window size
    if buttons.len() % 2 == 1 {
        buttons.push((String::new(), None));
    }

    for window in buttons.windows(2) {
        if let [(_, a), (_, Some(b))] = window {
            container.reorder_child_after(b, a.as_ref());
        }
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
                tx.send_update(payload).await;
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
        let container = gtk::Box::new(self.layout.orientation(info), 0);

        let mut button_map = ButtonMap::new();

        let (format_named, format_unnamed) = self.format.resolve();

        let item_context = WorkspaceItemContext {
            name_map: self.name_map.clone(),
            icon_size: self.icon_size,
            image_provider: context.ironbar.image_provider(),
            tx: context.controller_tx.clone(),
            format_named,
            format_unnamed,
        };

        // setup favorites
        let favorites = match self.favorites {
            Favorites::ByMonitor(map) => map.get(info.output_name).cloned(),
            Favorites::Global(vec) => Some(vec),
        }
        .unwrap_or_default();

        for favorite in &favorites {
            let index = favorite.parse::<i64>().unwrap_or(0);
            let btn = Button::new(-1, index, favorite, OpenState::Closed, &item_context);

            btn.button().set_tag("workspace_index", index);
            container.append(btn.button());
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
                let item_context = item_context.clone();
                move |workspace: Workspace, button_map: &mut ButtonMap| {
                    if favorites.contains(&workspace.name) {
                        let btn = button_map
                            .get_mut(&Identifier::Name(workspace.name.clone()))
                            .expect("favorite to exist");

                        // set an ID to track the open workspace for the favourite
                        btn.set_workspace_id(workspace.id);
                        btn.set_open_state(workspace.visibility.into());
                        let label = item_context.format_label(&workspace.name, workspace.index);
                        btn.set_label(&label);

                        btn.button().set_tag("workspace_index", workspace.index);
                    } else if let Some(btn) = button_map.find_button_mut(&workspace) {
                        let label = item_context.format_label(&workspace.name, workspace.index);
                        btn.set_label(&label);
                        btn.button().set_tag("workspace_index", workspace.index);
                    } else {
                        let btn = Button::new(
                            workspace.id,
                            workspace.index,
                            &workspace.name,
                            workspace.visibility.into(),
                            &item_context,
                        );

                        btn.button().set_tag("workspace_index", workspace.index);

                        container.append(btn.button());
                        btn.button().set_visible(true);

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

            context
                .subscribe()
                .recv_glib((), move |(), event| match event {
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
                    WorkspaceUpdate::Add(workspace) if has_initialized => {
                        if !self.hidden.contains(&workspace.name)
                            && (self.all_monitors || workspace.monitor == output_name)
                        {
                            add_workspace(workspace, &mut button_map);
                        }

                        reorder!();
                    }
                    WorkspaceUpdate::Remove(id) => remove_workspace(id, &mut button_map),
                    WorkspaceUpdate::Move(workspace) if has_initialized => {
                        if self.all_monitors {
                            if !self.hidden.contains(&workspace.name) {
                                add_workspace(workspace, &mut button_map);
                                reorder!();
                            }
                            return;
                        }

                        if workspace.monitor == output_name
                            && !self.hidden.contains(&workspace.name)
                        {
                            add_workspace(workspace, &mut button_map);
                            reorder!();
                        } else {
                            remove_workspace(workspace.id, &mut button_map);
                        }
                    }
                    WorkspaceUpdate::Focus { old, new } if has_initialized => {
                        // Open states are calculated here rather than using the workspace visibility
                        // as that seems to come back wrong, at least on Hyprland.
                        // Likely a deeper issue that needs exploring.

                        if let Some(old) = old
                            && let Some(button) = button_map.find_button_mut(&old)
                        {
                            let open_state = if new.monitor == old.monitor {
                                OpenState::Hidden
                            } else {
                                OpenState::Visible
                            };

                            button.set_open_state(open_state);
                        }

                        if let Some(button) = button_map.find_button_mut(&new) {
                            button.set_open_state(OpenState::Focused);
                        }
                    }
                    WorkspaceUpdate::Rename { id, name } if has_initialized => {
                        let button = if let Some(button) = button_map.get_mut(&Identifier::Id(id)) {
                            Some(button)
                        } else {
                            button_map.get_mut(&Identifier::Name(name.clone()))
                        };

                        if let Some(button) = button {
                            let index = button
                                .button()
                                .get_tag::<i64>("workspace_index")
                                .copied()
                                .unwrap_or(0);

                            let display_name = item_context.format_label(&name, index);

                            button.set_label(&display_name);
                            button.button().set_widget_name(&name);
                        }
                    }
                    WorkspaceUpdate::Urgent { id, urgent } if has_initialized => {
                        if let Some(button) = button_map
                            .get(&Identifier::Id(id))
                            .or_else(|| button_map.find_button_by_id(id))
                        {
                            button.set_urgent(urgent);
                        }
                    }
                    WorkspaceUpdate::Unknown if has_initialized => {
                        warn!("received unknown type workspace event")
                    }
                    // Avoids race conditions where e.g. we process workspace moves fired _before_
                    // we could send the WorkspaceUpdate::Init() event, resulting in duplicate
                    // workspaces.
                    // https://github.com/JakeStanger/ironbar/issues/1196#issuecomment-3407036546
                    _ => warn!("ignoring workspace event received before initialization"),
                });
        }

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_deserialization() {
        // Test string format
        let json = r#""{label}""#;
        let format: Format =
            serde_json::from_str(json).expect("failed to deserialize string format");
        assert_eq!(
            format.resolve(),
            ("{label}".to_string(), "{label}".to_string())
        );

        // Test object format
        let json = r#"
        {
            "named": "{name}",
            "unnamed": "{index}"
        }
        "#;
        let format: Format =
            serde_json::from_str(json).expect("failed to deserialize object format");
        assert_eq!(
            format.resolve(),
            ("{name}".to_string(), "{index}".to_string())
        );

        // Test object format with defaults
        let json = r#"
        {
            "named": "{name}"
        }
        "#;
        let format: Format =
            serde_json::from_str(json).expect("failed to deserialize object format with defaults");
        assert_eq!(
            format.resolve(),
            ("{name}".to_string(), "{label}".to_string())
        );
    }
}
