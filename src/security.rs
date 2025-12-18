use keyring::Entry;
use rand::RngCore;
use std::io;

pub trait KeyProvider {
    fn get_key(&self) -> io::Result<String>;
}

pub struct KeychainProvider {
    service: String,
    user: String,
}

impl KeychainProvider {
    pub fn new() -> Self {
        KeychainProvider {
            service: "dopper-cli".to_string(),
            user: "db-encryption-key".to_string(),
        }
    }

    fn generate_key() -> String {
        let mut key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        hex::encode(key)
    }
}

impl KeyProvider for KeychainProvider {
    fn get_key(&self) -> io::Result<String> {
        let entry = Entry::new(&self.service, &self.user).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, format!("Keyring init error: {}", e))
        })?;

        match entry.get_password() {
            Ok(key) => Ok(key),
            Err(keyring::Error::NoEntry) => {
                let new_key = Self::generate_key();
                entry.set_password(&new_key).map_err(|e| {
                    io::Error::new(io::ErrorKind::Other, format!("Failed to save key: {}", e))
                })?;
                Ok(new_key)
            }
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
}
