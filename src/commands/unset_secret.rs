use crate::shared::ask_for_confirmation;
use crate::shared::db_manager::DbManager;
use crate::shared::get_project_from_current_dir;

pub fn unset_secret(db_manager: &DbManager, key: &str, env: &str, yes: bool) {
    if let Some(project) = get_project_from_current_dir(db_manager) {
        match db_manager.get_environment(&project.id, env) {
            Ok(environment) => {
                if !yes {
                    // Check if it exists first
                    match db_manager.secret_exists(&environment.id, key) {
                        Ok(true) => {
                            if !ask_for_confirmation("This action is irrevocable. Are you sure?") {
                                return;
                            }
                        }
                        Ok(false) => {
                            println!(
                                "Secret '{}' not found in environment '{}'.",
                                key, env
                            );
                            return;
                        }
                        Err(e) => {
                            eprintln!("Error checking secret existence: {}", e);
                            return;
                        }
                    }
                }

                match db_manager.unset_secret(&environment.id, key) {
                    Ok(_) => println!(
                        "Secret '{}' unset for project '{}' in environment '{}'.",
                        key, project.name, env
                    ),
                    Err(e) => eprintln!("Error unsetting secret: {}", e),
                }
            }
            Err(e) => eprintln!("Error getting environment: {}", e),
        }
    }
}
