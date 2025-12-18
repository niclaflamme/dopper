use crate::db::DbManager;
use crate::shared::get_project_from_current_dir;

pub fn use_env(db_manager: &DbManager, slug: &str) {
    if let Some(project) = get_project_from_current_dir(db_manager) {
        match db_manager.set_active_environment(&project.id, slug) {
            Ok(_) => println!(
                "Active environment for project '{}' set to '{}'.",
                project.name, slug
            ),
            Err(e) => eprintln!("Error setting active environment: {}", e),
        }
    }
}