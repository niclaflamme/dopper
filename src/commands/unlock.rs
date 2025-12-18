use crate::shared::db_manager::DbManager;
use crate::shared::is_macos;

pub fn unlock(db_manager: &DbManager) {
    if !is_macos() {
        eprintln!("Unlocking is currently only supported on macOS.");
        return;
    }

    match db_manager.is_locked() {
        Ok(false) => {
            println!("Database is already unlocked.");
            return;
        }
        Err(e) => {
            eprintln!("Error checking lock status: {}", e);
            return;
        }
        Ok(true) => {}
    }

    match db_manager.unlock() {
        Ok(_) => println!("Database unlocked (decrypted) successfully."),
        Err(e) => eprintln!("Error unlocking database: {}", e),
    }
}
