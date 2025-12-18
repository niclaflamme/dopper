use crate::db::DbManager;
use crate::shared::{get_env_slug, get_project_from_current_dir};

pub fn print_dot_env(db_manager: &DbManager, env: Option<String>) {
    if let Some(project) = get_project_from_current_dir(db_manager) {
        let env_slug = get_env_slug(db_manager, &env, &project.id);
        
        match db_manager.get_environment(&project.id, &env_slug) {
            Ok(environment) => match db_manager.get_secrets(&environment.id) {
                Ok(secrets) => {
                    for secret in secrets {
                        println!("{}={}", secret.key, secret.value);
                    }
                }
                Err(e) => eprintln!("Error retrieving secrets: {}", e),
            },
            Err(_) => {
                eprintln!("Environment '{}' not found for project '{}'.", env_slug, project.name);
            }
        }
    }
}
