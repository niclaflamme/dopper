use crate::security::MasterKey;
use crate::shared::{ask_for_confirmation, db_manager::DbManager, get_project_from_current_dir};

pub fn delete_env(db_manager: &DbManager, master_key: &MasterKey, slug: &str) {
    if slug == "dev" || slug == "prod" {
        eprintln!("Cannot delete default environment '{}'.", slug);
        return;
    }

    if let Some(project) = get_project_from_current_dir(db_manager, master_key) {
        if ask_for_confirmation(&format!(
            "Are you sure you want to delete environment '{}'? This action is irrevocable.",
            slug
        )) {
            match db_manager.delete_environment(master_key, &project.id, slug) {
                Ok(_) => println!(
                    "Environment '{}' deleted for project '{}'.",
                    slug, project.name
                ),
                Err(e) => eprintln!("Error deleting environment: {}", e),
            }
        } else {
            println!("Operation aborted.");
        }
    }
}
