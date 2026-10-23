use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PasswordError {
    #[error("Password hashing failed: {0}")]
    HashError(String),
    #[error("Password verification failed")]
    VerificationFailed,
}

pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| PasswordError::HashError(e.to_string()))
}

pub fn verify_password(password: &str, password_hash: &str) -> Result<(), PasswordError> {
    let parsed_hash =
        PasswordHash::new(password_hash).map_err(|e| PasswordError::HashError(e.to_string()))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| PasswordError::VerificationFailed)
}
