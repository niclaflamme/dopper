use crate::shared::db_manager::DbManager;

pub fn get_lock_status(db_manager: &DbManager) {
    if !db_manager.db_path_exists() {
        eprintln!("Dopper database not found. Please run `dopper init` to initialize.");
        return;
    }
    println!("locked");
}
