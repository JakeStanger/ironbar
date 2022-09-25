use color_eyre::Result;
use swayipc_async::{Connection, Node, NodeType, ShellType};

pub fn get_node_id(node: &Node) -> &str {
    node.app_id.as_ref().map_or_else(
        || {
            node.window_properties
                .as_ref()
                .expect("Cannot find node window properties")
                .class
                .as_ref()
                .expect("Cannot find node name")
        },
        |app_id| app_id,
    )
}

/// Checks whether this application
/// is running under xwayland.
pub fn is_node_xwayland(node: &Node) -> bool {
    node.shell == Some(ShellType::Xwayland)
}

/// Recursively checks the provided node for any child application nodes.
/// Returns a list of any found application nodes.
fn check_node(node: Node, window_nodes: &mut Vec<Node>) {
    if node.name.is_some()
        && (node.node_type == NodeType::Con || node.node_type == NodeType::FloatingCon)
    {
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

/// Gets a flat vector of all currently open windows.
pub async fn get_open_windows(client: &mut Connection) -> Result<Vec<Node>> {
    let root_node = client.get_tree().await?;

    let mut window_nodes = vec![];
    check_node(root_node, &mut window_nodes);

    Ok(window_nodes)
}
