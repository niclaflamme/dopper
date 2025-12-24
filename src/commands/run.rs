use std::process::Command;

use crate::security::MasterKey;
use crate::shared::db_manager::DbManager;
use crate::shared::{get_effective_env, get_env_slug, get_project_from_current_dir};

pub fn run(db_manager: &DbManager, master_key: &MasterKey, command: &[String], env: &Option<String>) {
    if command.is_empty() {
        eprintln!("No command provided to `run`.");
        return;
    }

    if let Some(project) = get_project_from_current_dir(db_manager, master_key) {
        let env_slug = get_env_slug(db_manager, master_key, env, &project.id);
        match get_effective_env(db_manager, master_key, &project, &env_slug) {
            Ok(env_vars) => {
                let mut cmd = Command::new(&command[0]);
                cmd.args(&command[1..]);
                for (key, value) in env_vars {
                    cmd.env(key, value);
                }

                let mut child = cmd.spawn().expect("Failed to execute command");
                let status = child.wait().expect("Command wasn't running");
                if !status.success() {
                    eprintln!("Command exited with non-zero status: {}", status);
                }
            }
            Err(e) => eprintln!("{}", e),
        }
    }
}
