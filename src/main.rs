use clap::{Parser, Subcommand};
use comfy_table::{Row, Table};
use directories::UserDirs;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use dopper::config::Config;
use dopper::db::{self, DbManager};
use dopper::os;
use dopper::security::KeychainProvider;

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
        value: String,
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

// -------------------------------------------------------------------------------------------------
// ---- Application --------------------------------------------------------------------------------

struct Dopper {
    db_manager: DbManager,
}

impl Dopper {
    fn new(base_dir: PathBuf) -> Self {
        fs::create_dir_all(&base_dir).expect("Could not create dopper directory");
        let db_path = base_dir.join("dopper.db");
        let key_provider = Box::new(KeychainProvider::new());
        let db_manager = DbManager::new(db_path, key_provider);
        Dopper { db_manager }
    }

    fn init(&self) {
        if self.db_manager.db_path_exists() {
            println!("Dopper database already exists at {:?}.", self.db_manager.db_path());
            println!("If you want to re-initialize, please delete the database first using `dopper destroy` or manually deleting the file.");
            return;
        }

        self.db_manager
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
                self.lock();
            } else {
                println!(
                    "Database left unencrypted (Plaintext). You can encrypt it later using `dopper lock`."
                );
            }
        }
    }

    fn destroy(&self) {
        print!(
            "Are you sure you want to destroy the database? This action is irrevocable. (y/N): "
        );
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");

        if input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes") {
            match self.db_manager.delete_database() {
                Ok(_) => println!("Database destroyed successfully."),
                Err(e) => eprintln!("Error destroying database: {}", e),
            }
        } else {
            println!("Operation aborted.");
        }
    }

    fn create_project(&self, name: &str) {
        match self.db_manager.create_project(name) {
            Ok(project) => println!(
                "Project '{}' created successfully with id '{}'",
                project.name, project.id
            ),
            Err(e) => eprintln!("Error creating project: {}", e),
        }
    }

    fn delete_project(&self, name: &str) {
        print!("Are you sure you want to delete project '{}' and ALL associated data (secrets, envs)? (y/N): ", name);
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read input");

        if input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes") {
            match self.db_manager.delete_project(name) {
                Ok(_) => println!("Project '{}' deleted successfully.", name),
                Err(e) => eprintln!("Error deleting project: {}", e),
            }
        } else {
            println!("Operation aborted.");
        }
    }

    fn list_projects(&self) {
        match self.db_manager.list_projects() {
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

    fn link(&self, project_name: &str) {
        let project = match self.db_manager.get_project_by_name(project_name) {
            Ok(p) => p,
            Err(_) => {
                eprintln!(
                    "Project '{}' not found. Please create it first.",
                    project_name
                );
                return;
            }
        };

        let current_dir = env::current_dir().expect("Could not get current directory");
        match self.db_manager.link_directory(&project.id, &current_dir) {
            Ok(_) => println!(
                "Successfully linked directory {:?} to project '{}'.",
                current_dir, project.name
            ),
            Err(e) => eprintln!("Error linking directory: {}", e),
        }
    }

    fn get_project_from_current_dir(&self) -> Option<db::Project> {
        let current_dir = env::current_dir().expect("Could not get current directory");
        match self.db_manager.get_project_from_path(&current_dir) {
            Ok(Some(project)) => Some(project),
            Ok(None) => {
                eprintln!("Current directory is not linked to any project.");
                None
            }
            Err(e) => {
                eprintln!("Error getting project from path: {}", e);
                None
            }
        }
    }

    fn set_secret(&self, key: &str, value: &str, env: &str) {
        if let Some(project) = self.get_project_from_current_dir() {
            match self.db_manager.get_or_create_environment(&project.id, env) {
                Ok(environment) => match self.db_manager.set_secret(&environment.id, key, value) {
                    Ok(_) => println!(
                        "Secret '{}' set for project '{}' in environment '{}'.",
                        key, project.name, env
                    ),
                    Err(e) => eprintln!("Error setting secret: {}", e),
                },
                Err(e) => eprintln!("Error getting or creating environment: {}", e),
            }
        }
    }

    fn unset_secret(&self, key: &str, env: &str) {
        if let Some(project) = self.get_project_from_current_dir() {
            match self.db_manager.get_environment(&project.id, env) {
                Ok(environment) => match self.db_manager.unset_secret(&environment.id, key) {
                    Ok(_) => println!(
                        "Secret '{}' unset for project '{}' in environment '{}'.",
                        key, project.name, env
                    ),
                    Err(e) => eprintln!("Error unsetting secret: {}", e),
                },
                Err(e) => eprintln!("Error getting environment: {}", e),
            }
        }
    }

    fn list_secrets(&self, env: &Option<String>) {
        if let Some(project) = self.get_project_from_current_dir() {
            let env_slug = self.get_env_slug(env, &project.id);
            match self.db_manager.get_environment(&project.id, &env_slug) {
                Ok(environment) => match self.db_manager.get_secrets(&environment.id) {
                    Ok(secrets) => {
                        if secrets.is_empty() {
                            println!(
                                "No secrets found for project '{}' in environment '{}'.",
                                project.name, env_slug
                            );
                        } else {
                            println!(
                                "Secrets for project '{}' in environment '{}':",
                                project.name, env_slug
                            );
                            for secret in secrets {
                                println!("{}: {}", secret.key, secret.value);
                            }
                        }
                    }
                    Err(e) => eprintln!("Error listing secrets: {}", e),
                },
                Err(e) => eprintln!("Error getting environment: {}", e),
            }
        }
    }

    fn run(&self, command: &[String], env: &Option<String>) {
        if let Some(project) = self.get_project_from_current_dir() {
            let env_slug = self.get_env_slug(env, &project.id);
            match self.db_manager.get_environment(&project.id, &env_slug) {
                Ok(environment) => match self.db_manager.get_secrets(&environment.id) {
                    Ok(secrets) => {
                        let mut cmd = Command::new(&command[0]);
                        cmd.args(&command[1..]);
                        for secret in secrets {
                            cmd.env(secret.key, secret.value);
                        }

                        let mut child = cmd.spawn().expect("Failed to execute command");
                        let status = child.wait().expect("Command wasn't running");
                        if !status.success() {
                            eprintln!("Command exited with non-zero status: {}", status);
                        }
                    }
                    Err(e) => eprintln!("Error getting secrets: {}", e),
                },
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    eprintln!(
                        "No secrets found for project '{}' in environment '{}'.",
                        project.name, env_slug
                    );
                }
                Err(e) => eprintln!("Error getting environment: {}", e),
            }
        }
    }

    fn get_env_slug(&self, env: &Option<String>, project_id: &str) -> String {
        if let Some(env_str) = env {
            env_str.clone()
        } else {
            self.db_manager
                .get_active_environment(project_id)
                .unwrap_or_else(|_| "dev".to_string())
        }
    }

    fn set_active_environment(&self, slug: &str) {
        if let Some(project) = self.get_project_from_current_dir() {
            match self.db_manager.set_active_environment(&project.id, slug) {
                Ok(_) => println!(
                    "Active environment for project '{}' set to '{}'.",
                    project.name, slug
                ),
                Err(e) => eprintln!("Error setting active environment: {}", e),
            }
        }
    }

    fn create_env(&self, slug: &str) {
        if let Some(project) = self.get_project_from_current_dir() {
            match self.db_manager.create_environment(&project.id, slug) {
                Ok(_) => println!(
                    "Environment '{}' created for project '{}'.",
                    slug, project.name
                ),
                Err(e) => eprintln!("Error creating environment: {}", e),
            }
        }
    }

    fn delete_env(&self, slug: &str) {
        if slug == "dev" || slug == "prod" {
            eprintln!("Cannot delete default environment '{}'.", slug);
            return;
        }

        if let Some(project) = self.get_project_from_current_dir() {
            print!(
                "Are you sure you want to delete environment '{}'? This action is irrevocable. (y/N): ",
                slug
            );
            io::stdout().flush().expect("Failed to flush stdout");

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input");

            if input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes") {
                match self.db_manager.delete_environment(&project.id, slug) {
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

    fn dump(&self, file: Option<String>, stdout: bool) {
        match self.db_manager.dump() {
            Ok(dump) => {
                let json = serde_json::to_string_pretty(&dump).expect("Failed to serialize dump");

                if stdout {
                    println!("{}", json);
                } else {
                    let path = if let Some(f) = file {
                        PathBuf::from(f)
                    } else {
                        let config = Config::load();
                        config.get_dump_path()
                    };

                    fs::write(&path, json).expect("Failed to write dump file");
                    println!("Database dumped to '{}'", path.display());
                }
            }
            Err(e) => eprintln!("Error dumping database: {}", e),
        }
    }

    fn restore(&self, file: String) {
        let content = fs::read_to_string(&file).expect("Failed to read dump file");
        let dump: db::DopperDump =
            serde_json::from_str(&content).expect("Failed to parse dump file");

        print!(
            "This will OVERWRITE your current database with the content of '{}'. Are you sure? (y/N): ",
            file
        );
        io::stdout().flush().expect("Failed to flush stdout");

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read input");

        if input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes") {
            match self.db_manager.restore(dump) {
                Ok(_) => println!("Database restored successfully from '{}'.", file),
                Err(e) => eprintln!("Error restoring database: {}", e),
            }
        } else {
            println!("Restore aborted.");
        }
    }

    fn lock(&self) {
        if !os::is_macos() {
            eprintln!("Locking (encryption) is currently only supported on macOS.");
            return;
        }

        match self.db_manager.is_locked() {
            Ok(true) => {
                println!("Database is already locked.");
                return;
            }
            Err(e) => {
                eprintln!("Error checking lock status: {}", e);
                return;
            }
            Ok(false) => {}
        }

        match self.db_manager.lock() {
            Ok(_) => println!("Database locked (encrypted) successfully."),
            Err(e) => eprintln!("Error locking database: {}", e),
        }
    }

    fn lock_status(&self) {
        match self.db_manager.is_locked() {
            Ok(locked) => {
                if locked {
                    println!("locked");
                } else {
                    println!("unlocked");
                }
            }
            Err(e) => eprintln!("Error checking lock status: {}", e),
        }
    }

    fn unlock(&self) {
        if !os::is_macos() {
            eprintln!("Unlocking is currently only supported on macOS.");
            return;
        }

        match self.db_manager.is_locked() {
            Ok(false) => {
                println!("Database is already unlocked.");
                return;
            }
            Err(e) => {
                eprintln!("Error checking lock status: {}", e);
                return;
            }
            Ok(true) => {}
        }

        match self.db_manager.unlock() {
            Ok(_) => println!("Database unlocked (decrypted) successfully."),
            Err(e) => eprintln!("Error unlocking database: {}", e),
        }
    }

    fn list_environments(&self) {
        if let Some(project) = self.get_project_from_current_dir() {
            match self.db_manager.list_environments(&project.id) {
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
}

// -------------------------------------------------------------------------------------------------
// ---- Main ---------------------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let base_dir = UserDirs::new()
        .map(|dirs| dirs.home_dir().join(".dopper"))
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    let dopper = Dopper::new(base_dir);

    if !dopper.db_manager.db_path_exists() {
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
            if let Some(name) = project_name {
                dopper.link(name);
            } else {
                println!("No project name provided. Please specify a project to link.");
                dopper.list_projects();
            }
        }
        Commands::Project { command } => match command {
            ProjectCommands::Create { name } => {
                dopper.create_project(name);
            }
            ProjectCommands::List {} => {
                dopper.list_projects();
            }
            ProjectCommands::Delete { name } => {
                dopper.delete_project(name);
            }
        },
        Commands::Secrets { command } => match command {
            SecretsCommands::Set { key, value, env } => {
                dopper.set_secret(key, value, env);
            }
            SecretsCommands::Unset { key, env } => {
                dopper.unset_secret(key, env);
            }
            SecretsCommands::List { env } => {
                dopper.list_secrets(env);
            }
        },
        Commands::Env { command } => match command {
            EnvCommands::Use { slug } => {
                dopper.set_active_environment(slug);
            }
            EnvCommands::List {} => {
                dopper.list_environments();
            }
            EnvCommands::Create { slug } => {
                dopper.create_env(slug);
            }
            EnvCommands::Delete { slug } => {
                dopper.delete_env(slug);
            }
        },
        Commands::Run { command, env } => {
            if command.is_empty() {
                eprintln!("No command provided to `run`.");
            } else {
                dopper.run(command, env);
            }
        }
        Commands::Init {} => {
            dopper.init();
        }
        Commands::Destroy {} => {
            dopper.destroy();
        }
        Commands::Dump { file, stdout } => {
            dopper.dump(file.clone(), *stdout);
        }
        Commands::Restore { file } => {
            dopper.restore(file.clone());
        }
        Commands::Lock { command } => match command {
            Some(LockCommands::Status {}) => {
                dopper.lock_status();
            }
            None => {
                dopper.lock();
            }
        },
        Commands::Unlock {} => {
            dopper.unlock();
        }
    }

    Ok(())
}

// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
