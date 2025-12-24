use std::fs;

use clap::{Parser, Subcommand};
use directories::UserDirs;

use dopper::commands;
use dopper::security::{derive_key, MasterKey};
use dopper::shared::db_manager::DbManager;

#[derive(Parser, Debug)]
#[command(author, version, about = "Dopper: Environment variable manager with zero footprint", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Links the current directory to a project
    Link {
        /// Name of the project to link to. If not provided, lists existing projects or prompts to create a new one.
        project_name: Option<String>,
    },
    /// Manages projects
    Project {
        #[command(subcommand)]
        command: ProjectCommands,
    },
    /// Manages secrets for the current project
    Secrets {
        #[command(subcommand)]
        command: SecretsCommands,
    },
    /// Manages environments for the current project
    Env {
        #[command(subcommand)]
        command: EnvCommands,
    },
    /// Runs a command with the project's secrets loaded
    Run {
        /// The command to execute
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,

        #[arg(long, short)]
        env: Option<String>,
    },
    /// Initializes Dopper by creating the necessary database
    Init {},
    /// Destroys the database (Irrevocable)
    Destroy {},
    /// Dumps the database state to a JSON file (default) or stdout
    Dump {
        /// Output file path override
        #[arg(long, short)]
        file: Option<String>,

        /// Print to stdout instead of file
        #[arg(long)]
        stdout: bool,
    },
    /// Restores the database state from a JSON file
    Restore {
        /// Input file path
        file: String,
    },
    /// Encrypts the database (requires system auth)
    Lock {
        #[command(subcommand)]
        command: Option<LockCommands>,
    },
    /// Decrypts the database (stores in plaintext)
    Unlock {},
    /// Prints the environment variables in .env format
    Print {
        #[arg(long, short)]
        env: Option<String>,

        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Copies the environment variables in .env format to your clipboard
    Clip {
        #[arg(long, short)]
        env: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum LockCommands {
    /// Checks the lock status of the database
    Status {},
}

#[derive(Subcommand, Debug)]
enum ProjectCommands {
    /// Creates a new project
    Create { name: String },
    /// Lists all projects
    List {},
    /// Deletes a project and all its associated data
    Delete { name: String },
}

#[derive(Subcommand, Debug)]
enum SecretsCommands {
    /// Sets a secret for the current project
    Set {
        key: String,
        value: Option<String>,
        #[arg(long, short, default_value = "dev")]
        env: String,
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Unsets a secret for the current project
    Unset {
        key: String,
        #[arg(long, short, default_value = "dev")]
        env: String,
        /// Skip confirmation prompt
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Lists secrets for the current project
    List {
        #[arg(long, short)]
        env: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum EnvCommands {
    /// Switches the active environment for the current project
    Use { slug: String },
    /// Lists all environments for the current project
    List {},
    /// Creates a new environment
    Create { slug: String },
    /// Deletes an environment (except 'dev' and 'prod')
    Delete { slug: String },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    if !dopper::shared::is_macos() {
        eprintln!("Dopper currently only supports macOS.");
        std::process::exit(1);
    }

    let cli = Cli::parse();

    let base_dir = UserDirs::new()
        .map(|dirs| dirs.home_dir().join(".dopper"))
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Setup DbManager
    fs::create_dir_all(&base_dir).expect("Could not create dopper directory");
    let db_path = base_dir.join("dopper.db");
    let salt_path = base_dir.join("salt");
    let db_manager = DbManager::new(db_path);

    if !db_manager.db_path_exists() {
        match &cli.command {
            Commands::Init {} => {}
            Commands::Destroy {} => {} // Destroy handles missing DB gracefully
            Commands::Restore { .. } => {}
            _ => {
                println!("Dopper database not found. Please run `dopper init` to initialize.");
                return Ok(());
            }
        }
    }

    let master_key = match &cli.command {
        Commands::Init {} | Commands::Destroy {} | Commands::Lock { .. } | Commands::Unlock {} => None,
        _ => Some(load_master_key(&salt_path)?),
    };

    if let Some(master_key) = master_key.as_ref() {
        if db_manager.db_path_exists() {
            if let Err(err) = db_manager.connect(master_key) {
                if is_incorrect_password(&err) {
                    eprintln!("Access Denied: Incorrect Password");
                    std::process::exit(1);
                }
                return Err(err.into());
            }
        }
    }

    match &cli.command {
        Commands::Link { project_name } => {
            commands::link(&db_manager, master_key.as_ref().unwrap(), project_name.clone());
        }
        Commands::Project { command } => match command {
            ProjectCommands::Create { name } => {
                commands::create_project(&db_manager, master_key.as_ref().unwrap(), name);
            }
            ProjectCommands::List {} => {
                commands::list_projects(&db_manager, master_key.as_ref().unwrap());
            }
            ProjectCommands::Delete { name } => {
                commands::delete_project(&db_manager, master_key.as_ref().unwrap(), name);
            }
        },
        Commands::Secrets { command } => match command {
            SecretsCommands::Set { key, value, env, yes } => {
                commands::set_secret(
                    &db_manager,
                    master_key.as_ref().unwrap(),
                    key,
                    value.as_deref(),
                    env,
                    *yes,
                );
            }
            SecretsCommands::Unset { key, env, yes } => {
                commands::unset_secret(
                    &db_manager,
                    master_key.as_ref().unwrap(),
                    key,
                    env,
                    *yes,
                );
            }
            SecretsCommands::List { env } => {
                commands::list_secrets(&db_manager, master_key.as_ref().unwrap(), env.clone());
            }
        },
        Commands::Env { command } => match command {
            EnvCommands::Use { slug } => {
                commands::use_env(&db_manager, master_key.as_ref().unwrap(), slug);
            }
            EnvCommands::List {} => {
                commands::list_envs(&db_manager, master_key.as_ref().unwrap());
            }
            EnvCommands::Create { slug } => {
                commands::create_env(&db_manager, master_key.as_ref().unwrap(), slug);
            }
            EnvCommands::Delete { slug } => {
                commands::delete_env(&db_manager, master_key.as_ref().unwrap(), slug);
            }
        },
        Commands::Run { command, env } => {
            commands::run(&db_manager, master_key.as_ref().unwrap(), command, env);
        }
        Commands::Init {} => {
            commands::init(&db_manager, &salt_path);
        }
        Commands::Destroy {} => {
            commands::destroy(&db_manager);
        }
        Commands::Dump { file, stdout } => {
            commands::dump(&db_manager, master_key.as_ref().unwrap(), file.clone(), *stdout);
        }
        Commands::Restore { file } => {
            commands::restore(&db_manager, master_key.as_ref().unwrap(), file.clone());
        }
        Commands::Lock { command } => match command {
            Some(LockCommands::Status {}) => {
                commands::get_lock_status(&db_manager);
            }
            None => {
                commands::lock(&db_manager);
            }
        },
        Commands::Unlock {} => {
            commands::unlock(&db_manager);
        }
        Commands::Print { env, yes } => {
            commands::print(&db_manager, master_key.as_ref().unwrap(), env.clone(), *yes);
        }
        Commands::Clip { env } => {
            commands::clip(&db_manager, master_key.as_ref().unwrap(), env.clone());
        }
    }

    Ok(())
}

fn load_master_key(salt_path: &std::path::Path) -> anyhow::Result<MasterKey> {
    if !salt_path.exists() {
        eprintln!("Master password salt not found. Please run `dopper init`.");
        std::process::exit(1);
    }

    let salt = fs::read_to_string(salt_path)?.trim().to_string();
    let password = prompt_password("Enter Master Password: ");
    derive_key(&password, &salt)
}

fn prompt_password(prompt: &str) -> String {
    use std::io::Write;

    eprint!("{}", prompt);
    std::io::stderr().flush().expect("Failed to flush stderr");
    rpassword::read_password().expect("Failed to read password")
}

fn is_incorrect_password(err: &rusqlite::Error) -> bool {
    match err {
        rusqlite::Error::SqliteFailure(_, Some(msg)) => msg == "Incorrect Password",
        _ => false,
    }
}
