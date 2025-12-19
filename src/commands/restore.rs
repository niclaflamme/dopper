use std::fs;

use crate::shared::ask_for_confirmation;
use crate::shared::db_manager::{self, DbManager};

pub fn restore(db_manager: &DbManager, file: String) {
    let content = fs::read_to_string(&file).expect("Failed to read dump file");
    let dump: db_manager::DopperDump =
        serde_json::from_str(&content).expect("Failed to parse dump file");

    if ask_for_confirmation(&format!(
        "This will OVERWRITE your current database with the content of '{}'. Are you sure?",
        file
    )) {
        match db_manager.restore(dump) {
            Ok(_) => println!("Database restored successfully from '{}'.", file),
            Err(e) => eprintln!("Error restoring database: {}", e),
        }
    } else {
        println!("Restore aborted.");
    }
}
