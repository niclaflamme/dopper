use std::io::{self, Write};
use std::path::Path;

use crate::security::{derive_key, generate_salt};
use crate::shared::db_manager::DbManager;
use crate::shared::is_macos;

pub fn init(db_manager: &DbManager, salt_path: &Path) {
    if db_manager.db_path_exists() {
        println!(
            "Dopper database already exists at {:?}.",
            db_manager.db_path()
        );
        println!(
            "If you want to re-initialize, please delete the database first using `dopper destroy` or manually deleting the file."
        );
        return;
    }

    let master_password = prompt_master_password("Create a Master Password: ");
    let salt = generate_salt();
    std::fs::write(salt_path, &salt).expect("Failed to write salt file");
    let master_key = derive_key(&master_password, &salt).expect("Failed to derive master key");

    db_manager
        .initialize_db(&master_key)
        .expect("Database initialization failed");
    log::debug!("Dopper initialized successfully.");

    if is_macos() {
        println!("Master password set. Your database is encrypted.");
    }
}

fn prompt_master_password(prompt: &str) -> String {
    eprint!("{}", prompt);
    io::stderr().flush().expect("Failed to flush stderr");
    rpassword::read_password().expect("Failed to read password")
}
