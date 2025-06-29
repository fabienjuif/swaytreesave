// TODO: Resize support
// TODO: make a diff and kill not wanted windows + spawn missing ones?

use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use serde_yaml::to_string;

use crate::{
    config,
    consts::MAX_WAIT_DURATION,
    models::{Node, NodeLayout, NodeType},
    util::extract_cmdline,
};

pub fn save_tree(config_file_path: PathBuf, dry_run: bool) -> Result<()> {
    // build saveable tree
    let sway_tree = swayipc::Connection::new()?.get_tree()?;
    let mut tree = vec![];
    for node in sway_tree.iter() {
        if node.node_type == swayipc::NodeType::Workspace {
            if node.name.is_none() {
                continue;
            };
            if node.name.as_ref().unwrap() == "__i3_scratch" {
                continue;
            }
            tree.push(parse_children(node));
        }
    }

    // TODO: sort by workspace name

    if dry_run {
        println!("tree[{:?}]:\n{:?}", config_file_path, tree);
        return Ok(());
    }

    // saving into given config file
    let serialized_yaml = to_string(&tree).expect("Failed to serialize tree");
    fs::write(&config_file_path, serialized_yaml).expect(stringify!(
        "Failed to write to {}",
        config_file_path.display()
    ));

    //
    println!("tree saved into: {:?}", config_file_path);

    Ok(())
}

pub fn load_tree(
    config: &config::Config,
    config_file_path: PathBuf,
    dry_run: bool,
    no_kill: bool,
    workspace: Option<String>,
) -> Result<()> {
    eprintln!("Loading tree from {:?}", config_file_path);

    // loading tree from file
    let file_content = fs::read_to_string(&config_file_path).expect(stringify!(
        "Failed to read from {}",
        config_file_path.display()
    ));
    let tree: Vec<Node> = serde_yaml::from_str(&file_content).expect("Failed to deserialize tree");

    // cleaning everything (next time just diff windows if possible rather than starting from scratch)
    let mut connection = swayipc::Connection::new()?;
    let sway_tree = connection.get_tree()?;
    for node in sway_tree.iter() {
        if node.node_type == swayipc::NodeType::Workspace {
            if node.name.is_none() {
                continue;
            };
            if node.name.as_ref().unwrap() == "__i3_scratch" {
                continue;
            }
            if workspace.is_some() && node.name != workspace {
                continue;
            }
            kill_recursive(&mut connection, node, dry_run, no_kill);
        }
    }

    // TODO: remove this once kill_recursive is fixed
    if !dry_run && !no_kill {
        thread::sleep(Duration::from_millis(100));
    }

    // spawning windows
    for node in tree.iter() {
        if node.node_type == NodeType::Workspace {
            if node.name.is_none() {
                continue;
            };
            if node.name.as_ref().unwrap() == "__i3_scratch" {
                continue;
            }
            if workspace.is_some() && node.name != workspace {
                continue;
            }
            spawn_recursive(&mut connection, node, &config.desktop_exec, dry_run);
        }
    }

    Ok(())
}

fn parse_children(node: &swayipc::Node) -> Node {
    let name = if node.node_type == swayipc::NodeType::Workspace {
        node.name.clone()
    } else {
        None
    };
    let mut parent = Node {
        name,
        node_type: NodeType::from(node.node_type),
        app_id: node.app_id.clone(),
        nodes: vec![],
        fullscreen_mode: node.fullscreen_mode,
        percent: node.percent,
        layout: NodeLayout::from(node.layout),
        ..Default::default()
    };

    if let Some(pid) = &node.pid {
        parent.exec = match extract_cmdline(pid) {
            Ok(cmd) => Some(cmd),
            Err(_) => {
                eprintln!("Failed to extract command line for PID {}", pid);
                None
            }
        }
    }

    for child in node.nodes.iter() {
        parent.nodes.push(parse_children(child));
    }

    parent
}

fn kill_recursive(
    connection: &mut swayipc::Connection,
    node: &swayipc::Node,
    dry_run: bool,
    no_kill: bool,
) {
    if node.node_type == swayipc::NodeType::Con || node.node_type == swayipc::NodeType::FloatingCon
    {
        // TODO: count before/after to check if the app is really killed
        let cmd = format!("[con_id={}] kill", node.id);
        println!("\t{:?} => {:?}", node.app_id, cmd);
        if !dry_run && !no_kill {
            connection
                .run_command(cmd)
                .expect(stringify!("Failed to kill node with id {}", node.id));
        }
    }

    for child in node.nodes.iter() {
        kill_recursive(connection, child, dry_run, no_kill);
    }
}

fn spawn_recursive(
    connection: &mut swayipc::Connection,
    node: &Node,
    desktop_exec: &str,
    dry_run: bool,
) {
    if node.node_type == NodeType::Workspace {
        if let Some(name) = &node.name {
            let cmd = format!("workspace {}", name);
            println!("{:?}", cmd);
            if !dry_run {
                connection
                    .run_command(cmd)
                    .expect(stringify!("Failed to spawn workspace {}", name));
            }
        }
    }

    if matches!(
        node.node_type,
        NodeType::Con | NodeType::FloatingCon | NodeType::Unknown
    ) {
        let cmd = if let Some(desktop_file) = &node.desktop_entry {
            Some(format!(
                "exec {} \"{}\"",
                desktop_exec,
                desktop_file.replace("\"", "\\\"")
            ))
        } else if let Some(exec) = &node.exec {
            Some(format!("exec \"{}\"", exec.replace("\"", "\\\"")))
        } else {
            node.app_id
                .as_ref()
                .map(|app_id| format!("exec {}", app_id))
        };

        if let Some(cmd) = cmd {
            println!("\t{:?}", cmd);
            if !dry_run {
                for i in 0..node.retry.unwrap_or(1) {
                    if i > 0 {
                        println!("\tRetrying...");
                    }
                    match spawn_and_wait(connection, &cmd, &node.app_id, &node.timeout) {
                        Ok(_) => break,
                        Err(e) => {
                            eprintln!("{}", e);
                        }
                    }
                }
            }
        }
    }

    for (index, child) in node.nodes.iter().enumerate() {
        spawn_recursive(connection, child, desktop_exec, dry_run);
        if index == 0 {
            if node.layout == NodeLayout::SplitH {
                let cmd = "split h".to_string();
                println!("\t{:?}", cmd);

                if !dry_run {
                    connection
                        .run_command(cmd)
                        .expect(stringify!("Failed to split h"));
                }
            } else if node.layout == NodeLayout::SplitV {
                let cmd = "split v".to_string();
                println!("\t{:?}", cmd);
                if !dry_run {
                    connection
                        .run_command(cmd)
                        .expect(stringify!("Failed to split v"));
                }
            }
        }
    }
}

fn spawn_and_wait(
    connection: &mut swayipc::Connection,
    cmd: &str,
    app_id: &Option<String>,
    timeout: &Option<Duration>,
) -> swayipc::Fallible<()> {
    let before = if let Some(app_id) = &app_id {
        count_app_ids(connection, app_id).expect("Failed to count app ids")
    } else {
        0
    };
    connection
        .run_command(cmd)
        .expect(stringify!("Failed to spawn app {}", node.id));
    if let Some(app_id) = &app_id {
        let now = Instant::now();
        while let Ok(after) = count_app_ids(connection, app_id) {
            if after > before {
                break;
            }
            if now.elapsed() > timeout.unwrap_or(MAX_WAIT_DURATION) {
                return Err(swayipc::Error::Io(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Timed out waiting for app to spawn",
                )));
            }
            eprintln!("sleep 100ms");
            thread::sleep(Duration::from_millis(100));
        }
    }
    Ok(())
}

fn count_app_ids(connection: &mut swayipc::Connection, app_id: &str) -> swayipc::Fallible<usize> {
    let mut count = 0;
    for child in connection.get_tree()?.nodes.iter() {
        count += count_app_ids_recurse(app_id, child);
    }
    Ok(count)
}

fn count_app_ids_recurse(app_id: &str, node: &swayipc::Node) -> usize {
    let mut count = if node.app_id == Some(app_id.to_string()) {
        1
    } else {
        0
    };
    for child in node.nodes.iter() {
        count += count_app_ids_recurse(app_id, child);
    }
    count
}

impl From<swayipc::NodeType> for NodeType {
    fn from(node_type: swayipc::NodeType) -> Self {
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
}

impl From<swayipc::NodeLayout> for NodeLayout {
    fn from(node_layout: swayipc::NodeLayout) -> Self {
        match node_layout {
            swayipc::NodeLayout::SplitH => NodeLayout::SplitH,
            swayipc::NodeLayout::SplitV => NodeLayout::SplitV,
            swayipc::NodeLayout::Stacked => NodeLayout::Stacked,
            swayipc::NodeLayout::Tabbed => NodeLayout::Tabbed,
            swayipc::NodeLayout::Output => NodeLayout::Output,
            swayipc::NodeLayout::Dockarea => NodeLayout::Dockarea,
            swayipc::NodeLayout::None => NodeLayout::None,
            _ => NodeLayout::Unknown,
        }
    }
}
