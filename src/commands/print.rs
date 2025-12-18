use crate::shared::db_manager::DbManager;
use crate::shared::{
    ask_for_confirmation, get_effective_env, get_env_slug, get_project_from_current_dir,
};

pub fn print(db_manager: &DbManager, env: Option<String>) {
    if let Some(project) = get_project_from_current_dir(db_manager) {
        let env_slug = get_env_slug(db_manager, &env, &project.id);

        if !ask_for_confirmation(&format!(
            "You are about to print secrets for environment '{}'. Are you sure?",
            env_slug
        )) {
            return;
        }

        match get_effective_env(db_manager, &project, &env_slug) {
            Ok(env_vars) => {
                for (key, value) in env_vars {
                    println!("{}={}", key, value);
                }
            }
            Err(e) => eprintln!("{}", e),
        }
    }
}
