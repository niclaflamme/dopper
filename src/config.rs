use directories::UserDirs;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub dump_path: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let config_path = UserDirs::new()
            .map(|dirs| dirs.home_dir().join(".dopper").join("settings.toml"));

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

    pub fn get_dump_path(&self) -> PathBuf {
        if let Some(path_str) = &self.dump_path {
            // fast and loose tilde expansion for config values
            if path_str.starts_with("~/") {
                 if let Some(dirs) = UserDirs::new() {
                     return dirs.home_dir().join(&path_str[2..]);
                 }
            }
            return PathBuf::from(path_str);
        }

        // Default default
        UserDirs::new()
            .map(|dirs| dirs.home_dir().join(".dopper").join("dump.json"))
            .unwrap_or_else(|| PathBuf::from(".dopper/dump.json"))
    }
}
