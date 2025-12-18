use crate::db::DbManager;
use crate::shared::utils;

pub fn set_secret(db_manager: &DbManager, key: &str, value: &str, env: &str) {
    if let Some(project) = utils::get_project_from_current_dir(db_manager) {
        match db_manager.get_or_create_environment(&project.id, env) {
            Ok(environment) => match db_manager.set_secret(&environment.id, key, value) {
                Ok(_) => println!(
                    "Secret '{}' set for project '{}' in environment '{}'.",
                    key, project.name, env
                ),
                Err(e) => eprintln!("Error setting secret: {}", e),
            },
            Err(e) => eprintln!("Error getting or creating environment: {}", e),
        }
    }
}