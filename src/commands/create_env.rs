use crate::shared::db_manager::DbManager;
use crate::shared::get_project_from_current_dir;

pub fn create_env(db_manager: &DbManager, slug: &str) {
    if let Some(project) = get_project_from_current_dir(db_manager) {
        match db_manager.create_environment(&project.id, slug) {
            Ok(_) => println!(
                "Environment '{}' created for project '{}'.",
                slug, project.name
            ),
            Err(e) => eprintln!("Error creating environment: {}", e),
        }
    }
}
