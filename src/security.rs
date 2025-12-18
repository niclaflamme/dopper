use keyring::Entry;
use rand::RngCore;
use std::io;
use std::sync::OnceLock;

pub trait KeyProvider {
    fn get_key(&self) -> io::Result<String>;
    fn read_key(&self) -> io::Result<Option<String>>;
}

pub struct KeychainProvider {
    service: String,
    user: String,
    cached_key: OnceLock<String>,
}

impl KeychainProvider {
    pub fn new() -> Self {
        KeychainProvider {
            service: "dopper-cli".to_string(),
            user: "db-encryption-key".to_string(),
            cached_key: OnceLock::new(),
        }
    }

    fn generate_key() -> String {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        hex::encode(key)
    }

    fn entry(&self) -> io::Result<Entry> {
        Entry::new(&self.service, &self.user)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Keyring init error: {}", e)))
    }
}

impl KeyProvider for KeychainProvider {
    fn get_key(&self) -> io::Result<String> {
        if let Some(key) = self.cached_key.get() {
            return Ok(key.clone());
        }

        let entry = self.entry()?;

        match entry.get_password() {
            Ok(key) => {
                let _ = self.cached_key.set(key.clone());
                Ok(key)
            }
            Err(keyring::Error::NoEntry) => {
                let new_key = Self::generate_key();
                entry.set_password(&new_key).map_err(|e| {
                    io::Error::new(io::ErrorKind::Other, format!("Failed to save key: {}", e))
                })?;
                let _ = self.cached_key.set(new_key.clone());
                Ok(new_key)
            }
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to retrieve key: {}", e),
            )),
        }
    }

    fn read_key(&self) -> io::Result<Option<String>> {
        if let Some(key) = self.cached_key.get() {
            return Ok(Some(key.clone()));
        }

        let entry = self.entry()?;

        match entry.get_password() {
            Ok(key) => {
                let _ = self.cached_key.set(key.clone());
                Ok(Some(key))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to retrieve key: {}", e),
            )),
        }
    }
}

pub struct MockKeyProvider {
    key: String,
}

impl MockKeyProvider {
    pub fn new(key: &str) -> Self {
        MockKeyProvider {
            key: key.to_string(),
        }
    }
}

impl KeyProvider for MockKeyProvider {
    fn get_key(&self) -> io::Result<String> {
        Ok(self.key.clone())
    }

    fn read_key(&self) -> io::Result<Option<String>> {
        Ok(Some(self.key.clone()))
    }
}
