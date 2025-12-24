use crate::security::MasterKey;
use crate::shared::db_manager::DbManager;

pub fn create_project(db_manager: &DbManager, master_key: &MasterKey, name: &str) {
    match db_manager.create_project(master_key, name) {
        Ok(project) => println!(
            "Project '{}' created successfully with id '{}'",
            project.name, project.id
        ),
        Err(e) => eprintln!("Error creating project: {}", e),
    }
}
