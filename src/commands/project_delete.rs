use crate::db::DbManager;
use crate::shared::interactive;

pub fn delete(db_manager: &DbManager, name: &str) {
    if interactive::confirm(&format!("Are you sure you want to delete project '{}' and ALL associated data (secrets, envs)?", name)) {
        match db_manager.delete_project(name) {
            Ok(_) => println!("Project '{}' deleted successfully.", name),
            Err(e) => eprintln!("Error deleting project: {}", e),
        }
    } else {
        println!("Operation aborted.");
    }
}
