use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use serde_yaml::to_string;

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
    pub fn touch_if_not_exists(path: &PathBuf) -> Result<(), std::io::Error> {
        if !path.exists() {
            let config = Config::default();
            config.save(path)
        } else {
            Ok(())
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let serialized_yaml = to_string(self).expect("Failed to serialize config");
        fs::write(path, serialized_yaml)
    }

    pub fn load(path: &PathBuf) -> Result<Self, std::io::Error> {
        let file_content = fs::read_to_string(path)?;
        let config: Config =
            serde_yaml::from_str(&file_content).expect("Failed to deserialize config");
        Ok(config)
    }
}
