use std::fs;
use std::path::PathBuf;
use crate::db::DbManager;
use crate::config::Config;

pub fn dump(db_manager: &DbManager, file: Option<String>, stdout: bool) {
    match db_manager.dump() {
        Ok(dump) => {
            let json = serde_json::to_string_pretty(&dump).expect("Failed to serialize dump");

            if stdout {
                println!("{}", json);
            } else {
                let path = if let Some(f) = file {
                    PathBuf::from(f)
                } else {
                    let config = Config::load();
                    config.get_dump_path()
                };

                fs::write(&path, json).expect("Failed to write dump file");
                println!("Database dumped to '{}'", path.display());
            }
        }
        Err(e) => eprintln!("Error dumping database: {}", e),
    }
}
