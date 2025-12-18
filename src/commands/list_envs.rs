use comfy_table::{Row, Table};

use crate::shared::db_manager::DbManager;
use crate::shared::get_project_from_current_dir;

pub fn list_envs(db_manager: &DbManager) {
    if let Some(project) = get_project_from_current_dir(db_manager) {
        match db_manager.list_environments(&project.id) {
            Ok(environments) => {
                if environments.is_empty() {
                    println!("No environments found for project '{}'.", project.name);
                } else {
                    let active_env = db_manager
                        .get_active_environment(&project.id)
                        .unwrap_or_else(|_| "dev".to_string());

                    let mut table = Table::new();
                    table.set_header(vec!["Active", "Slug"]);
                    for env in environments {
                        let is_active = if env.slug == active_env { "✔" } else { "" };
                        table.add_row(Row::from(vec![
                            is_active.to_string(),
                            env.slug,
                        ]));
                    }
                    println!("{table}");
                }
            }
            Err(e) => eprintln!("Error listing environments: {}", e),
        }
    }
}
