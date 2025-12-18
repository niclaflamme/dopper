use crate::shared::db_manager::DbManager;

pub fn list_projects(db_manager: &DbManager) {
    match db_manager.list_projects() {
        Ok(projects) => {
            if projects.is_empty() {
                println!("No projects found.");
            } else {
                println!("Projects:");
                for project in projects {
                    println!(
                        "- {} (id: {}, created: {})",
                        project.name, project.id, project.created_at
                    );
                }
            }
        }
        Err(e) => eprintln!("Error listing projects: {}", e),
    }
}
