use crate::shared::ask_for_confirmation;
use crate::security::MasterKey;
use crate::shared::db_manager::DbManager;
use crate::shared::get_project_from_current_dir;
use rpassword::prompt_password;

pub fn set_secret(
    db_manager: &DbManager,
    master_key: &MasterKey,
    key: &str,
    value: Option<&str>,
    env: &str,
    yes: bool,
) {
    if let Some(project) = get_project_from_current_dir(db_manager, master_key) {
        match db_manager.get_or_create_environment(master_key, &project.id, env) {
            Ok(environment) => {
                // Check if secret already exists
                if !yes {
                    match db_manager.secret_exists(master_key, &environment.id, key) {
                        Ok(true) => {
                            if !ask_for_confirmation(&format!(
                                "There is already a value for '{}' in the '{}' environment. Overwrite?",
                                key, env
                            )) {
                                return;
                            }
                        }
                        Ok(false) => {}
                        Err(e) => {
                            eprintln!("Error checking secret existence: {}", e);
                            return;
                        }
                    }
                }

                let final_value = match value {
                    Some(v) => v.to_string(),
                    None => {
                        let prompt = format!("Enter value for {}: ", key);
                        match prompt_password(prompt) {
                            Ok(v) => v,
                            Err(e) => {
                                eprintln!("Error reading value: {}", e);
                                return;
                            }
                        }
                    }
                };

                match db_manager.set_secret(master_key, &environment.id, key, &final_value) {
                    Ok(_) => println!(
                        "Secret '{}' set for project '{}' in environment '{}'.",
                        key, project.name, env
                    ),
                    Err(e) => eprintln!("Error setting secret: {}", e),
                }
            }
            Err(e) => eprintln!("Error getting or creating environment: {}", e),
        }
    }
}
