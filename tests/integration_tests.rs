use std::fs;
use std::path::Path;

use rusqlite::Connection;
use tempfile::tempdir;

use dopper::security::MasterKey;
use dopper::shared::db_manager::DbManager;
use dopper::shared::get_effective_env;

// -------------------------------------------------------------------------------------------------
// ---- Setup --------------------------------------------------------------------------------------

fn setup() -> (DbManager, MasterKey, tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().expect("failed to create temp dir");
    let db_path = dir.path().join("dopper.test.db");
    let manager = DbManager::new(db_path.clone());
    let master_key = MasterKey(vec![0x11; 32]);
    manager
        .initialize_db(&master_key)
        .expect("failed to init db");
    (manager, master_key, dir, db_path)
}

fn open_conn_with_key(path: &Path, master_key: &MasterKey) -> Connection {
    let conn = Connection::open(path).expect("failed to open db");
    let key_hex = hex::encode(&master_key.0);
    let pragma = format!("PRAGMA key = 'x''{}'''", key_hex);
    conn.execute_batch(&pragma)
        .expect("failed to set encryption key");
    conn
}

// -------------------------------------------------------------------------------------------------
// ---- Tests --------------------------------------------------------------------------------------

#[test]
fn test_project_creation_and_listing() {
    let (manager, master_key, _dir, _db_path) = setup();

    // List empty
    let projects = manager
        .list_projects(&master_key)
        .expect("failed to list projects");
    assert!(projects.is_empty());

    // Create
    let project = manager
        .create_project(&master_key, "p1")
        .expect("failed to create");
    assert_eq!(project.name, "p1");

    // List one
    let projects = manager
        .list_projects(&master_key)
        .expect("failed to list projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "p1");

    // Get by ID
    let fetched = manager
        .get_project_by_id(&master_key, &project.id)
        .expect("failed to get by id");
    assert_eq!(fetched.name, "p1");

    // Get by Name
    let fetched_name = manager
        .get_project_by_name(&master_key, "p1")
        .expect("failed to get by name");
    assert_eq!(fetched_name.id, project.id);
}

#[test]
fn test_secrets_management() {
    let (manager, master_key, _dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "p1")
        .expect("failed to create");

    // Environment auto-creation
    let env = manager
        .get_or_create_environment(&master_key, &project.id, "prod")
        .expect("failed to get env");
    assert_eq!(env.slug, "prod");

    // Set Secret
    manager
        .set_secret(&master_key, &env.id, "DB_HOST", "localhost")
        .expect("failed to set");

    // Get Secrets
    let secrets = manager
        .get_secrets(&master_key, &env.id)
        .expect("failed to get secrets");
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].key, "DB_HOST");
    assert_eq!(secrets[0].value, "localhost");

    // Update Secret
    manager
        .set_secret(&master_key, &env.id, "DB_HOST", "127.0.0.1")
        .expect("failed to update");
    let secrets = manager
        .get_secrets(&master_key, &env.id)
        .expect("failed to get secrets");
    assert_eq!(secrets[0].value, "127.0.0.1");

    // Unset Secret
    manager
        .unset_secret(&master_key, &env.id, "DB_HOST")
        .expect("failed to unset");
    let secrets = manager
        .get_secrets(&master_key, &env.id)
        .expect("failed to get secrets");
    assert!(secrets.is_empty());
}

#[test]
fn test_directory_linking() {
    let (manager, master_key, dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "p1")
        .expect("failed to create");

    let app_dir = dir.path().join("app");
    fs::create_dir(&app_dir).expect("failed to create app dir");

    // Link
    manager
        .link_directory(&master_key, &project.id, &app_dir)
        .expect("failed to link");

    // Resolve from exact path
    let resolved = manager
        .get_project_from_path(&master_key, &app_dir)
        .expect("failed to resolve")
        .expect("should find project");
    assert_eq!(resolved.id, project.id);

    // Resolve from subdir
    let sub_dir = app_dir.join("src/utils");
    fs::create_dir_all(&sub_dir).expect("failed to create subdir");
    let resolved_sub = manager
        .get_project_from_path(&master_key, &sub_dir)
        .expect("failed to resolve sub")
        .expect("should find project from sub");
    assert_eq!(resolved_sub.id, project.id);

    // Resolve from non-linked path
    let other_dir = dir.path().join("other");
    fs::create_dir(&other_dir).expect("failed to create other dir");
    let resolved_none = manager
        .get_project_from_path(&master_key, &other_dir)
        .expect("failed to resolve none");
    assert!(resolved_none.is_none());
}

#[test]
fn test_migration_corruption_recovery() {
    let (manager, master_key, _dir, db_path) = setup();

    // 1. Create some data to verify it gets wiped
    let _project = manager
        .create_project(&master_key, "wipeme")
        .expect("failed to create project");
    let projects = manager
        .list_projects(&master_key)
        .expect("failed to list");
    assert!(!projects.is_empty());

    // 2. Corrupt the database
    // We manually change the stored MD5 of a migration to verify validation logic
    {
        let conn = open_conn_with_key(&db_path, &master_key);
        // Update the md5 of the first migration to something 'wrong'
        conn.execute(
            "UPDATE migrations SET md5 = 'BAD_MD5_HASH' WHERE name = '0001_initial_schema.sql'",
            [],
        )
        .expect("failed to corrupt db");
    }

    // 3. Trigger initialization again.
    // This should detect corruption (file md5 != DB md5), delete the DB, and re-run migrations.
    manager
        .initialize_db(&master_key)
        .expect("failed to re-initialize db");

    // 4. Verify Recovery
    // The DB file should exist (it was recreated)
    assert!(db_path.exists());

    // The project we created should be GONE because the DB was wiped
    let projects_after = manager
        .list_projects(&master_key)
        .expect("failed to list after recovery");
    assert!(
        projects_after.is_empty(),
        "Database should have been wiped due to corruption"
    );

    // 5. Verify the migration record is correct again
    // We can't access `get_applied_migrations` directly (private), but since `list_projects` worked, the table exists.
    // Let's verify via raw connection that the MD5 is back to normal (not 'BAD_MD5_HASH')
    let conn = open_conn_with_key(&db_path, &master_key);
    let mut stmt = conn
        .prepare("SELECT md5 FROM migrations WHERE name = '0001_initial_schema.sql'")
        .expect("prep");
    let md5: String = stmt.query_row([], |row| row.get(0)).expect("query");

    assert_ne!(
        md5, "BAD_MD5_HASH",
        "MD5 should have been corrected (reset) by recovery"
    );
}

#[test]
fn test_default_environments() {
    let (manager, master_key, _dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "defaults_test")
        .expect("failed to create");

    // Check that dev and prod exist
    let envs = manager
        .list_environments(&master_key, &project.id)
        .expect("failed to list envs");
    let slugs: Vec<String> = envs.into_iter().map(|e| e.slug).collect();
    assert!(slugs.contains(&"dev".to_string()), "Should contain dev");
    assert!(slugs.contains(&"prod".to_string()), "Should contain prod");

    // Check that default active env is 'dev'
    let active = manager
        .get_active_environment(&master_key, &project.id)
        .expect("failed to get active");
    assert_eq!(active, "dev");
}

#[test]
fn test_env_lifecycle() {
    let (manager, master_key, _dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "env_lifecycle")
        .expect("failed to create");

    // Create new env
    let yolo = manager
        .create_environment(&master_key, &project.id, "yolo")
        .expect("failed to create yolo");
    assert_eq!(yolo.slug, "yolo");

    // Verify it exists
    let envs = manager
        .list_environments(&master_key, &project.id)
        .expect("failed to list");
    assert!(envs.iter().any(|e| e.slug == "yolo"));

    // Verify uniqueness error
    manager
        .create_environment(&master_key, &project.id, "yolo")
        .expect_err("should fail to create duplicate");

    // Delete env
    manager
        .delete_environment(&master_key, &project.id, "yolo")
        .expect("failed to delete yolo");

    // Verify it's gone
    let envs_after = manager
        .list_environments(&master_key, &project.id)
        .expect("failed to list after delete");
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
    let (manager, master_key, _dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "dump_test")
        .expect("failed to create");
    let env = manager
        .get_or_create_environment(&master_key, &project.id, "dev")
        .unwrap();
    manager
        .set_secret(&master_key, &env.id, "SECRET_KEY", "ABC")
        .expect("failed to set secret");

    // Dump
    let dump = manager.dump(&master_key).expect("failed to dump");
    assert_eq!(dump.projects.len(), 1);
    assert_eq!(dump.secrets.len(), 1);

    // Destroy (simulate by creating new manager on empty path or just use restore which wipes)
    // Let's create a NEW manager on a NEW db to verify portability
    let dir2 = tempdir().expect("failed to create temp dir 2");
    let db_path2 = dir2.path().join("dopper.restore.db");
    let manager2 = DbManager::new(db_path2.clone());
    let master_key2 = MasterKey(vec![0x22; 32]);

    // Restore
    // Note: restore calls initialize_db internally now, so we don't need to call it explicitly on manager2
    manager2
        .restore(&master_key2, dump)
        .expect("failed to restore");

    // Verify
    let projects = manager2
        .list_projects(&master_key2)
        .expect("failed to list projects");
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "dump_test");
    assert_eq!(projects[0].id, project.id); // ID should be preserved

    let envs = manager2
        .list_environments(&master_key2, &projects[0].id)
        .expect("failed to list envs");
    let dev_env = envs
        .iter()
        .find(|e| e.slug == "dev")
        .expect("dev env missing");

    let secrets = manager2
        .get_secrets(&master_key2, &dev_env.id)
        .expect("failed to get secrets");
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].key, "SECRET_KEY");
    assert_eq!(secrets[0].value, "ABC");
}

#[test]
fn test_project_deletion() {
    let (manager, master_key, _dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "to_delete")
        .expect("failed to create");

    // Create some state to verify cascade
    let env = manager
        .get_or_create_environment(&master_key, &project.id, "dev")
        .unwrap();
    manager
        .set_secret(&master_key, &env.id, "K", "V")
        .unwrap();
    manager
        .set_active_environment(&master_key, &project.id, "prod")
        .unwrap();

    // Delete
    manager
        .delete_project(&master_key, "to_delete")
        .expect("failed to delete");

    // Verify
    let projects = manager
        .list_projects(&master_key)
        .expect("failed to list");
    assert!(!projects.iter().any(|p| p.name == "to_delete"));

    // Verify orphans are gone (checking secrets via raw query would be ideal, but verifying via API is safe enough if cascading works.
    // However, since we deleted the project, we can't access envs via project_id.
    // If we try to get secrets for the old env_id, it should return empty if deleted.
    let secrets = manager
        .get_secrets(&master_key, &env.id)
        .expect("failed to get secrets");
    assert!(secrets.is_empty(), "Secrets should be cascaded delete");

    // Verify env is gone
    let env_check = manager.get_environment(&master_key, &project.id, "dev");
    assert!(env_check.is_err(), "Environment should be gone");
}

#[test]
fn test_secret_isolation() {
    let (manager, master_key, _dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "iso_test")
        .expect("failed to create");

    let dev = manager
        .get_or_create_environment(&master_key, &project.id, "dev")
        .unwrap();
    let prod = manager
        .get_or_create_environment(&master_key, &project.id, "prod")
        .unwrap();

    manager
        .set_secret(&master_key, &dev.id, "API_KEY", "DEV_KEY")
        .unwrap();
    manager
        .set_secret(&master_key, &prod.id, "API_KEY", "PROD_KEY")
        .unwrap();

    let dev_secrets = manager.get_secrets(&master_key, &dev.id).unwrap();
    let prod_secrets = manager.get_secrets(&master_key, &prod.id).unwrap();

    assert_eq!(dev_secrets.len(), 1);
    assert_eq!(dev_secrets[0].value, "DEV_KEY");

    assert_eq!(prod_secrets.len(), 1);
    assert_eq!(prod_secrets[0].value, "PROD_KEY");
}

#[test]
fn test_active_env_switching() {
    let (manager, master_key, _dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "switch_test")
        .expect("failed to create");

    // Default is dev
    assert_eq!(
        manager
            .get_active_environment(&master_key, &project.id)
            .unwrap(),
        "dev"
    );

    // Switch to prod
    manager
        .set_active_environment(&master_key, &project.id, "prod")
        .expect("failed to switch");
    assert_eq!(
        manager
            .get_active_environment(&master_key, &project.id)
            .unwrap(),
        "prod"
    );

    // Switch to custom
    manager
        .create_environment(&master_key, &project.id, "staging")
        .unwrap();
    manager
        .set_active_environment(&master_key, &project.id, "staging")
        .unwrap();
    assert_eq!(
        manager
            .get_active_environment(&master_key, &project.id)
            .unwrap(),
        "staging"
    );
}

#[test]
fn test_integrity_restores_default_envs() {
    let (manager, master_key, _dir, db_path) = setup();
    let project = manager
        .create_project(&master_key, "integrity_test")
        .expect("failed to create");

    // Manually delete 'prod' (simulating corruption/tampering)
    {
        let conn = open_conn_with_key(&db_path, &master_key);
        conn.execute(
            "DELETE FROM environments WHERE project_id = ? AND slug = 'prod'",
            [&project.id],
        )
        .expect("failed to delete prod");
    }

    // Verify it's gone
    let envs = manager
        .list_environments(&master_key, &project.id)
        .expect("failed to list");
    assert!(!envs.iter().any(|e| e.slug == "prod"));

    // Trigger integrity check (via initialize_db)
    manager
        .initialize_db(&master_key)
        .expect("failed to init db");

    // Verify 'prod' is back
    let envs_after = manager
        .list_environments(&master_key, &project.id)
        .expect("failed to list after init");
    assert!(
        envs_after.iter().any(|e| e.slug == "prod"),
        "Prod should have been restored"
    );
}

#[test]
fn test_print_retrieval() {
    let (manager, master_key, _dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "print_test")
        .expect("failed to create");

    let env = manager
        .get_or_create_environment(&master_key, &project.id, "dev")
        .unwrap();

    manager
        .set_secret(&master_key, &env.id, "TEST_KEY", "TEST_VALUE")
        .expect("failed to set");
    manager
        .set_secret(&master_key, &env.id, "ANOTHER_KEY", "12345")
        .expect("failed to set");

    let secrets = manager
        .get_secrets(&master_key, &env.id)
        .expect("failed to get secrets");

    // We can't guarantee order from DB usually, so we check existence
    assert!(
        secrets
            .iter()
            .any(|s| s.key == "TEST_KEY" && s.value == "TEST_VALUE")
    );
    assert!(
        secrets
            .iter()
            .any(|s| s.key == "ANOTHER_KEY" && s.value == "12345")
    );
    assert_eq!(secrets.len(), 2);
}

#[test]
fn test_get_effective_env_injection() {
    let (manager, master_key, _dir, _db_path) = setup();
    let project = manager
        .create_project(&master_key, "effective_env_test")
        .expect("failed to create");
    let env_slug = "dev";

    // We don't set any secrets, just check for system ones
    manager
        .get_or_create_environment(&master_key, &project.id, env_slug)
        .unwrap();

    let env_vars = get_effective_env(&manager, &master_key, &project, env_slug)
        .expect("failed to get effective env");

    assert!(
        env_vars
            .iter()
            .any(|(k, v)| k == "DOPPER_PROJECT_ID" && v == &project.id)
    );
    assert!(
        env_vars
            .iter()
            .any(|(k, v)| k == "DOPPER_ENV" && v == env_slug)
    );
}

// -------------------------------------------------------------------------------------------------
// -------------------------------------------------------------------------------------------------
