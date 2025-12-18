use comfy_table::{Row, Table};
use crate::db::DbManager;
use crate::shared::get_project_from_current_dir;

pub fn list_envs(db_manager: &DbManager) {
    if let Some(project) = get_project_from_current_dir(db_manager) {
        match db_manager.list_environments(&project.id) {
            Ok(environments) => {
                if environments.is_empty() {
                    println!("No environments found for project '{}'.", project.name);
                } else {
                    let mut table = Table::new();
                    table.set_header(vec!["ID", "Project ID", "Slug"]);
                    for env in environments {
                        table.add_row(Row::from(vec![env.id, env.project_id, env.slug]));
                    }
                    println!("{table}");
                }
            }
            Err(e) => eprintln!("Error listing environments: {}", e),
        }
    }
}
