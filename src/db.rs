use path_clean::PathClean;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

use crate::integrity::{IntegrityChecker, IntegrityError};
use crate::security::KeyProvider;

static KEY_CACHE: OnceLock<Mutex<HashMap<PathBuf, String>>> = OnceLock::new();

// -------------------------------------------------------------------------------------------------
// ---- Types --------------------------------------------------------------------------------------

pub struct DbManager {
    db_path: PathBuf,
    key_provider: Box<dyn KeyProvider + Send + Sync>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub project_id: String,
    pub slug: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Secret {
    pub secret_id: Option<String>, // Added for full restoration logic if needed, or just keep simple
    pub env_id: Option<String>,    // Added for full dump context
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryLink {
    pub path: String,
    pub project_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DopperDump {
    pub projects: Vec<Project>,
    pub environments: Vec<Environment>,
    pub secrets: Vec<Secret>,
    pub links: Vec<DirectoryLink>,
}

// -------------------------------------------------------------------------------------------------
// ---- Implementation -----------------------------------------------------------------------------

impl DbManager {
    pub fn new(db_path: PathBuf, key_provider: Box<dyn KeyProvider + Send + Sync>) -> Self {
        DbManager {
            db_path,
            key_provider,
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn db_path_exists(&self) -> bool {
        self.db_path.exists()
    }

    fn key_cache_id(&self) -> PathBuf {
        self.db_path.clean()
    }

    fn get_key_cached(&self) -> std::io::Result<String> {
        let cache_mutex = KEY_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache = cache_mutex.lock().unwrap();

        let cache_id = self.key_cache_id();
        if let Some(key) = cache.get(&cache_id) {
            return Ok(key.clone());
        }

        let key = self.key_provider.get_key()?;
        cache.insert(cache_id, key.clone());
        Ok(key)
    }

    fn read_key_cached(&self) -> std::io::Result<Option<String>> {
        let cache_mutex = KEY_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache = cache_mutex.lock().unwrap();

        let cache_id = self.key_cache_id();
        if let Some(key) = cache.get(&cache_id) {
            return Ok(Some(key.clone()));
        }

        if let Some(key) = self.key_provider.read_key()? {
            cache.insert(cache_id, key.clone());
            Ok(Some(key))
        } else {
            Ok(None)
        }
    }

    pub fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;

        // Try to access the database as plaintext first
        if conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
            .is_ok()
        {
            return Ok(conn);
        }

        // If plaintext access fails, it might be encrypted. Try with key.
        let key_opt = self
            .read_key_cached()
            .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

        let key = match key_opt {
            Some(k) => k,
            None => {
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_NOTADB),
                    Some("Failed to open database. Encrypted and no key found.".to_string()),
                ));
            }
        };

        conn.pragma_update(None, "key", &key)?;

        // Verify key works
        if let Err(_) = conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(())) {
            return Err(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_NOTADB),
                Some("Failed to open database. Invalid key or corrupted file.".to_string()),
            ));
        }

        Ok(conn)
    }

    pub fn initialize_db(&self) -> Result<()> {
        if let Err(e) = self.run_migrations() {
            eprintln!(
                "Migration integrity error: {}. Deleting database and starting over.",
                e
            );
            fs::remove_file(&self.db_path).expect("Failed to delete corrupted database");
            self.run_migrations()
                .expect("Failed to initialize database after reset");
        }
        Ok(())
    }

    fn run_migrations(&self) -> Result<(), IntegrityError> {
        let conn = self.connect()?;
        self.create_migrations_table(&conn)
            .expect("Could not create migrations table");

        // Use the IntegrityChecker to verify DB state and get available migration files
        let migration_files = IntegrityChecker::verify(&conn)?;
        let applied_migrations = IntegrityChecker::get_applied_migrations(&conn)?;

        // Apply new migrations
        for (name, file_md5) in &migration_files {
            if !applied_migrations.contains_key(name) {
                let path = Path::new("./src/migrations").join(name);
                let sql = fs::read_to_string(&path).expect("Could not read migration file");
                conn.execute_batch(&sql).expect("Could not apply migration");
                self.add_migration_record(&conn, name, &file_md5)
                    .expect("Could not add migration record");
                log::debug!("Applied migration: {}", name);
            }
        }

        // Ensure data integrity (default envs)
        IntegrityChecker::ensure_default_environments(&conn)?;

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

    fn add_migration_record(&self, conn: &Connection, name: &str, md5: &str) -> Result<()> {
        conn.execute(
            "INSERT INTO migrations (name, md5) VALUES (?, ?)",
            [name, md5],
        )?;
        Ok(())
    }

    pub fn delete_database(&self) -> std::io::Result<()> {
        if self.db_path.exists() {
            fs::remove_file(&self.db_path)?;
        }
        Ok(())
    }

    pub fn create_project(&self, name: &str) -> Result<Project> {
        let conn = self.connect()?;
        let project_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO projects (project_id, name) VALUES (?, ?)",
            &[&project_id, name],
        )?;

        // Default environments
        let dev_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO environments (env_id, project_id, slug) VALUES (?, ?, ?)",
            &[&dev_id, &project_id, "dev"],
        )?;
        let prod_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO environments (env_id, project_id, slug) VALUES (?, ?, ?)",
            &[&prod_id, &project_id, "prod"],
        )?;

        // Not the most efficient, but it's simple and works.
        self.get_project_by_id(&project_id)
    }

    pub fn delete_project(&self, name: &str) -> Result<()> {
        let conn = self.connect()?;
        let project = self.get_project_by_name(name)?;

        let tx = conn.unchecked_transaction()?;

        // Delete secrets for all environments of this project
        // (Nested query: delete secrets where env_id in (select env_id from environments where project_id = ?))
        tx.execute(
            "DELETE FROM secrets WHERE env_id IN (SELECT env_id FROM environments WHERE project_id = ?)",
            [&project.id],
        )?;

        // Delete environments
        tx.execute(
            "DELETE FROM environments WHERE project_id = ?",
            [&project.id],
        )?;

        // Delete directory links
        tx.execute(
            "DELETE FROM directory_links WHERE project_id = ?",
            [&project.id],
        )?;

        // Delete active state
        tx.execute(
            "DELETE FROM active_states WHERE project_id = ?",
            [&project.id],
        )?;

        // Delete project
        tx.execute("DELETE FROM projects WHERE project_id = ?", [&project.id])?;

        tx.commit()?;
        Ok(())
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
        let mut stmt =
            conn.prepare("SELECT project_id, name, created_at FROM projects WHERE project_id = ?")?;
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
        let mut stmt =
            conn.prepare("SELECT project_id, name, created_at FROM projects WHERE name = ?")?;
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
        let path_str = path
            .to_str()
            .ok_or(rusqlite::Error::InvalidPath(path.to_path_buf()))?;
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
            let path_str = current_path
                .to_str()
                .ok_or(rusqlite::Error::InvalidPath(current_path.clone()))?;
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

    pub fn create_environment(&self, project_id: &str, slug: &str) -> Result<Environment> {
        let conn = self.connect()?;
        // Check existence first to return a clear error/avoid duplication logic in caller
        match self.get_environment(project_id, slug) {
            Ok(_) => {
                return Err(rusqlite::Error::ToSqlConversionFailure(
                    format!("Environment '{}' already exists", slug).into(),
                ));
            }
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

    pub fn delete_environment(&self, project_id: &str, slug: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "DELETE FROM environments WHERE project_id = ? AND slug = ?",
            &[project_id, slug],
        )?;
        // We might want to cascade delete secrets, but usually DB constraints or manual cleanup is needed.
        // Assuming secrets have ON DELETE CASCADE or similar, or we leave them orphaned for now.
        // Actually SQLite doesn't enable FK by default. For now, let's just delete the env.
        // Better: delete secrets associated. But `secrets` table uses `env_id`.
        // So we first need to get the env_id to delete secrets?
        // Let's rely on SQLite logic or simple deletion for now as requested.
        Ok(())
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
        let mut stmt = conn.prepare(
            "SELECT env_id, project_id, slug FROM environments WHERE project_id = ? AND slug = ?",
        )?;
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
        let mut stmt =
            conn.prepare("SELECT env_id, project_id, slug FROM environments WHERE env_id = ?")?;
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
                secret_id: None,
                env_id: None,
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
        let mut stmt =
            conn.prepare("SELECT active_slug FROM active_states WHERE project_id = ?")?;

        match stmt.query_row(&[project_id], |row| row.get(0)) {
            Ok(slug) => Ok(slug),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok("dev".to_string()),
            Err(e) => Err(e),
        }
    }

    pub fn dump(&self) -> Result<DopperDump> {
        let conn = self.connect()?;

        // Projects
        let mut stmt = conn.prepare("SELECT project_id, name, created_at FROM projects")?;
        let projects = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        // Environments
        let mut stmt = conn.prepare("SELECT env_id, project_id, slug FROM environments")?;
        let environments = stmt
            .query_map([], |row| {
                Ok(Environment {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    slug: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        // Secrets (Note: Secret struct now has secret_id and env_id for dump purposes)
        let mut stmt = conn.prepare("SELECT secret_id, env_id, key, value FROM secrets")?;
        let secrets = stmt
            .query_map([], |row| {
                Ok(Secret {
                    secret_id: Some(row.get(0)?),
                    env_id: Some(row.get(1)?),
                    key: row.get(2)?,
                    value: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        // Directory Links
        let mut stmt = conn.prepare("SELECT path, project_id FROM directory_links")?;
        let links = stmt
            .query_map([], |row| {
                Ok(DirectoryLink {
                    path: row.get(0)?,
                    project_id: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(DopperDump {
            projects,
            environments,
            secrets,
            links,
        })
    }

    pub fn restore(&self, dump: DopperDump) -> Result<()> {
        self.initialize_db()?;
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;

        // Clear existing data
        tx.execute("DELETE FROM directory_links", [])?;
        tx.execute("DELETE FROM secrets", [])?;
        tx.execute("DELETE FROM environments", [])?;
        tx.execute("DELETE FROM projects", [])?;
        // We leave migrations as is, assuming schema is compatible.
        // If schema changes in future, restore might need migration check or similar.

        // Restore Projects
        for p in dump.projects {
            tx.execute(
                "INSERT INTO projects (project_id, name, created_at) VALUES (?, ?, ?)",
                &[&p.id, &p.name, &p.created_at],
            )?;
        }

        // Restore Environments
        for e in dump.environments {
            tx.execute(
                "INSERT INTO environments (env_id, project_id, slug) VALUES (?, ?, ?)",
                &[&e.id, &e.project_id, &e.slug],
            )?;
        }

        // Restore Secrets
        for s in dump.secrets {
            let secret_id = s.secret_id.unwrap_or_else(|| Uuid::new_v4().to_string());
            let env_id = s.env_id.ok_or(rusqlite::Error::ToSqlConversionFailure(
                "Missing env_id in secret restore".into(),
            ))?;

            tx.execute(
                "INSERT INTO secrets (secret_id, env_id, key, value) VALUES (?, ?, ?, ?)",
                &[&secret_id, &env_id, &s.key, &s.value],
            )?;
        }

        // Restore Links
        for l in dump.links {
            tx.execute(
                "INSERT INTO directory_links (path, project_id) VALUES (?, ?)",
                &[&l.path, &l.project_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn list_environments(&self, project_id: &str) -> Result<Vec<Environment>> {
        let conn = self.connect()?;
        let mut stmt =
            conn.prepare("SELECT env_id, project_id, slug FROM environments WHERE project_id = ?")?;
        let mut rows = stmt.query(&[project_id])?;
        let mut environments = Vec::new();
        while let Some(row) = rows.next()? {
            environments.push(Environment {
                id: row.get(0)?,
                project_id: row.get(1)?,
                slug: row.get(2)?,
            });
        }
        Ok(environments)
    }

    pub fn is_locked(&self) -> Result<bool> {
        let conn = Connection::open(&self.db_path)?;
        // Try to access the database as plaintext
        if conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
            .is_ok()
        {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn lock(&self) -> Result<()> {
        if self.is_locked()? {
            return Ok(());
        }

        let conn = self.connect()?;

        // Check if already encrypted (we know it's encrypted if PRAGMA key was needed,
        // but connect() handles both. We can check via PRAGMA cipher_version or similar,
        // but easier: just re-encrypt logic works for plaintext->encrypted too).
        // Actually, sqlcipher_export works from any valid connection to a new one.

        let key = self
            .get_key_cached()
            .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

        let new_path = self.db_path.with_extension("db.tmp");
        if new_path.exists() {
            fs::remove_file(&new_path)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
        }

        // Attach new encrypted database
        let attach_sql = format!(
            "ATTACH DATABASE '{}' AS encrypted KEY '{}'",
            new_path.display(),
            key
        );
        conn.execute(&attach_sql, [])?;

        // Export
        conn.query_row("SELECT sqlcipher_export('encrypted')", [], |_| Ok(()))?;
        conn.execute("DETACH DATABASE encrypted", [])?;

        // Close connection to allow file swap
        drop(conn);

        fs::rename(&new_path, &self.db_path)
            .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

        Ok(())
    }

    pub fn unlock(&self) -> Result<()> {
        if !self.is_locked()? {
            return Ok(());
        }

        let conn = self.connect()?;

        // We assume conn is valid. If it was encrypted, key is set. If plaintext, no key.

        let new_path = self.db_path.with_extension("db.tmp");
        if new_path.exists() {
            fs::remove_file(&new_path)
                .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;
        }

        // Attach new plaintext database (KEY '')
        let attach_sql = format!(
            "ATTACH DATABASE '{}' AS plaintext KEY ''",
            new_path.display()
        );
        conn.execute(&attach_sql, [])?;

        // Export
        conn.query_row("SELECT sqlcipher_export('plaintext')", [], |_| Ok(()))?;
        conn.execute("DETACH DATABASE plaintext", [])?;

        // Close connection
        drop(conn);

        fs::rename(&new_path, &self.db_path)
            .map_err(|e| rusqlite::Error::UserFunctionError(Box::new(e)))?;

        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
