use std::fs;
use std::path::PathBuf;

use crate::security::MasterKey;
use crate::shared::{db_manager::DbManager, get_dump_path, load_config};

pub fn dump(db_manager: &DbManager, master_key: &MasterKey, file: Option<String>, stdout: bool) {
    match db_manager.dump(master_key) {
        Ok(dump) => {
            let json = serde_json::to_string_pretty(&dump).expect("Failed to serialize dump");

            if stdout {
                println!("{}", json);
            } else {
                let path = if let Some(f) = file {
                    PathBuf::from(f)
                } else {
                    let config = load_config();
                    get_dump_path(&config)
                };

                fs::write(&path, json).expect("Failed to write dump file");
                println!("Database dumped to '{}'", path.display());
            }
        }
        Err(e) => eprintln!("Error dumping database: {}", e),
    }
}
