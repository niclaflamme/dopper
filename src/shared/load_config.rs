use std::fs;

use directories::UserDirs;

use crate::types::Config;

pub fn load_config() -> Config {
    let config_path =
        UserDirs::new().map(|dirs| dirs.home_dir().join(".dopper").join("settings.toml"));

    if let Some(path) = config_path {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = toml::from_str(&content) {
                    return config;
                }
            }
        }
    }

    Config { dump_path: None }
}
