use crate::db::DbManager;
use crate::shared::{get_env_slug, get_project_from_current_dir};

pub fn list_secrets(db_manager: &DbManager, env: Option<String>) {
    if let Some(project) = get_project_from_current_dir(db_manager) {
        let env_slug = get_env_slug(db_manager, &env, &project.id);
        match db_manager.get_environment(&project.id, &env_slug) {
            Ok(environment) => match db_manager.get_secrets(&environment.id) {
                Ok(secrets) => {
                    if secrets.is_empty() {
                        println!(
                            "No secrets found for project '{}' in environment '{}'.",
                            project.name, env_slug
                        );
                    } else {
                        println!(
                            "Secrets for project '{}' in environment '{}':",
                            project.name, env_slug
                        );
                        for secret in secrets {
                            println!("{}: {}", secret.key, secret.value);
                        }
                    }
                }
                Err(e) => eprintln!("Error listing secrets: {}", e),
            },
            Err(e) => eprintln!("Error getting environment: {}", e),
        }
    }
}
