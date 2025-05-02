use std::{fs, path::PathBuf, thread, time::Duration};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_yaml::to_string;
use swayipc::{Connection, Fallible};

// TODO: make a diff and kill not wanted windows + spawn missing ones?
// TODO: watch get_tree to check if an app is started (with a watchdog) instead of hardcoding a sleep

/// Save your sway tree, and reload it. Provide a name if you wish!
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    mode: Mode,

    /// Name of your tree
    #[arg(long)]
    name: Option<String>,

    /// Dry run
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// No kill
    #[arg(long, default_value_t = false)]
    no_kill: bool,
}

#[derive(Subcommand, Debug, Clone)]
enum Mode {
    /// Save your current sway tree
    Save,
    /// Load a sway tree
    Load,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
enum NodeType {
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
    fn from_sway(node_type: swayipc::NodeType) -> Self {
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
    fn from_sway(node_layout: swayipc::NodeLayout) -> Self {
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

    fn is_unknown(&self) -> bool {
        matches!(self, NodeLayout::Unknown)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
struct Node {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app_id: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "NodeType::is_window", default)]
    node_type: NodeType,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    nodes: Vec<Node>,
    #[serde(skip_serializing_if = "skip_if_none_or_zero")]
    fullscreen_mode: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    percent: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    desktop_file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exec: Option<String>,
    #[serde(skip_serializing_if = "NodeLayout::is_unknown", default)]
    layout: NodeLayout,
}

fn main() -> Fallible<()> {
    let options = Args::parse();

    let xdg_dirs = xdg::BaseDirectories::with_prefix("swaytreesave").unwrap();
    let config_file_path = xdg_dirs
        .place_config_file((options.name.unwrap_or("default".to_owned())) + ".yaml")
        .expect("Failed to create config file");

    match options.mode {
        Mode::Save => save_tree(config_file_path, options.dry_run),
        Mode::Load => load_tree(config_file_path, options.dry_run, options.no_kill),
    }
}

fn save_tree(config_file_path: PathBuf, dry_run: bool) -> Fallible<()> {
    // build saveable tree
    let sway_tree = Connection::new()?.get_tree()?;
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

fn load_tree(config_file_path: PathBuf, dry_run: bool, no_kill: bool) -> Fallible<()> {
    eprintln!("Loading tree from {:?}", config_file_path);

    // loading tree from file
    let file_content = fs::read_to_string(&config_file_path).expect(stringify!(
        "Failed to read from {}",
        config_file_path.display()
    ));
    let tree: Vec<Node> = serde_yaml::from_str(&file_content).expect("Failed to deserialize tree");

    // cleaning everything (next time just diff windows if possible rather than starting from scratch)
    let mut connection = Connection::new()?;
    let sway_tree = connection.get_tree()?;
    for node in sway_tree.iter() {
        if node.node_type == swayipc::NodeType::Workspace {
            if node.name.is_none() {
                continue;
            };
            if node.name.as_ref().unwrap() == "__i3_scratch" {
                continue;
            }
            kill_recursive(&mut connection, node, dry_run, no_kill);
        }
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
            spawn_recursive(&mut connection, node, dry_run);
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
        node_type: NodeType::from_sway(node.node_type),
        app_id: node.app_id.clone(),
        nodes: vec![],
        fullscreen_mode: node.fullscreen_mode,
        percent: node.percent,
        layout: NodeLayout::from_sway(node.layout),
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

fn kill_recursive(connection: &mut Connection, node: &swayipc::Node, dry_run: bool, no_kill: bool) {
    if node.app_id == Some("code-oss".to_owned()) {
        // FIXME:
        return;
    }
    if node.node_type == swayipc::NodeType::Con || node.node_type == swayipc::NodeType::FloatingCon
    {
        let cmd = format!("[con_id={}] kill", node.id);
        if dry_run || no_kill {
            println!("\t{:?} => {:?}", node.app_id, cmd);
        } else {
            connection
                .run_command(cmd)
                .expect(stringify!("Failed to kill node with id {}", node.id));
        }
    }

    for child in node.nodes.iter() {
        kill_recursive(connection, child, dry_run, no_kill);
    }
}

fn spawn_recursive(connection: &mut Connection, node: &Node, dry_run: bool) {
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

    if node.node_type == NodeType::Con
        || node.node_type == NodeType::FloatingCon
        || node.node_type == NodeType::Unknown
    {
        let cmd = if let Some(desktop_file) = &node.desktop_file {
            Some(format!("exec {}", desktop_file))
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
                connection
                    .run_command(cmd)
                    .expect(stringify!("Failed to spawn app {}", node.id));
                thread::sleep(Duration::from_secs(1));
            }
        }
    }

    for (index, child) in node.nodes.iter().enumerate() {
        spawn_recursive(connection, child, dry_run);
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

fn skip_if_none_or_zero(opt: &Option<u8>) -> bool {
    matches!(opt, None | Some(0))
}

fn extract_cmdline(pid: &i32) -> Result<String, std::io::Error> {
    let path = format!("/proc/{}/cmdline", pid);

    let data = fs::read(&path)?;
    let joined = data
        .split(|&b| b == 0)
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(joined)
}
