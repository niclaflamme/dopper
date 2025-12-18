use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Integrity(#[from] IntegrityError),
}

#[derive(Error, Debug)]
pub enum IntegrityError {
    #[error("Database is corrupted. MD5 mismatch for migration {0}.")]
    Corruption(String),
    #[error("Missing migration file: {0}")]
    MissingFile(String),
    #[error("Database error: {0}")]
    DbError(#[from] rusqlite::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
