use crate::types::Config;
use directories::UserDirs;
use std::path::PathBuf;

pub fn get_dump_path(config: &Config) -> PathBuf {
    if let Some(path_str) = &config.dump_path {
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
