use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::compositor::{Visibility, Workspace, WorkspaceClient, WorkspaceUpdate};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::image::new_icon_button;
use crate::modules::{Module, ModuleInfo, ModuleParts, WidgetContext};
use crate::{module_impl, spawn, Ironbar};
use color_eyre::{Report, Result};
use gtk::prelude::*;
use gtk::{Button, IconTheme};
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{debug, trace, warn};

#[derive(Debug, Deserialize, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum SortOrder {
    /// Shows workspaces in the order they're added
    Added,
    /// Shows workspaces in numeric order.
    /// Named workspaces are added to the end in alphabetical order.
    Alphanumeric,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Alphanumeric
    }
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
    name_map: Option<HashMap<String, String>>,

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
    /// `added` always appends to the end, `alphanumeric` sorts by number/name.
    ///
    /// **Valid options**: `added`, `alphanumeric`
    /// <br>
    /// **Default**: `alphanumeric`
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

/// Creates a button from a workspace
fn create_button(
    name: &str,
    visibility: Visibility,
    name_map: &HashMap<String, String>,
    icon_theme: &IconTheme,
    icon_size: i32,
    tx: &Sender<String>,
) -> Button {
    let label = name_map.get(name).map_or(name, String::as_str);

    let button = new_icon_button(label, icon_theme, icon_size);
    button.set_widget_name(name);

    button.add_class("item");

    if visibility.is_visible() {
        button.add_class("visible");
    }

    if visibility.is_focused() {
        button.add_class("focused");
    }

    if !visibility.is_visible() {
        button.add_class("inactive");
    }

    {
        let tx = tx.clone();
        let name = name.to_string();
        button.connect_clicked(move |button| {
            if !button.style_context().has_class("focused") {
                tx.send_spawn(name.clone());
            }
        });
    }

    button
}

fn reorder_workspaces(container: &gtk::Box) {
    let mut buttons = container
        .children()
        .into_iter()
        .map(|child| {
            let label = child
                .downcast_ref::<Button>()
                .and_then(|button| button.label())
                .unwrap_or_else(|| child.widget_name())
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

fn find_btn(map: &HashMap<i64, Button>, workspace: &Workspace) -> Option<Button> {
    map.get(&workspace.id)
        .or_else(|| {
            map.values()
                .find(|&btn| btn.widget_name() == workspace.name)
        })
        .cloned()
}

impl WorkspacesModule {
    fn show_workspace_check(&self, output: &String, work: &Workspace) -> bool {
        (work.visibility.is_focused() || !self.hidden.contains(&work.name))
            && (self.all_monitors || output == &work.monitor)
    }
}

impl Module<gtk::Box> for WorkspacesModule {
    type SendMessage = WorkspaceUpdate;
    type ReceiveMessage = String;

    module_impl!("workspaces");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        mut rx: Receiver<Self::ReceiveMessage>,
    ) -> Result<()> {
        let tx = context.tx.clone();
        let client = context.ironbar.clients.borrow_mut().workspaces()?;
        // Subscribe & send events
        spawn(async move {
            let mut srx = client.subscribe_workspace_change();

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

            while let Some(name) = rx.recv().await {
                if let Err(e) = client.focus(name.clone()) {
                    warn!("Couldn't focus workspace '{name}': {e:#}");
                };
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

        let name_map = self.name_map.clone().unwrap_or_default();
        let favs = self.favorites.clone();
        let mut fav_names: Vec<String> = vec![];

        let mut button_map: HashMap<i64, Button> = HashMap::new();

        {
            let container = container.clone();
            let output_name = info.output_name.to_string();
            let icon_theme = info.icon_theme.clone();
            let icon_size = self.icon_size;

            // keep track of whether init event has fired previously
            // since it fires for every workspace subscriber
            let mut has_initialized = false;

            context.subscribe().recv_glib(move |event| {
                match event {
                    WorkspaceUpdate::Init(workspaces) => {
                        if !has_initialized {
                            trace!("Creating workspace buttons");

                            let mut added = HashSet::new();

                            let mut add_workspace =
                                |id: i64, name: &str, visibility: Visibility| {
                                    let item = create_button(
                                        name,
                                        visibility,
                                        &name_map,
                                        &icon_theme,
                                        icon_size,
                                        &context.controller_tx,
                                    );

                                    container.add(&item);
                                    button_map.insert(id, item);
                                };

                            // add workspaces from client
                            for workspace in &workspaces {
                                if self.show_workspace_check(&output_name, workspace) {
                                    add_workspace(
                                        workspace.id,
                                        &workspace.name,
                                        workspace.visibility,
                                    );
                                    added.insert(workspace.name.to_string());
                                }
                            }

                            let mut add_favourites = |names: &Vec<String>| {
                                for name in names {
                                    fav_names.push(name.to_string());

                                    if !added.contains(name) {
                                        // Favourites are added with the same name and ID
                                        // as Hyprland will initialize them this way.
                                        // Since existing workspaces are added above,
                                        // this means there shouldn't be any issues with renaming.
                                        add_workspace(
                                            -(Ironbar::unique_id() as i64),
                                            name,
                                            Visibility::Hidden,
                                        );
                                        added.insert(name.to_string());
                                    }
                                }
                            };

                            // add workspaces from favourites
                            match &favs {
                                Favorites::Global(names) => add_favourites(names),
                                Favorites::ByMonitor(map) => {
                                    if let Some(to_add) = map.get(&output_name) {
                                        add_favourites(to_add);
                                    }
                                }
                            }

                            if self.sort == SortOrder::Alphanumeric {
                                reorder_workspaces(&container);
                            }

                            container.show_all();
                            has_initialized = true;
                        }
                    }
                    WorkspaceUpdate::Focus { old, new } => {
                        if let Some(btn) = old.as_ref().and_then(|w| find_btn(&button_map, w)) {
                            if Some(new.monitor.as_str())
                                == old.as_ref().map(|w| w.monitor.as_str())
                            {
                                btn.style_context().remove_class("visible");
                            }

                            btn.style_context().remove_class("focused");
                        }

                        if let Some(btn) = find_btn(&button_map, &new) {
                            btn.add_class("visible");
                            btn.add_class("focused");
                        }
                    }
                    WorkspaceUpdate::Rename { id, name } => {
                        if let Some(btn) = button_map.get(&id) {
                            let name = name_map.get(&name).unwrap_or(&name);
                            btn.set_label(name);
                        }

                        if self.sort == SortOrder::Alphanumeric {
                            reorder_workspaces(&container);
                        }
                    }
                    WorkspaceUpdate::Add(workspace) => {
                        if fav_names.contains(&workspace.name) {
                            let btn = button_map.get(&workspace.id);
                            if let Some(btn) = btn {
                                btn.style_context().remove_class("inactive");
                            }
                        } else if self.show_workspace_check(&output_name, &workspace) {
                            let name = workspace.name;
                            let item = create_button(
                                &name,
                                workspace.visibility,
                                &name_map,
                                &icon_theme,
                                icon_size,
                                &context.controller_tx,
                            );

                            container.add(&item);
                            if self.sort == SortOrder::Alphanumeric {
                                reorder_workspaces(&container);
                            }

                            item.show();

                            if !name.is_empty() {
                                button_map.insert(workspace.id, item);
                            }
                        }
                    }
                    WorkspaceUpdate::Move(workspace) => {
                        if !self.hidden.contains(&workspace.name) && !self.all_monitors {
                            if workspace.monitor == output_name {
                                let name = workspace.name;
                                let item = create_button(
                                    &name,
                                    workspace.visibility,
                                    &name_map,
                                    &icon_theme,
                                    icon_size,
                                    &context.controller_tx,
                                );

                                container.add(&item);

                                if self.sort == SortOrder::Alphanumeric {
                                    reorder_workspaces(&container);
                                }

                                item.show();

                                if !name.is_empty() {
                                    button_map.insert(workspace.id, item);
                                }
                            } else if let Some(item) = button_map.get(&workspace.id) {
                                container.remove(item);
                            }
                        }
                    }
                    WorkspaceUpdate::Remove(workspace) => {
                        let button = button_map.get(&workspace);
                        if let Some(item) = button {
                            if workspace < 0 {
                                // if fav_names.contains(&workspace) {
                                item.style_context().add_class("inactive");
                            } else {
                                container.remove(item);
                            }
                        }
                    }
                    WorkspaceUpdate::Urgent { id, urgent } => {
                        let button = button_map.get(&id);
                        if let Some(item) = button {
                            if urgent {
                                item.add_class("urgent");
                            } else {
                                item.style_context().remove_class("urgent");
                            }
                        }
                    }
                    WorkspaceUpdate::Unknown => warn!("Received unknown type workspace event"),
                };
            });
        }

        Ok(ModuleParts {
            widget: container,
            popup: None,
        })
    }
}
