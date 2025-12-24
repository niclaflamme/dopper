use crate::shared::db_manager::DbManager;

pub fn unlock(db_manager: &DbManager) {
    if !db_manager.db_path_exists() {
        eprintln!("Dopper database not found. Please run `dopper init` to initialize.");
        return;
    }
    eprintln!("Unlocking to plaintext is not supported with the master password model.");
}
