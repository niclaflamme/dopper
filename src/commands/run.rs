use std::process::Command;

use crate::db::DbManager;
use crate::shared::{get_env_slug, get_project_from_current_dir};

pub fn run(db_manager: &DbManager, command: &[String], env: &Option<String>) {
    if command.is_empty() {
        eprintln!("No command provided to `run`.");
        return;
    }

    if let Some(project) = get_project_from_current_dir(db_manager) {
        let env_slug = get_env_slug(db_manager, env, &project.id);
        match db_manager.get_environment(&project.id, &env_slug) {
            Ok(environment) => match db_manager.get_secrets(&environment.id) {
                Ok(secrets) => {
                    let mut cmd = Command::new(&command[0]);
                    cmd.args(&command[1..]);
                    for secret in secrets {
                        cmd.env(secret.key, secret.value);
                    }

                    let mut child = cmd.spawn().expect("Failed to execute command");
                    let status = child.wait().expect("Command wasn't running");
                    if !status.success() {
                        eprintln!("Command exited with non-zero status: {}", status);
                    }
                }
                Err(e) => eprintln!("Error getting secrets: {}", e),
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                eprintln!(
                    "No secrets found for project '{}' in environment '{}'.",
                    project.name, env_slug
                );
            }
            Err(e) => eprintln!("Error getting environment: {}", e),
        }
    }
}
