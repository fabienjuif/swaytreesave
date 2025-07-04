use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_yaml::to_string;
use xdg::BaseDirectories;

use crate::models::Compositor;

const DEFAULT_DESKTOP_EXEC: &str = "gtk-launch";
fn default_desktop_exec() -> String {
    DEFAULT_DESKTOP_EXEC.to_string()
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    /// The desktop launcher to use
    #[serde(
        skip_serializing_if = "String::is_empty",
        default = "default_desktop_exec"
    )]
    pub desktop_exec: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            desktop_exec: DEFAULT_DESKTOP_EXEC.to_string(),
        }
    }
}

impl Config {
    pub fn touch_if_not_exists(path: &Path) -> Result<()> {
        if !path.exists() {
            let config = Config::default();
            return config.save(path);
        }
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let serialized_yaml = to_string(self).context("on to_string()")?;
        fs::write(path, serialized_yaml).context("on fs::write()")?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let file_content = fs::read_to_string(path).context("on fs::read_to_string()")?;
        let config: Config =
            serde_yaml::from_str(&file_content).context("on serde_yaml::from_str()")?;
        Ok(config)
    }
}

pub fn get_tree_path(
    base_dirs: BaseDirectories,
    compositor: Compositor,
    tree_name: Option<String>,
) -> Result<PathBuf> {
    let sub_dir = match compositor {
        Compositor::Sway => "",
        Compositor::Niri => "niri",
    };
    let file_path_str = &format!(
        "{sub_dir}/{}.yaml",
        tree_name.unwrap_or("default".to_owned())
    );
    base_dirs.place_config_file(file_path_str).context(format!(
        "failed to access config file path: {file_path_str}"
    ))
}
