use clap::{Parser, Subcommand};
use directories::UserDirs;
use std::fs;
use std::path::PathBuf;
use std::env;
use std::process::Command;

mod db;
use db::DbManager;

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
    /// DEPRECATED: use `project list` or `secrets list` instead.
    List {},
    /// Initializes Dopper by creating the necessary database
    Init {},
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
}

struct Dopper {
    db_manager: DbManager,
}

impl Dopper {
    fn new(base_dir: PathBuf) -> Self {
        fs::create_dir_all(&base_dir).expect("Could not create dopper directory");
        let db_manager = DbManager::new(&base_dir);
        Dopper { db_manager }
    }

    fn init(&self) {
        self.db_manager.initialize_db().expect("Database initialization failed");
        println!("Dopper initialized successfully.");
    }

    fn create_project(&self, name: &str) {
        match self.db_manager.create_project(name) {
            Ok(project) => println!("Project '{}' created successfully with id '{}'", project.name, project.id),
            Err(e) => eprintln!("Error creating project: {}", e),
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
                        println!("- {} (id: {}, created: {})", project.name, project.id, project.created_at);
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
                eprintln!("Project '{}' not found. Please create it first.", project_name);
                return;
            }
        };

        let current_dir = env::current_dir().expect("Could not get current directory");
        match self.db_manager.link_directory(&project.id, &current_dir) {
            Ok(_) => println!("Successfully linked directory {:?} to project '{}'.", current_dir, project.name),
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
                Ok(environment) => {
                    match self.db_manager.set_secret(&environment.id, key, value) {
                        Ok(_) => println!("Secret '{}' set for project '{}' in environment '{}'.", key, project.name, env),
                        Err(e) => eprintln!("Error setting secret: {}", e),
                    }
                }
                Err(e) => eprintln!("Error getting or creating environment: {}", e),
            }
        }
    }

    fn unset_secret(&self, key: &str, env: &str) {
        if let Some(project) = self.get_project_from_current_dir() {
            match self.db_manager.get_environment(&project.id, env) {
                Ok(environment) => {
                    match self.db_manager.unset_secret(&environment.id, key) {
                        Ok(_) => println!("Secret '{}' unset for project '{}' in environment '{}'.", key, project.name, env),
                        Err(e) => eprintln!("Error unsetting secret: {}", e),
                    }
                }
                Err(e) => eprintln!("Error getting environment: {}", e),
            }
        }
    }

    fn list_secrets(&self, env: &Option<String>) {
        if let Some(project) = self.get_project_from_current_dir() {
            let env_slug = self.get_env_slug(env, &project.id);
            match self.db_manager.get_environment(&project.id, &env_slug) {
                Ok(environment) => {
                    match self.db_manager.get_secrets(&environment.id) {
                        Ok(secrets) => {
                            if secrets.is_empty() {
                                println!("No secrets found for project '{}' in environment '{}'.", project.name, env_slug);
                            } else {
                                println!("Secrets for project '{}' in environment '{}':", project.name, env_slug);
                                for secret in secrets {
                                    println!("{}: {}", secret.key, secret.value);
                                }
                            }
                        }
                        Err(e) => eprintln!("Error listing secrets: {}", e),
                    }
                }
                Err(e) => eprintln!("Error getting environment: {}", e),
            }
        }
    }

    fn run(&self, command: &[String], env: &Option<String>) {
        if let Some(project) = self.get_project_from_current_dir() {
            let env_slug = self.get_env_slug(env, &project.id);
            match self.db_manager.get_environment(&project.id, &env_slug) {
                Ok(environment) => {
                    match self.db_manager.get_secrets(&environment.id) {
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
                    }
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    eprintln!("No secrets found for project '{}' in environment '{}'.", project.name, env_slug);
                }
                Err(e) => eprintln!("Error getting environment: {}", e),
            }
        }
    }

    fn get_env_slug(&self, env: &Option<String>, project_id: &str) -> String {
        if let Some(env_str) = env {
            env_str.clone()
        } else {
            self.db_manager.get_active_environment(project_id).unwrap_or_else(|_| "dev".to_string())
        }
    }

    fn set_active_environment(&self, slug: &str) {
        if let Some(project) = self.get_project_from_current_dir() {
            match self.db_manager.set_active_environment(&project.id, slug) {
                Ok(_) => println!("Active environment for project '{}' set to '{}'.", project.name, slug),
                Err(e) => eprintln!("Error setting active environment: {}", e),
            }
        }
    }

    fn list_environments(&self) {
        if let Some(project) = self.get_project_from_current_dir() {
            match self.db_manager.list_environments(&project.id) {
                Ok(environments) => {
                    if environments.is_empty() {
                        println!("No environments found for project '{}'.", project.name);
                    } else {
                        println!("Environments for project '{}':", project.name);
                        for env in environments {
                            println!("- {}", env);
                        }
                    }
                }
                Err(e) => eprintln!("Error listing environments: {}", e),
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let base_dir = UserDirs::new()
        .map(|dirs| dirs.home_dir().join(".dopper"))
        .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;

    let dopper = Dopper::new(base_dir);

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
                println!("Delete project command received. Name: {}", name);
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
        },
        Commands::Run { command, env } => {
            if command.is_empty() {
                eprintln!("No command provided to `run`.");
            } else {
                dopper.run(command, env);
            }
        }
        Commands::List {} => {
            eprintln!("`list` is deprecated. Use `project list` or `secrets list` instead.");
        }
        Commands::Init {} => {
            dopper.init();
        }
    }

    Ok(())
}
