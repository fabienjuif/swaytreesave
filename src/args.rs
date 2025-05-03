use clap::{Parser, Subcommand};

/// Save your sway tree, and reload it. Provide a name if you wish!
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub mode: Mode,

    /// Name of your tree
    #[arg(long)]
    pub name: Option<String>,

    /// Dry run
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,

    /// No kill
    #[arg(long, default_value_t = false)]
    pub no_kill: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Mode {
    /// Save your current sway tree
    Save,
    /// Load a sway tree
    Load {
        /// Specify the workspace to load.
        /// Other workspaces app will not be killed, and only this workspace apps will be loaded from config file.
        #[arg(long)]
        workspace: Option<String>,
    },
}
