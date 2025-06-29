mod args;
mod config;
mod consts;
mod models;
mod niri;
mod sway;
mod util;

use anyhow::{Context, Result};
use args::{Args, Mode};
use clap::Parser;

fn main() -> Result<()> {
    let options = Args::parse();

    let app_name = env!("CARGO_PKG_NAME");
    let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name).context(format!(
        "failed to access swaytreesave xdg directories: {app_name}"
    ))?;

    let config_file = "config.yaml";
    let config_file_path = xdg_dirs
        .place_config_file(config_file)
        .context(format!("failed to access config file path: {config_file}"))?;
    config::Config::touch_if_not_exists(&config_file_path).context(format!(
        "failed to create config file: {}",
        config_file_path.display()
    ))?;
    let config = config::Config::load(&config_file_path).context(format!(
        "failed to load config file: {}",
        config_file_path.display()
    ))?;

    let tree_file = options.name.unwrap_or("default".to_owned()) + ".yaml";
    let tree_file_path = xdg_dirs
        .place_config_file(&tree_file)
        .context(format!("failed to access tree file path: {tree_file}"))?;

    match options.mode {
        Mode::Save => sway::save_tree(&tree_file_path, options.dry_run)
            .context(format!("failed to save tree: {}", tree_file_path.display())),
        Mode::Load { workspace } => sway::load_tree(
            &config,
            &tree_file_path,
            options.dry_run,
            options.no_kill,
            workspace,
        )
        .context(format!("failed to load tree: {}", tree_file_path.display())),
    }
}
