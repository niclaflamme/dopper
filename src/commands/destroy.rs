use crate::db::DbManager;
use crate::shared::ask_for_confirmation;

pub fn destroy(db_manager: &DbManager) {
     if ask_for_confirmation("Are you sure you want to destroy the database? This action is irrevocable.") {
        match db_manager.delete_database() {
            Ok(_) => println!("Database destroyed successfully."),
            Err(e) => eprintln!("Error destroying database: {}", e),
        }
    } else {
        println!("Operation aborted.");
    }
}