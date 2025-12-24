use crate::security::MasterKey;
use crate::shared::db_manager::DbManager;

pub fn get_env_slug(
    db_manager: &DbManager,
    master_key: &MasterKey,
    env: &Option<String>,
    project_id: &str,
) -> String {
    if let Some(env_str) = env {
        env_str.clone()
    } else {
        db_manager
            .get_active_environment(master_key, project_id)
            .unwrap_or_else(|_| "dev".to_string())
    }
}
