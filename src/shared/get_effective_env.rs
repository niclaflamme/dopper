use anyhow::{Result, anyhow};

use crate::shared::db_manager::{DbManager, Project};

pub fn get_effective_env(
    db_manager: &DbManager,
    project: &Project,
    env_slug: &str,
) -> Result<Vec<(String, String)>> {
    let environment = db_manager
        .get_environment(&project.id, env_slug)
        .map_err(|_| {
            anyhow!(
                "Environment '{}' not found for project '{}'.",
                env_slug,
                project.name
            )
        })?;

    let secrets = db_manager
        .get_secrets(&environment.id)
        .map_err(|e| anyhow!("Error retrieving secrets: {}", e))?;

    let mut env_vars: Vec<(String, String)> =
        secrets.into_iter().map(|s| (s.key, s.value)).collect();

    // Inject system variables
    // Prepend or append? Usually overwrites happen if duplicates.
    // If the user set DOPPER_PROJECT_ID manually in secrets, it would appear twice here.
    // We should probably filter it out from secrets or ensure these take precedence (or let user override).
    // Let's assume these system vars should be added. Using a Vec means duplicates are possible.
    // Command::env overrides. println! just prints.
    // Let's filter out any existing keys that clash with system ones to enforce system values?
    // Or just push them.

    // Let's enforce system values by removing any user-defined ones with the same key first, just in case.
    env_vars.retain(|(k, _)| k != "DOPPER_PROJECT_ID" && k != "DOPPER_ENV");

    env_vars.push(("DOPPER_PROJECT_ID".to_string(), project.id.clone()));
    env_vars.push(("DOPPER_ENV".to_string(), env_slug.to_string()));

    Ok(env_vars)
}
