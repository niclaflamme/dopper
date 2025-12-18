use dopper::db::DbManager;
use dopper::security::KeyProvider;
use std::io;
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

struct CountingKeyProvider {
    key: String,
    count: Arc<Mutex<usize>>,
}

impl CountingKeyProvider {
    fn new(key: &str) -> Self {
        CountingKeyProvider {
            key: key.to_string(),
            count: Arc::new(Mutex::new(0)),
        }
    }
}

impl KeyProvider for CountingKeyProvider {
    fn get_key(&self) -> io::Result<String> {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        Ok(self.key.clone())
    }

    fn read_key(&self) -> io::Result<Option<String>> {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        Ok(Some(self.key.clone()))
    }
}

#[test]
fn test_key_caching() {
    let dir = tempdir().expect("failed to create temp dir");
    let db_path = dir.path().join("dopper_cache_test.db");
    
    // Create a provider and keep a reference to check counts (via shared Arc)
    let key_val = "test-key-cache";
    let provider_struct = CountingKeyProvider::new(key_val);
    let count_ref = provider_struct.count.clone();
    
    let manager = DbManager::new(db_path.clone(), Box::new(provider_struct));
    
    // Initialize DB (plaintext)
    manager.initialize_db().expect("failed to init db");
    
    // Lock it (encrypt it) -> Should trigger get_key once
    manager.lock().expect("failed to lock");
    assert_eq!(*count_ref.lock().unwrap(), 1, "Should have called get_key once for lock");

    // Perform multiple operations that require connection
    // connect() is called internally for each
    
    // 1. Create project
    let project = manager.create_project("p1").expect("failed to create");
    // 2. Create env
    manager.create_environment(&project.id, "staging").expect("failed to create env");
    // 3. Set secret
    // Note: creating env returns an object, so we need env_id
    let env = manager.get_environment(&project.id, "staging").expect("failed to get env");
    manager.set_secret(&env.id, "K", "V").expect("failed to set secret");

    // Check count again. It should STILL be 1 because of caching.
    // If caching wasn't working, create_project, create_environment, get_environment, set_secret 
    // would all trigger calls (some trigger multiple calls).
    assert_eq!(*count_ref.lock().unwrap(), 1, "Should not have called get_key again due to caching");
    
    // Unlock
    manager.unlock().expect("failed to unlock");
    // Unlock calls connect() (uses cache) and then decrypts.
    // So count should remain 1.
    assert_eq!(*count_ref.lock().unwrap(), 1, "Unlock should use cached key");
}
