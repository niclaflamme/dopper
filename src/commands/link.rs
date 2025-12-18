use std::env;
use std::io::{self, Write};

use crate::db::DbManager;

pub fn link(db_manager: &DbManager, project_name: Option<String>) {
    let projects = match db_manager.list_projects() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error listing projects: {}", e);
            return;
        }
    };

    if projects.is_empty() {
        println!("No projects found.");
        return;
    }

    let project = if let Some(name) = project_name {
        match projects.into_iter().find(|p| p.name == name) {
            Some(p) => p,
            None => {
                eprintln!("Project '{}' not found. Please create it first.", name);
                return;
            }
        }
    } else {
        println!("Select a project to link:");
        for (i, p) in projects.iter().enumerate() {
            println!("{}. {}", i + 1, p.name);
        }
        print!("Selection (1-{}): ", projects.len());
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");

        let choice: usize = match input.trim().parse::<usize>() {
            Ok(n) if n > 0 && n <= projects.len() => n - 1,
            _ => {
                eprintln!("Invalid selection.");
                return;
            }
        };
        projects.into_iter().nth(choice).unwrap()
    };

    let current_dir = env::current_dir().expect("Could not get current directory");
    match db_manager.link_directory(&project.id, &current_dir) {
        Ok(_) => println!(
            "Successfully linked directory {:?} to project '{}'.",
            current_dir, project.name
        ),
        Err(e) => eprintln!("Error linking directory: {}", e),
    }
}
