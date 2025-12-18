use crate::shared::db_manager::DbManager;
use crate::shared::get_project_from_current_dir;

pub fn unset_secret(db_manager: &DbManager, key: &str, env: &str) {
    if let Some(project) = get_project_from_current_dir(db_manager) {
        match db_manager.get_environment(&project.id, env) {
            Ok(environment) => match db_manager.unset_secret(&environment.id, key) {
                Ok(_) => println!(
                    "Secret '{}' unset for project '{}' in environment '{}'.",
                    key, project.name, env
                ),
                Err(e) => eprintln!("Error unsetting secret: {}", e),
            },
            Err(e) => eprintln!("Error getting environment: {}", e),
        }
    }
}
