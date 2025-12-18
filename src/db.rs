// src/db.rs

use rusqlite::{Connection, Result};
use std::path::{Path, PathBuf};
use std::fs;
use md5;
use thiserror::Error;
use std::collections::HashMap;
use uuid::Uuid;
use path_clean::PathClean;

#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("Database is corrupted. MD5 mismatch for migration {0}.")]
    Corruption(String),
    #[error("Missing migration file: {0}")]
    MissingFile(String),
}

pub struct DbManager {
    db_path: PathBuf,
}

#[derive(Debug)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug)]
pub struct Environment {
    pub id: String,
    pub project_id: String,
    pub slug: String,
}

#[derive(Debug)]
pub struct Secret {
    pub key: String,
    pub value: String,
}

impl DbManager {
    pub fn new(base_dir: &Path) -> Self {
        let db_path = base_dir.join("dopper.db");
        DbManager { db_path }
    }

    pub fn connect(&self) -> Result<Connection> {
        Connection::open(&self.db_path)
    }

    pub fn initialize_db(&self) -> Result<()> {
        if let Err(e) = self.run_migrations() {
            eprintln!("Migration integrity error: {}. Deleting database and starting over.", e);
            fs::remove_file(&self.db_path).expect("Failed to delete corrupted database");
            self.run_migrations().expect("Failed to initialize database after reset");
        }
        Ok(())
    }

    fn run_migrations(&self) -> Result<(), MigrationError> {
        let conn = self.connect().expect("Could not connect to database");
        self.create_migrations_table(&conn).expect("Could not create migrations table");

        let applied_migrations = self.get_applied_migrations(&conn).expect("Could not get applied migrations");
        let migration_files = self.get_migration_files().expect("Could not get migration files");

        // Check for corruption and missing files
        for (name, applied_md5) in &applied_migrations {
            if let Some(file_md5) = migration_files.get(name) {
                if file_md5 != applied_md5 {
                    return Err(MigrationError::Corruption(name.clone()));
                }
            } else {
                return Err(MigrationError::MissingFile(name.clone()));
            }
        }

        // Apply new migrations
        for (name, file_md5) in &migration_files {
            if !applied_migrations.contains_key(name) {
                let path = Path::new("./src/migrations").join(name);
                let sql = fs::read_to_string(&path).expect("Could not read migration file");
                conn.execute_batch(&sql).expect("Could not apply migration");
                self.add_migration_record(&conn, name, &file_md5).expect("Could not add migration record");
                println!("Applied migration: {}", name);
            }
        }
        Ok(())
    }

    fn create_migrations_table(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS migrations (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                md5 TEXT NOT NULL,
                applied_at TEXT DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;
        Ok(())
    }

    fn get_applied_migrations(&self, conn: &Connection) -> Result<HashMap<String, String>> {
        let mut stmt = conn.prepare("SELECT name, md5 FROM migrations")?;
        let mut rows = stmt.query([])?;
        let mut migrations = HashMap::new();
        while let Some(row) = rows.next()? {
            migrations.insert(row.get(0)?, row.get(1)?);
        }
        Ok(migrations)
    }

    fn get_migration_files(&self) -> Result<HashMap<String, String>, std::io::Error> {
        let mut files = HashMap::new();
        let migrations_dir = Path::new("./src/migrations");
        if !migrations_dir.exists() {
            fs::create_dir(migrations_dir)?;
        }
        for entry in fs::read_dir(migrations_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
                let content = fs::read_to_string(&path)?;
                let digest = md5::compute(content.as_bytes());
                files.insert(file_name, format!("{:x}", digest));
            }
        }
        Ok(files)
    }

    fn add_migration_record(&self, conn: &Connection, name: &str, md5: &str) -> Result<()> {
        conn.execute("INSERT INTO migrations (name, md5) VALUES (?, ?)", [name, md5])?;
        Ok(())
    }
    
    pub fn create_project(&self, name: &str) -> Result<Project> {
        let conn = self.connect()?;
        let project_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (project_id, name) VALUES (?, ?)",
            &[&project_id, name],
        )?;
        // Not the most efficient, but it's simple and works.
        self.get_project_by_id(&project_id)
    }

    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT project_id, name, created_at FROM projects")?;
        let mut rows = stmt.query([])?;
        let mut projects = Vec::new();
        while let Some(row) = rows.next()? {
            projects.push(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            });
        }
        Ok(projects)
    }

    pub fn get_project_by_id(&self, project_id: &str) -> Result<Project> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT project_id, name, created_at FROM projects WHERE project_id = ?")?;
        stmt.query_row(&[project_id], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
    }

    pub fn get_project_by_name(&self, name: &str) -> Result<Project> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT project_id, name, created_at FROM projects WHERE name = ?")?;
        stmt.query_row(&[name], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
            })
        })
    }
    
    pub fn link_directory(&self, project_id: &str, path: &Path) -> Result<()> {
        let conn = self.connect()?;
        let path_str = path.to_str().ok_or(rusqlite::Error::InvalidPath(path.to_path_buf()))?;
        conn.execute(
            "INSERT OR REPLACE INTO directory_links (path, project_id) VALUES (?, ?)",
            &[path_str, project_id],
        )?;
        Ok(())
    }

    pub fn get_project_from_path(&self, path: &Path) -> Result<Option<Project>> {
        let conn = self.connect()?;
        let mut current_path = PathBuf::from(path).clean();

        loop {
            let path_str = current_path.to_str().ok_or(rusqlite::Error::InvalidPath(current_path.clone()))?;
            let mut stmt = conn.prepare("SELECT p.project_id, p.name, p.created_at FROM projects p JOIN directory_links dl ON p.project_id = dl.project_id WHERE dl.path = ?")?;
            
            match stmt.query_row(&[path_str], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                })
            }) {
                Ok(project) => return Ok(Some(project)),
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    if !current_path.pop() {
                        return Ok(None);
                    }
                }
                Err(e) => return Err(e),
            }
        }
    }

    pub fn get_or_create_environment(&self, project_id: &str, slug: &str) -> Result<Environment> {
        let conn = self.connect()?;
        match self.get_environment(project_id, slug) {
            Ok(env) => Ok(env),
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                let env_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO environments (env_id, project_id, slug) VALUES (?, ?, ?)",
                    &[&env_id, project_id, slug],
                )?;
                self.get_environment_by_id(&env_id)
            }
            Err(e) => Err(e),
        }
    }

    pub fn get_environment(&self, project_id: &str, slug: &str) -> Result<Environment> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT env_id, project_id, slug FROM environments WHERE project_id = ? AND slug = ?")?;
        stmt.query_row(&[project_id, slug], |row| {
            Ok(Environment {
                id: row.get(0)?,
                project_id: row.get(1)?,
                slug: row.get(2)?,
            })
        })
    }

    fn get_environment_by_id(&self, env_id: &str) -> Result<Environment> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT env_id, project_id, slug FROM environments WHERE env_id = ?")?;
        stmt.query_row(&[env_id], |row| {
            Ok(Environment {
                id: row.get(0)?,
                project_id: row.get(1)?,
                slug: row.get(2)?,
            })
        })
    }

    pub fn set_secret(&self, env_id: &str, key: &str, value: &str) -> Result<()> {
        let conn = self.connect()?;
        let secret_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT OR REPLACE INTO secrets (secret_id, env_id, key, value) VALUES (?, ?, ?, ?)",
            &[&secret_id, env_id, key, value],
        )?;
        Ok(())
    }

    pub fn unset_secret(&self, env_id: &str, key: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "DELETE FROM secrets WHERE env_id = ? AND key = ?",
            &[env_id, key],
        )?;
        Ok(())
    }

    pub fn get_secrets(&self, env_id: &str) -> Result<Vec<Secret>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT key, value FROM secrets WHERE env_id = ?")?;
        let mut rows = stmt.query(&[env_id])?;
        let mut secrets = Vec::new();
        while let Some(row) = rows.next()? {
            secrets.push(Secret {
                key: row.get(0)?,
                value: row.get(1)?,
            });
        }
        Ok(secrets)
    }

    pub fn set_active_environment(&self, project_id: &str, slug: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "INSERT OR REPLACE INTO active_states (project_id, active_slug) VALUES (?, ?)",
            &[project_id, slug],
        )?;
        Ok(())
    }

    pub fn get_active_environment(&self, project_id: &str) -> Result<String> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT active_slug FROM active_states WHERE project_id = ?")?;
        stmt.query_row(&[project_id], |row| row.get(0))
    }

    pub fn list_environments(&self, project_id: &str) -> Result<Vec<String>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare("SELECT slug FROM environments WHERE project_id = ?")?;
        let mut rows = stmt.query(&[project_id])?;
        let mut environments = Vec::new();
        while let Some(row) = rows.next()? {
            environments.push(row.get(0)?);
        }
        Ok(environments)
    }
}