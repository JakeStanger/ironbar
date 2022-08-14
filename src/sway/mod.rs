use serde::Deserialize;

pub mod node;

#[derive(Deserialize, Debug)]
pub struct WorkspaceEvent {
    pub change: String,
    pub old: Option<Workspace>,
    pub current: Option<Workspace>,
}

#[derive(Deserialize, Debug)]
pub struct Workspace {
    pub name: String,
    pub focused: bool,
    // pub num: i32,
    pub output: String,
}

#[derive(Debug, Deserialize)]
pub struct WindowEvent {
    pub change: String,
    pub container: SwayNode,
}

#[derive(Debug, Deserialize)]
pub struct SwayNode {
    #[serde(rename = "type")]
    pub node_type: String,
    pub id: i32,
    pub name: Option<String>,
    pub app_id: Option<String>,
    pub focused: bool,
    pub urgent: bool,
    pub nodes: Vec<SwayNode>,
    pub floating_nodes: Vec<SwayNode>,
    pub shell: Option<String>,
    pub window_properties: Option<WindowProperties>,
}

#[derive(Debug, Deserialize)]
pub struct WindowProperties {
    pub class: String,
}

#[derive(Deserialize)]
pub struct SwayOutput {
    pub name: String,
}
