use std::vec;

// TODO: I do not think Niri expose the columns, so this is not possible to see how windows are arranged
// TODO: same for columns widths then
use anyhow::{Context, Result, anyhow, bail};
use tracing::{debug, warn};

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

    /// Clears all current workspaces and closes all current windows.
    pub fn clear(&mut self) -> Result<()> {
        warn!("clearing all workspaces and windows -not supported yet-");
        Ok(())
    }

    pub fn load_tree(&mut self, tree: &[Node]) -> Result<()> {
        for (idx, node) in tree.iter().enumerate() {
            if !matches!(node.node_type, NodeType::Root | NodeType::Workspace) {
                warn!(
                    "skipping: uncompatible node at idx={idx}, looking for Workspace: {}",
                    node.node_type
                );
                continue;
            }

            let ref_workspace = niri_ipc::WorkspaceReferenceArg::Index((idx + 1) as u8);

            // name the workspace if it has a name
            if let Some(name) = &node.name {
                debug!("setting workspace name: {name}");
                let _ = self.send(niri_ipc::Request::Action(
                    niri_ipc::Action::SetWorkspaceName {
                        name: name.to_string(),
                        workspace: Some(ref_workspace.clone()),
                    },
                ))?;
            }

            // move current view to the workspace
            debug!("focusing workspace: {:?}", ref_workspace);
            let _ = self.send(niri_ipc::Request::Action(
                niri_ipc::Action::FocusWorkspace {
                    reference: ref_workspace,
                },
            ))?;

            // now it should be split containers, informations we do not have in Niri yet
            for (idx, node) in node.nodes.iter().enumerate() {
                if !matches!(
                    node.node_type,
                    NodeType::Con | NodeType::FloatingCon | NodeType::Unknown
                ) {
                    warn!(
                        "skipping: uncompatible node at idx={idx}, looking for Con(tainer): {}",
                        node.node_type
                    );
                    continue;
                }

                // spawn the applications (windows)
                for (idx, node) in node.nodes.iter().enumerate() {
                    if !matches!(
                        node.node_type,
                        NodeType::Con | NodeType::FloatingCon | NodeType::Unknown
                    ) {
                        warn!(
                            "skipping: uncompatible node at idx={idx}, looking for Con(tainer): {}",
                            node.node_type
                        );
                        continue;
                    }

                    debug!("spawning window: {:?}", node.app_id);
                    warn!("we do not support spawning windows in Niri for now");
                }
            }

            // go back to the first workspace
            debug!("going back to focusing the first workspace");
            let _ = self.send(niri_ipc::Request::Action(
                niri_ipc::Action::FocusWorkspace {
                    reference: niri_ipc::WorkspaceReferenceArg::Index(1),
                },
            ))?;
        }
        Ok(())
    }

    fn send(&mut self, request: niri_ipc::Request) -> Result<niri_ipc::Response> {
        self.socket
            .send(request)
            .context("on socket.send()")?
            .map_err(|e| anyhow!("on decoding Niri answer: {:?}", e))
    }
}
