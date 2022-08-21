use crate::sway::{SwayClient, SwayNode};
use color_eyre::Result;
use ksway::IpcCommand;

impl SwayNode {
    pub fn get_id(&self) -> &str {
        self.app_id.as_ref().map_or_else(
            || {
                self.window_properties
                    .as_ref()
                    .expect("Cannot find node window properties")
                    .class
                    .as_ref()
                    .expect("Cannot find node name")
            },
            |app_id| app_id,
        )
    }

    pub fn is_xwayland(&self) -> bool {
        self.shell == Some(String::from("xwayland"))
    }
}

fn check_node(node: SwayNode, window_nodes: &mut Vec<SwayNode>) {
    if node.name.is_some() && (node.node_type == "con" || node.node_type == "floating_con") {
        window_nodes.push(node);
    } else {
        node.nodes.into_iter().for_each(|node| {
            check_node(node, window_nodes);
        });

        node.floating_nodes.into_iter().for_each(|node| {
            check_node(node, window_nodes);
        });
    }
}

impl SwayClient {
    pub fn get_open_windows(&mut self) -> Result<Vec<SwayNode>> {
        let root_node = self.ipc(IpcCommand::GetTree)?;
        let root_node = serde_json::from_slice(&root_node)?;

        let mut window_nodes = vec![];
        check_node(root_node, &mut window_nodes);

        Ok(window_nodes)
    }
}
