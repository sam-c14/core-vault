use crate::password::{hash_password, verify_password, PasswordError};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GeneratedCredentials {
    pub client_id: String,
    pub raw_client_secret: String,
    pub client_secret_hash: String,
}

/// Generates a standardized client_id and cryptographically secure Argon2id-hashed client_secret.
pub fn generate_client_credentials(
    tenant_code: &str,
) -> Result<GeneratedCredentials, PasswordError> {
    let clean_code = tenant_code.to_lowercase().replace(['-', ' '], "_");
    let random_suffix = Uuid::new_v4().simple().to_string();
    let client_id = format!("client_{}_{}", clean_code, &random_suffix[..12]);
    let raw_client_secret = format!("sec_live_{}", Uuid::new_v4().simple());

    let client_secret_hash = hash_password(&raw_client_secret)?;

    Ok(GeneratedCredentials {
        client_id,
        raw_client_secret,
        client_secret_hash,
    })
}

/// Verifies a provided client_secret against an Argon2id hash.
pub fn verify_client_secret(provided_secret: &str, stored_hash: &str) -> Result<(), PasswordError> {
    verify_password(provided_secret, stored_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_verify_client_credentials() {
        let creds = generate_client_credentials("WALLET_APP").unwrap();
        assert!(creds.client_id.starts_with("client_wallet_app_"));
        assert!(creds.raw_client_secret.starts_with("sec_live_"));
        assert_ne!(creds.raw_client_secret, creds.client_secret_hash);

        // Verification succeeds with correct secret
        assert!(verify_client_secret(&creds.raw_client_secret, &creds.client_secret_hash).is_ok());

        // Verification fails with wrong secret
        assert!(verify_client_secret("sec_live_wrong_secret", &creds.client_secret_hash).is_err());
    }
}
