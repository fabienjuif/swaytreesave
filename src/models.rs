use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::consts::*;

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
    pub fn from_sway(node_type: swayipc::NodeType) -> Self {
        match node_type {
            swayipc::NodeType::Root => NodeType::Root,
            swayipc::NodeType::Output => NodeType::Output,
            swayipc::NodeType::Workspace => NodeType::Workspace,
            swayipc::NodeType::Con => NodeType::Con,
            swayipc::NodeType::FloatingCon => NodeType::FloatingCon,
            swayipc::NodeType::Dockarea => NodeType::Dockarea,
            _ => NodeType::Unknown,
        }
    }

    fn is_window(&self) -> bool {
        matches!(self, NodeType::Con | NodeType::FloatingCon)
    }
}

impl NodeLayout {
    pub fn from_sway(node_layout: swayipc::NodeLayout) -> Self {
        match node_layout {
            swayipc::NodeLayout::SplitH => NodeLayout::SplitH,
            swayipc::NodeLayout::SplitV => NodeLayout::SplitV,
            swayipc::NodeLayout::Stacked => NodeLayout::Stacked,
            swayipc::NodeLayout::Tabbed => NodeLayout::Tabbed,
            swayipc::NodeLayout::Output => NodeLayout::Output,
            swayipc::NodeLayout::Dockarea => NodeLayout::Dockarea,
            _ => NodeLayout::Unknown,
        }
    }

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
    pub desktop_file: Option<String>,
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
