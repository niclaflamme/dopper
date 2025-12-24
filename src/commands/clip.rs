use crate::security::MasterKey;
use crate::shared::db_manager::DbManager;
use crate::shared::{get_effective_env, get_env_slug, get_project_from_current_dir, set_clipboard};

pub fn clip(db_manager: &DbManager, master_key: &MasterKey, env: Option<String>) {
    if let Some(project) = get_project_from_current_dir(db_manager, master_key) {
        let env_slug = get_env_slug(db_manager, master_key, &env, &project.id);

        match get_effective_env(db_manager, master_key, &project, &env_slug) {
            Ok(env_vars) => {
                let mut output = String::new();
                for (key, value) in env_vars {
                    output.push_str(&key);
                    output.push('=');
                    output.push_str(&value);
                    output.push('\n');
                }

                if let Err(e) = set_clipboard(&output) {
                    eprintln!("{e}");
                }
            }
            Err(e) => eprintln!("{}", e),
        }
    }
}
