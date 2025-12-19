use crate::shared::ask_for_confirmation;
use crate::shared::db_manager::DbManager;

pub fn delete_project(db_manager: &DbManager, name: &str) {
    if ask_for_confirmation(&format!(
        "Are you sure you want to delete project '{}' and ALL associated data (secrets, envs)?",
        name
    )) {
        match db_manager.delete_project(name) {
            Ok(_) => println!("Project '{}' deleted successfully.", name),
            Err(e) => eprintln!("Error deleting project: {}", e),
        }
    } else {
        println!("Operation aborted.");
    }
}
