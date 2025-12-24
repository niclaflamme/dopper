use std::env;

use crate::security::MasterKey;
use crate::shared::db_manager::{self, DbManager};

pub fn get_project_from_current_dir(
    db_manager: &DbManager,
    master_key: &MasterKey,
) -> Option<db_manager::Project> {
    let current_dir = env::current_dir().expect("Could not get current directory");
    match db_manager.get_project_from_path(master_key, &current_dir) {
        Ok(Some(project)) => Some(project),
        Ok(None) => {
            eprintln!("Current directory is not linked to any project.");
            None
        }
        Err(e) => {
            eprintln!("Error getting project from path: {}", e);
            None
        }
    }
}
