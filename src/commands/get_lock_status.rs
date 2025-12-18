use crate::db::DbManager;

pub fn get_lock_status(db_manager: &DbManager) {
    match db_manager.is_locked() {
        Ok(locked) => {
            if locked {
                println!("locked");
            } else {
                println!("unlocked");
            }
        }
        Err(e) => eprintln!("Error checking lock status: {}", e),
    }
}
