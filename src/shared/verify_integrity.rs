use std::collections::HashMap;
use std::fs;
use std::path::Path;

use md5;
use rusqlite::Connection;
use uuid::Uuid;

use crate::types::IntegrityError;

/// Verifies the integrity of applied migrations against the file system.
/// Returns the map of all available migration files (name -> md5) if successful.
pub fn verify_integrity(conn: &Connection) -> Result<HashMap<String, String>, IntegrityError> {
    let applied_migrations = get_applied_migrations(conn)?;
    let migration_files = get_migration_files()?;

    // Check for corruption and missing files
    for (name, applied_md5) in &applied_migrations {
        if let Some(file_md5) = migration_files.get(name) {
            if file_md5 != applied_md5 {
                return Err(IntegrityError::Corruption(name.clone()));
            }
        } else {
            return Err(IntegrityError::MissingFile(name.clone()));
        }
    }

    Ok(migration_files)
}

pub fn ensure_default_environments(conn: &Connection) -> Result<(), IntegrityError> {
    let mut stmt = conn.prepare("SELECT project_id FROM projects")?;
    let project_ids: Vec<String> = stmt
        .query_map([], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    for pid in project_ids {
        ensure_env(conn, &pid, "dev")?;
        ensure_env(conn, &pid, "prod")?;
    }
    Ok(())
}

pub fn get_applied_migrations(
    conn: &Connection,
) -> Result<HashMap<String, String>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT name, md5 FROM migrations")?;
    let mut rows = stmt.query([])?;
    let mut migrations = HashMap::new();
    while let Some(row) = rows.next()? {
        migrations.insert(row.get(0)?, row.get(1)?);
    }
    Ok(migrations)
}

fn ensure_env(conn: &Connection, project_id: &str, slug: &str) -> Result<(), IntegrityError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM environments WHERE project_id = ? AND slug = ?)",
        [project_id, slug],
        |row| row.get(0),
    )?;

    if !exists {
        let env_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO environments (env_id, project_id, slug) VALUES (?, ?, ?)",
            &[&env_id, project_id, slug],
        )?;
        log::debug!(
            "Integrity: Restored missing '{}' environment for project '{}'",
            slug,
            project_id
        );
    }
    Ok(())
}

fn get_migration_files() -> Result<HashMap<String, String>, std::io::Error> {
    let mut files = HashMap::new();
    let migrations_dir = Path::new("./src/migrations");

    // Ensure directory exists (mostly for clean state in new checkouts)
    if !migrations_dir.exists() {
        fs::create_dir_all(migrations_dir)?;
    }

    for entry in fs::read_dir(migrations_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            // Ignore non-sql files if any? For now assume all files are migrations.
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(".sql") {
                    let content = fs::read_to_string(&path)?;
                    let digest = md5::compute(content.as_bytes());
                    files.insert(name.to_string(), format!("{:x}", digest));
                }
            }
        }
    }
    Ok(files)
}
