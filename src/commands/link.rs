use std::env;
use crate::db::DbManager;
use crate::commands::project_list;

pub fn link(db_manager: &DbManager, project_name: Option<String>) {
    if let Some(name) = project_name {
        let project = match db_manager.get_project_by_name(&name) {
            Ok(p) => p,
            Err(_) => {
                eprintln!(
                    "Project '{}' not found. Please create it first.",
                    name
                );
                return;
            }
        };

        let current_dir = env::current_dir().expect("Could not get current directory");
        match db_manager.link_directory(&project.id, &current_dir) {
            Ok(_) => println!(
                "Successfully linked directory {:?} to project '{}'.",
                current_dir, project.name
            ),
            Err(e) => eprintln!("Error linking directory: {}", e),
        }
    } else {
        println!("No project name provided. Please specify a project to link.");
        project_list::list(db_manager);
    }
}
