use crate::db::DbManager;

pub fn get_env_slug(db_manager: &DbManager, env: &Option<String>, project_id: &str) -> String {
    if let Some(env_str) = env {
        env_str.clone()
    } else {
        db_manager
            .get_active_environment(project_id)
            .unwrap_or_else(|_| "dev".to_string())
    }
}
