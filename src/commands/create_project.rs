use crate::db::DbManager;

pub fn create_project(db_manager: &DbManager, name: &str) {
    match db_manager.create_project(name) {
        Ok(project) => println!(
            "Project '{}' created successfully with id '{}'",
            project.name, project.id
        ),
        Err(e) => eprintln!("Error creating project: {}", e),
    }
}
