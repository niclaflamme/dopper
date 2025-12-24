use anyhow::{anyhow, Result};
use argon2::Argon2;
use base64::{engine::general_purpose, Engine as _};
use rand::rngs::OsRng;
use rand::RngCore;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MasterKey(pub Vec<u8>);

pub fn generate_salt() -> String {
    let mut salt = [0u8; 32];
    OsRng.fill_bytes(&mut salt);
    general_purpose::STANDARD.encode(salt)
}

pub fn derive_key(password: &str, salt: &str) -> Result<MasterKey> {
    let salt_bytes = general_purpose::STANDARD
        .decode(salt)
        .map_err(|e| anyhow!("Invalid salt encoding: {}", e))?;
    let mut output = vec![0u8; 32];
    Argon2::default()
        .hash_password_into(password.as_bytes(), &salt_bytes, &mut output)
        .map_err(|e| anyhow!("Failed to derive key: {}", e))?;
    Ok(MasterKey(output))
}
