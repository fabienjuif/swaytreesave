use std::{fmt::Display, fs, path::Path, str::FromStr, time::Duration};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml::to_string;

use crate::consts::*;

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub enum Compositor {
    Sway, // or i3
    Niri,
}

impl Display for Compositor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Compositor::Sway => write!(f, "sway"),
            Compositor::Niri => write!(f, "niri"),
        }
    }
}

impl FromStr for Compositor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "sway" | "i3" => Ok(Compositor::Sway),
            "niri" => Ok(Compositor::Niri),
            _ => Err(anyhow::anyhow!("Unknown compositor: {s}")),
        }
    }
}

// TODO: use this in sway side
pub fn save_tree(tree_path: &Path, tree: &Vec<Node>) -> Result<()> {
    let serialized_yaml = to_string(&tree).context("on to_string()")?;
    fs::write(tree_path, serialized_yaml)
        .context(format!("on fs::write({})", tree_path.display()))?;

    Ok(())
}

// TODO: use this in sway side
pub fn load_tree(tree_path: &Path) -> Result<Vec<Node>> {
    let file_content = fs::read_to_string(tree_path).context("on fs::read_to_string()")?;
    let tree: Vec<Node> =
        serde_yaml::from_str(&file_content).context("on serde_yaml::from_str()")?;
    Ok(tree)
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum NodeType {
    Root,
    Output,
    Workspace,
    Con,
    FloatingCon,
    Dockarea, // i3-specific
    #[default]
    Unknown = 1000,
}

impl Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Root => write!(f, "root"),
            NodeType::Output => write!(f, "output"),
            NodeType::Workspace => write!(f, "workspace"),
            NodeType::Con => write!(f, "con"),
            NodeType::FloatingCon => write!(f, "floating_con"),
            NodeType::Dockarea => write!(f, "dockarea"),
            NodeType::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum NodeLayout {
    SplitH,
    SplitV,
    Stacked,
    Tabbed,
    Output,
    Dockarea, // i3-specific
    None,
    #[default]
    Unknown = 1000,
}

impl NodeType {
    fn is_window(&self) -> bool {
        matches!(self, NodeType::Con | NodeType::FloatingCon)
    }
}

impl NodeLayout {
    fn is_none(&self) -> bool {
        matches!(self, NodeLayout::Unknown | NodeLayout::None)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct Node {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "NodeType::is_window", default)]
    pub node_type: NodeType,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub nodes: Vec<Node>,
    #[serde(skip_serializing_if = "none_or_zero_u8")]
    pub fullscreen_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desktop_entry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<String>,
    #[serde(skip_serializing_if = "NodeLayout::is_none", default)]
    pub layout: NodeLayout,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<u8>,
    #[serde(
        with = "humantime_serde",
        skip_serializing_if = "Option::is_none",
        default = "default_timeout"
    )]
    pub timeout: Option<Duration>,
}

fn none_or_zero_u8(opt: &Option<u8>) -> bool {
    matches!(opt, None | Some(0))
}
