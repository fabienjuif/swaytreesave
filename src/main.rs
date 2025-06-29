mod args;
mod config;
mod consts;
mod models;
mod niri;
mod sway;
mod util;

use anyhow::{Context, Ok, Result};
use args::{Args, Mode};
use clap::Parser;
use models::{Compositor, load_tree, save_tree};
use tracing::{error, level_filters::LevelFilter, warn};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use xdg::BaseDirectories;

fn main() {
    if let Err(e) = run() {
        error!("{e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let app_name = env!("CARGO_PKG_NAME");
    // we keep the guard around for the duration of the application
    // to ensure that all logs are flushed before the application exits.
    let _guard = init_logging(app_name).context("on init_logging()")?;

    let options = Args::parse();

    let base_dirs = xdg::BaseDirectories::with_prefix(app_name)
        .context(format!("failed to access xdg directories: {app_name}"))?;

    // TODO: make sway branch reuse get_tree_path() function
    // TODO: make sway branch reuse save_tree() function
    if options.compositor == Compositor::Sway {
        let config_file = "config.yaml";
        let config_file_path = base_dirs
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
        let tree_file_path = base_dirs
            .place_config_file(&tree_file)
            .context(format!("failed to access tree file path: {tree_file}"))?;

        return match options.mode {
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
        };
    }

    // Niri branch
    let tree_path = config::get_tree_path(base_dirs, options.compositor, options.name)?;
    let mut n = niri::Niri::new()?;

    match options.mode {
        Mode::Save => {
            let tree = n.get_tree().context("on niri::Niri::get_tree()")?;
            save_tree(&tree_path, &tree).context("on save_tree()")
        }
        Mode::Load { workspace } => {
            if let Some(ws) = &workspace {
                warn!(
                    "loading a specific workspace is incompatible with Niri, ignoring it (trying to load {ws})"
                );
            }
            n.clear().context("on n.clear()")?;
            let tree = load_tree(&tree_path).context("on load_tree()")?;
            n.load_tree(&tree).context("on n.load_tree()")
        }
    }
}

// the returned guard must be held for the duration you want logging to occur.
// when it is dropped, any buffered logs are flushed.
fn init_logging(application_name: &str) -> Result<WorkerGuard> {
    let xdg_dirs = BaseDirectories::with_prefix(application_name)?;
    let log_directory = xdg_dirs.create_state_directory("logs")?;
    let file_appender = tracing_appender::rolling::daily(log_directory, application_name);
    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(file_appender);
    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env_lossy();
    let file_subscriber = tracing_subscriber::fmt::layer().with_writer(non_blocking_writer);
    let console_subscriber = tracing_subscriber::fmt::layer().with_writer(std::io::stdout);
    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(console_subscriber)
        .with(env_filter)
        .init();
    Ok(_guard)
}
