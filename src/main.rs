mod args;
mod config;
mod consts;
mod models;
mod niri;
mod sway;
mod util;

use std::path::PathBuf;

use anyhow::Result;
use args::{Args, Mode};
use clap::Parser;

fn main() -> Result<()> {
    let options = Args::parse();

    let xdg_dirs = xdg::BaseDirectories::with_prefix("swaytreesave").unwrap();

    let config_file_path: PathBuf = xdg_dirs
        .place_config_file("config.yaml")
        .expect("Failed to create config file");
    config::Config::touch_if_not_exists(&config_file_path).expect("Failed to create config file");
    let config = config::Config::load(&config_file_path).expect("Failed to load config file");

    let tree_file_path: PathBuf = xdg_dirs
        .place_config_file((options.name.unwrap_or("default".to_owned())) + ".yaml")
        .expect("Failed to create tree file");

    match options.mode {
        Mode::Save => sway::save_tree(tree_file_path, options.dry_run),
        Mode::Load { workspace } => sway::load_tree(
            &config,
            tree_file_path,
            options.dry_run,
            options.no_kill,
            workspace,
        ),
    }
}
