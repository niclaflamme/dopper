use std::io::{self, Write};
use crate::db::DbManager;
use crate::os;

pub fn init(db_manager: &DbManager) {
    if db_manager.db_path_exists() {
        println!("Dopper database already exists at {:?}.", db_manager.db_path());
        println!("If you want to re-initialize, please delete the database first using `dopper destroy` or manually deleting the file.");
        return;
    }

    db_manager
        .initialize_db()
        .expect("Database initialization failed");
    log::debug!("Dopper initialized successfully.");

    if os::is_macos() {
        println!("\nWould you like to encrypt your database? (Recommended)");
        println!(
            "This will secure your secrets using your system keychain. You may be prompted for your system password when accessing Dopper."
        );
        print!("Enable encryption? (y/N): ");
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");

        if input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes") {
            match db_manager.lock() {
                Ok(_) => println!("Database locked (encrypted) successfully."),
                Err(e) => eprintln!("Error locking database: {}", e),
            }
        } else {
            println!(
                "Database left unencrypted (Plaintext). You can encrypt it later using `dopper lock`."
            );
        }
    }
}
