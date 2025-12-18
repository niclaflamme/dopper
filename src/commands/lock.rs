use crate::db::DbManager;
use crate::shared::is_macos;

pub fn lock(db_manager: &DbManager) {
    if !is_macos() {
        eprintln!("Locking (encryption) is currently only supported on macOS.");
        return;
    }

    match db_manager.is_locked() {
        Ok(true) => {
            println!("Database is already locked.");
            return;
        }
        Err(e) => {
            eprintln!("Error checking lock status: {}", e);
            return;
        }
        Ok(false) => {}
    }

    match db_manager.lock() {
        Ok(_) => println!("Database locked (encrypted) successfully."),
        Err(e) => eprintln!("Error locking database: {}", e),
    }
}