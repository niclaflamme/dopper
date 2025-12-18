use dopper::db::DbManager;
use dopper::security::MockKeyProvider;
use rusqlite::Connection;
use std::fs;
use tempfile::tempdir;

fn setup() -> (DbManager, tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("failed to create temp dir");
    let db_path = dir.path().join("dopper.test.db");
    let key_provider = Box::new(MockKeyProvider::new("test-key-123"));
    let manager = DbManager::new(db_path.clone(), key_provider);
    manager.initialize_db().expect("failed to init db");
    (manager, dir, db_path)
}

#[test]
fn test_project_creation_and_listing() {
    let (manager, _dir, _db_path) = setup();

    // List empty
    let projects = manager.list_projects().expect("failed to list projects");
    assert!(projects.is_empty());

    // Create
    let project = manager.create_project("p1").expect("failed to create");
    assert_eq!(project.name, "p1");

    // List one
    let projects = manager.list_projects().expect("failed to list projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "p1");

    // Get by ID
    let fetched = manager.get_project_by_id(&project.id).expect("failed to get by id");
    assert_eq!(fetched.name, "p1");

    // Get by Name
    let fetched_name = manager.get_project_by_name("p1").expect("failed to get by name");
    assert_eq!(fetched_name.id, project.id);
}

#[test]
fn test_secrets_management() {
    let (manager, _dir, _db_path) = setup();
    let project = manager.create_project("p1").expect("failed to create");
    
    // Environment auto-creation
    let env = manager.get_or_create_environment(&project.id, "prod").expect("failed to get env");
    assert_eq!(env.slug, "prod");

    // Set Secret
    manager.set_secret(&env.id, "DB_HOST", "localhost").expect("failed to set");

    // Get Secrets
    let secrets = manager.get_secrets(&env.id).expect("failed to get secrets");
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].key, "DB_HOST");
    assert_eq!(secrets[0].value, "localhost");

    // Update Secret
    manager.set_secret(&env.id, "DB_HOST", "127.0.0.1").expect("failed to update");
    let secrets = manager.get_secrets(&env.id).expect("failed to get secrets");
    assert_eq!(secrets[0].value, "127.0.0.1");

    // Unset Secret
    manager.unset_secret(&env.id, "DB_HOST").expect("failed to unset");
    let secrets = manager.get_secrets(&env.id).expect("failed to get secrets");
    assert!(secrets.is_empty());
}

#[test]
fn test_directory_linking() {
    let (manager, dir, _db_path) = setup();
    let project = manager.create_project("p1").expect("failed to create");

    let app_dir = dir.path().join("app");
    fs::create_dir(&app_dir).expect("failed to create app dir");

    // Link
    manager.link_directory(&project.id, &app_dir).expect("failed to link");

    // Resolve from exact path
    let resolved = manager.get_project_from_path(&app_dir)
        .expect("failed to resolve")
        .expect("should find project");
    assert_eq!(resolved.id, project.id);

    // Resolve from subdir
    let sub_dir = app_dir.join("src/utils");
    fs::create_dir_all(&sub_dir).expect("failed to create subdir");
    let resolved_sub = manager.get_project_from_path(&sub_dir)
        .expect("failed to resolve sub")
        .expect("should find project from sub");
    assert_eq!(resolved_sub.id, project.id);

    // Resolve from non-linked path
    let other_dir = dir.path().join("other");
    fs::create_dir(&other_dir).expect("failed to create other dir");
    let resolved_none = manager.get_project_from_path(&other_dir).expect("failed to resolve none");
    assert!(resolved_none.is_none());
}

#[test]
fn test_migration_corruption_recovery() {
    let (manager, _dir, db_path) = setup();

    // 1. Create some data to verify it gets wiped
    let _project = manager.create_project("wipeme").expect("failed to create project");
    let projects = manager.list_projects().expect("failed to list");
    assert!(!projects.is_empty());

    // 2. Corrupt the database
    // We manually change the stored MD5 of a migration to verify validation logic
    {
        let conn = Connection::open(&db_path).expect("failed to open db for corruption");
        // Update the md5 of the first migration to something 'wrong'
        conn.execute(
            "UPDATE migrations SET md5 = 'BAD_MD5_HASH' WHERE name = '0001_initial_schema.sql'",
            [],
        ).expect("failed to corrupt db");
    }

    // 3. Trigger initialization again. 
    // This should detect corruption (file md5 != DB md5), delete the DB, and re-run migrations.
    manager.initialize_db().expect("failed to re-initialize db");

    // 4. Verify Recovery
    // The DB file should exist (it was recreated)
    assert!(db_path.exists());

    // The project we created should be GONE because the DB was wiped
    let projects_after = manager.list_projects().expect("failed to list after recovery");
    assert!(projects_after.is_empty(), "Database should have been wiped due to corruption");

    // 5. Verify the migration record is correct again
    // We can't access `get_applied_migrations` directly (private), but since `list_projects` worked, the table exists.
    // Let's verify via raw connection that the MD5 is back to normal (not 'BAD_MD5_HASH')
    let conn = Connection::open(&db_path).expect("failed to open db");
    let mut stmt = conn.prepare("SELECT md5 FROM migrations WHERE name = '0001_initial_schema.sql'").expect("prep");
    let md5: String = stmt.query_row([], |row| row.get(0)).expect("query");
    
    assert_ne!(md5, "BAD_MD5_HASH", "MD5 should have been corrected (reset) by recovery");
}

#[test]
fn test_default_environments() {
    let (manager, _dir, _db_path) = setup();
    let project = manager.create_project("defaults_test").expect("failed to create");

    // Check that dev and prod exist
    let envs = manager.list_environments(&project.id).expect("failed to list envs");
    let slugs: Vec<String> = envs.into_iter().map(|e| e.slug).collect();
    assert!(slugs.contains(&"dev".to_string()), "Should contain dev");
    assert!(slugs.contains(&"prod".to_string()), "Should contain prod");

    // Check that default active env is 'dev'
    let active = manager.get_active_environment(&project.id).expect("failed to get active");
    assert_eq!(active, "dev");
}

#[test]
fn test_env_lifecycle() {
    let (manager, _dir, _db_path) = setup();
    let project = manager.create_project("env_lifecycle").expect("failed to create");

    // Create new env
    let yolo = manager.create_environment(&project.id, "yolo").expect("failed to create yolo");
    assert_eq!(yolo.slug, "yolo");

    // Verify it exists
    let envs = manager.list_environments(&project.id).expect("failed to list");
    assert!(envs.iter().any(|e| e.slug == "yolo"));

    // Verify uniqueness error
    manager.create_environment(&project.id, "yolo").expect_err("should fail to create duplicate");

    // Delete env
    manager.delete_environment(&project.id, "yolo").expect("failed to delete yolo");
    
    // Verify it's gone
    let envs_after = manager.list_environments(&project.id).expect("failed to list after delete");
    assert!(!envs_after.iter().any(|e| e.slug == "yolo"));

    // Note: The protection for 'dev'/'prod' deletion is in the CLI layer (main.rs), 
    // not the DbManager, so we can't test that specific restriction here in integration tests 
    // unless we move that logic to DbManager. 
    // However, we can verify we CAN delete them via DbManager if we wanted, 
    // but the requirement is "kill envs, except prod and dev".
    // If the restriction is strictly CLI, this test is sufficient for the DB capability.
}

#[test]
fn test_dump_restore() {
    let (manager, _dir, _db_path) = setup();
    let project = manager.create_project("dump_test").expect("failed to create");
    let env = manager.get_or_create_environment(&project.id, "dev").unwrap();
    manager.set_secret(&env.id, "SECRET_KEY", "ABC").expect("failed to set secret");

    // Dump
    let dump = manager.dump().expect("failed to dump");
    assert_eq!(dump.projects.len(), 1);
    assert_eq!(dump.secrets.len(), 1);

    // Destroy (simulate by creating new manager on empty path or just use restore which wipes)
    // Let's create a NEW manager on a NEW db to verify portability
    let dir2 = tempdir().expect("failed to create temp dir 2");
    let db_path2 = dir2.path().join("dopper.restore.db");
    let key_provider2 = Box::new(MockKeyProvider::new("test-key-456"));
    let manager2 = DbManager::new(db_path2.clone(), key_provider2);
    
    // Restore
    // Note: restore calls initialize_db internally now, so we don't need to call it explicitly on manager2
    manager2.restore(dump).expect("failed to restore");

    // Verify
    let projects = manager2.list_projects().expect("failed to list projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "dump_test");
    assert_eq!(projects[0].id, project.id); // ID should be preserved

    let envs = manager2.list_environments(&projects[0].id).expect("failed to list envs");
    let dev_env = envs.iter().find(|e| e.slug == "dev").expect("dev env missing");
    
    let secrets = manager2.get_secrets(&dev_env.id).expect("failed to get secrets");
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].key, "SECRET_KEY");
    assert_eq!(secrets[0].value, "ABC");
}

#[test]
fn test_project_deletion() {
    let (manager, _dir, _db_path) = setup();
    let project = manager.create_project("to_delete").expect("failed to create");
    
    // Create some state to verify cascade
    let env = manager.get_or_create_environment(&project.id, "dev").unwrap();
    manager.set_secret(&env.id, "K", "V").unwrap();
    manager.set_active_environment(&project.id, "prod").unwrap();

    // Delete
    manager.delete_project("to_delete").expect("failed to delete");

    // Verify
    let projects = manager.list_projects().expect("failed to list");
    assert!(!projects.iter().any(|p| p.name == "to_delete"));

    // Verify orphans are gone (checking secrets via raw query would be ideal, but verifying via API is safe enough if cascading works.
    // However, since we deleted the project, we can't access envs via project_id.
    // If we try to get secrets for the old env_id, it should return empty if deleted.
    let secrets = manager.get_secrets(&env.id).expect("failed to get secrets");
    assert!(secrets.is_empty(), "Secrets should be cascaded delete");
    
    // Verify env is gone
    let env_check = manager.get_environment(&project.id, "dev");
    assert!(env_check.is_err(), "Environment should be gone");
}

#[test]
fn test_secret_isolation() {
    let (manager, _dir, _db_path) = setup();
    let project = manager.create_project("iso_test").expect("failed to create");
    
    let dev = manager.get_or_create_environment(&project.id, "dev").unwrap();
    let prod = manager.get_or_create_environment(&project.id, "prod").unwrap();

    manager.set_secret(&dev.id, "API_KEY", "DEV_KEY").unwrap();
    manager.set_secret(&prod.id, "API_KEY", "PROD_KEY").unwrap();

    let dev_secrets = manager.get_secrets(&dev.id).unwrap();
    let prod_secrets = manager.get_secrets(&prod.id).unwrap();

    assert_eq!(dev_secrets.len(), 1);
    assert_eq!(dev_secrets[0].value, "DEV_KEY");

    assert_eq!(prod_secrets.len(), 1);
    assert_eq!(prod_secrets[0].value, "PROD_KEY");
}

#[test]
fn test_active_env_switching() {
    let (manager, _dir, _db_path) = setup();
    let project = manager.create_project("switch_test").expect("failed to create");
    
    // Default is dev
    assert_eq!(manager.get_active_environment(&project.id).unwrap(), "dev");

    // Switch to prod
    manager.set_active_environment(&project.id, "prod").expect("failed to switch");
    assert_eq!(manager.get_active_environment(&project.id).unwrap(), "prod");

    // Switch to custom
    manager.create_environment(&project.id, "staging").unwrap();
    manager.set_active_environment(&project.id, "staging").unwrap();
    assert_eq!(manager.get_active_environment(&project.id).unwrap(), "staging");
}

#[test]
fn test_encryption_flow() {
    let (manager, _dir, _db_path) = setup();
    
    // Create data
    let project = manager.create_project("lock_test").unwrap();
    let env = manager.get_or_create_environment(&project.id, "dev").unwrap();
    manager.set_secret(&env.id, "S", "Secret").unwrap();

    // Lock
    manager.lock().expect("failed to lock");
    
    // Access should still work (auto-unlock via key provider)
    let secrets = manager.get_secrets(&env.id).unwrap();
    assert_eq!(secrets[0].value, "Secret");

    // Unlock
    manager.unlock().expect("failed to unlock");

    // Access should still work
    let secrets_plain = manager.get_secrets(&env.id).unwrap();
    assert_eq!(secrets_plain[0].value, "Secret");
}

#[test]
fn test_integrity_restores_default_envs() {
    let (manager, _dir, db_path) = setup();
    let project = manager.create_project("integrity_test").expect("failed to create");
    
    // Manually delete 'prod' (simulating corruption/tampering)
    {
        let conn = Connection::open(&db_path).expect("failed to open db");
        conn.execute("DELETE FROM environments WHERE project_id = ? AND slug = 'prod'", [&project.id]).expect("failed to delete prod");
    }

    // Verify it's gone
    let envs = manager.list_environments(&project.id).expect("failed to list");
    assert!(!envs.iter().any(|e| e.slug == "prod"));

    // Trigger integrity check (via initialize_db)
    manager.initialize_db().expect("failed to init db");

    // Verify 'prod' is back
    let envs_after = manager.list_environments(&project.id).expect("failed to list after init");
    assert!(envs_after.iter().any(|e| e.slug == "prod"), "Prod should have been restored");
}