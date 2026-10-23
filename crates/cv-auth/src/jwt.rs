use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum JwtError {
    #[error("Token creation failed: {0}")]
    CreationError(String),
    #[error("Token validation failed: {0}")]
    ValidationError(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub tenant_id: Uuid,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

pub struct JwtService {
    secret: Vec<u8>,
}

impl JwtService {
    pub fn new(secret: &str) -> Self {
        Self {
            secret: secret.as_bytes().to_vec(),
        }
    }

    pub fn generate_token(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        role: &str,
    ) -> Result<String, JwtError> {
        let now = Utc::now();
        let exp = (now + Duration::minutes(15)).timestamp() as usize;
        let iat = now.timestamp() as usize;

        let claims = Claims {
            sub: user_id,
            tenant_id,
            role: role.to_string(),
            exp,
            iat,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.secret),
        )
        .map_err(|e| JwtError::CreationError(e.to_string()))
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, JwtError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.secret),
            &Validation::default(),
        )
        .map_err(|e| JwtError::ValidationError(e.to_string()))?;

        Ok(token_data.claims)
    }
}
