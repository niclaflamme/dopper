use std::fs;

use clap::{Parser, Subcommand};
use directories::UserDirs;

use dopper::commands;
use dopper::shared::db_manager::DbManager;
use dopper::shared::keychain::KeychainProvider;

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
    },
    /// Unsets a secret for the current project
    Unset {
        key: String,
        #[arg(long, short, default_value = "dev")]
        env: String,
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
    let cli = Cli::parse();

    let base_dir = UserDirs::new()
        .map(|dirs| dirs.home_dir().join(".dopper"))
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    // Setup DbManager
    fs::create_dir_all(&base_dir).expect("Could not create dopper directory");
    let db_path = base_dir.join("dopper.db");
    let key_provider = Box::new(KeychainProvider::new());
    let db_manager = DbManager::new(db_path, key_provider);

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

    match &cli.command {
        Commands::Link { project_name } => {
            commands::link(&db_manager, project_name.clone());
        }
        Commands::Project { command } => match command {
            ProjectCommands::Create { name } => {
                commands::create_project(&db_manager, name);
            }
            ProjectCommands::List {} => {
                commands::list_projects(&db_manager);
            }
            ProjectCommands::Delete { name } => {
                commands::delete_project(&db_manager, name);
            }
        },
        Commands::Secrets { command } => match command {
            SecretsCommands::Set { key, value, env } => {
                commands::set_secret(&db_manager, key, value.as_deref(), env);
            }
            SecretsCommands::Unset { key, env } => {
                commands::unset_secret(&db_manager, key, env);
            }
            SecretsCommands::List { env } => {
                commands::list_secrets(&db_manager, env.clone());
            }
        },
        Commands::Env { command } => match command {
            EnvCommands::Use { slug } => {
                commands::use_env(&db_manager, slug);
            }
            EnvCommands::List {} => {
                commands::list_envs(&db_manager);
            }
            EnvCommands::Create { slug } => {
                commands::create_env(&db_manager, slug);
            }
            EnvCommands::Delete { slug } => {
                commands::delete_env(&db_manager, slug);
            }
        },
        Commands::Run { command, env } => {
            commands::run(&db_manager, command, env);
        }
        Commands::Init {} => {
            commands::init(&db_manager);
        }
        Commands::Destroy {} => {
            commands::destroy(&db_manager);
        }
        Commands::Dump { file, stdout } => {
            commands::dump(&db_manager, file.clone(), *stdout);
        }
        Commands::Restore { file } => {
            commands::restore(&db_manager, file.clone());
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
            commands::print(&db_manager, env.clone(), *yes);
        }
        Commands::Clip { env } => {
            commands::clip(&db_manager, env.clone());
        }
    }

    Ok(())
}
