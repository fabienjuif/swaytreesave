use std::vec;

// TODO: I do not think Niri expose the columns, so this is not possible to see how windows are arranged
// TODO: same for columns widths then
use anyhow::{Context, Result, anyhow, bail};
use tracing::{debug, info, warn};

use crate::{
    config::Config,
    models::{Node, NodeLayout, NodeType},
};

pub struct Niri {
    socket: niri_ipc::socket::Socket,
    cfg: Config,
    dry_run: bool,
}

// TODO: make it a trait for sway
impl Niri {
    pub fn new(cfg: Config, dry_run: bool) -> Result<Self> {
        let socket = niri_ipc::socket::Socket::connect().context("on Socket::connect()")?;
        Ok(Self {
            socket,
            cfg,
            dry_run,
        })
    }

    /// Spawns a command and waits for it to finish.
    /// TODO: wait is not implemented yet, so it just spawns the command.
    fn spawn_and_wait(&mut self, node: &Node) -> Result<()> {
        let cmd = if let Some(desktop_file) = &node.desktop_entry {
            debug!("\tspawning from desktop entry: {desktop_file}");
            Some(format!(
                "{} \"{}\"",
                self.cfg.desktop_exec,
                desktop_file.replace("\"", "\\\"")
            ))
        } else if let Some(exec) = &node.exec {
            debug!("\tspawning from exec: {exec}");
            Some(exec.replace("\"", "\\\""))
        } else {
            debug!("\tspawning from app_id: {:?}", node.app_id);
            node.app_id.clone()
        };

        let Some(cmd) = cmd else {
            bail!("cannot spawn application without app_id, desktop_entry or exec");
        };

        let _ = self
            .send(niri_ipc::Request::Action(niri_ipc::Action::Spawn {
                command: vec!["sh".to_string(), "-c".to_string(), cmd.to_string()],
            }))
            .context(format!("on spawn action with command: {cmd}"))?;
        Ok(())
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

                    debug!(
                        "spawning application: {:?}",
                        node.app_id.as_ref().unwrap_or(&"unknown".to_string())
                    );
                    self.spawn_and_wait(node)
                        .context(format!("on spawn_and_wait for node: {node:?}"))?;
                }
            }
        }
        // go back to the first workspace
        debug!("going back to focusing the first workspace");
        let _ = self.send(niri_ipc::Request::Action(
            niri_ipc::Action::FocusWorkspace {
                reference: niri_ipc::WorkspaceReferenceArg::Index(1),
            },
        ))?;
        Ok(())
    }

    fn send(&mut self, request: niri_ipc::Request) -> Result<niri_ipc::Response> {
        if self.dry_run {
            info!("dry run mode, not sending request: {:?}", request);
            return Ok(niri_ipc::Response::Handled);
        }
        self.socket
            .send(request)
            .context("on socket.send()")?
            .map_err(|e| anyhow!("on decoding Niri answer: {:?}", e))
    }
}
