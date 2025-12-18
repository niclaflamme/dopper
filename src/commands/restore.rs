use std::fs;
use crate::db::{self, DbManager};
use crate::shared::interactive;

pub fn restore(db_manager: &DbManager, file: String) {
    let content = fs::read_to_string(&file).expect("Failed to read dump file");
    let dump: db::DopperDump =
        serde_json::from_str(&content).expect("Failed to parse dump file");

    if interactive::confirm(&format!("This will OVERWRITE your current database with the content of '{}'. Are you sure?", file)) {
        match db_manager.restore(dump) {
            Ok(_) => println!("Database restored successfully from '{}'.", file),
            Err(e) => eprintln!("Error restoring database: {}", e),
        }
    } else {
        println!("Restore aborted.");
    }
}
