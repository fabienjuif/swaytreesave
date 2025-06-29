use std::vec;

// TODO: I do not think Niri expose the columns, so this is not possible to see how windows are arranged
// TODO: same for columns widths then
use anyhow::{Context, Result, anyhow, bail};
use tracing::debug;

use crate::models::{Node, NodeLayout, NodeType};

pub struct Niri {
    socket: niri_ipc::socket::Socket,
}

impl Niri {
    pub fn new() -> Result<Self> {
        let socket = niri_ipc::socket::Socket::connect().context("on Socket::connect()")?;
        Ok(Self { socket })
    }

    pub fn get_tree(&mut self) -> Result<Vec<Node>> {
        // get workspaces
        let reply = self
            .socket
            .send(niri_ipc::Request::Workspaces)
            .context("on socket.send()")?
            .map_err(|e: String| anyhow!("on decoding Niri answer: {:?}", e))?;

        let niri_ipc::Response::Workspaces(workspaces) = reply else {
            return Err(anyhow!("Unexpected response type from Niri"));
        };
        let mut workspaces = workspaces;
        workspaces.sort_by_key(|w| w.idx);

        let mut nodes = Vec::new();
        for (idx, workspace) in workspaces.into_iter().enumerate() {
            debug!("workspace: {workspace:?}");
            let node = Node {
                name: Some(workspace.name.unwrap_or(idx.to_string())),
                node_type: crate::models::NodeType::Workspace,
                // niri does not provide layout, so we default to SplitH
                nodes: vec![Node {
                    node_type: NodeType::Con,
                    layout: NodeLayout::SplitH,
                    ..Default::default()
                }],
                ..Default::default()
            };
            nodes.push(node);
        }

        // get windows and map them to workspaces
        let reply = self
            .socket
            .send(niri_ipc::Request::Windows)
            .context("on socket.send()")?
            .map_err(|e| anyhow!("on decoding Niri answer: {:?}", e))?;

        let niri_ipc::Response::Windows(windows) = reply else {
            return Err(anyhow!("unexpected response type from Niri"));
        };

        for window in windows {
            debug!("window: {window:?}");
            let Some(workspace_id) = window.workspace_id else {
                continue;
            };
            let workspace_idx = (workspace_id - 1) as usize;

            if workspace_idx >= nodes.len() {
                bail!(
                    "workspace index {} out of bounds for nodes length {}",
                    workspace_idx,
                    nodes.len()
                );
            }

            let node = Node {
                node_type: NodeType::Con,
                app_id: window.app_id,
                ..Default::default()
            };

            // niri does not provide layout, so we default to the first found Con(tainer)
            if nodes[workspace_idx].nodes.is_empty() {
                bail!(
                    "workspace {} has no nodes, cannot determine layout",
                    workspace_idx
                );
            }
            nodes[workspace_idx].nodes[0].nodes.push(node);
        }

        Ok(nodes)
    }
}
