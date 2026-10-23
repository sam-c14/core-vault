pub mod jwt;
pub mod password;

pub use jwt::{Claims, JwtService};
pub use password::{hash_password, verify_password, PasswordError};

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_password_hashing_and_verification() {
        let password = "SuperSecureBankPassword#2026";
        let hash = hash_password(password).unwrap();
        assert_ne!(password, hash);
        assert!(verify_password(password, &hash).is_ok());
        assert!(verify_password("WrongPassword123", &hash).is_err());
    }

    #[test]
    fn test_jwt_generation_and_validation() {
        let secret = "secret_key_for_testing_purposes_only_123456789";
        let jwt_service = JwtService::new(secret);
        let user_id = Uuid::new_v4();
        let tenant_id = Uuid::new_v4();

        let token = jwt_service
            .generate_token(user_id, tenant_id, "OPERATIONS_OFFICER")
            .unwrap();
        assert!(!token.is_empty());

        let claims = jwt_service.verify_token(&token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.tenant_id, tenant_id);
        assert_eq!(claims.role, "OPERATIONS_OFFICER");

        // Tampered token fails validation
        let tampered_token = format!("{}tampered", token);
        assert!(jwt_service.verify_token(&tampered_token).is_err());

        // Token verified with wrong secret fails
        let wrong_service = JwtService::new("different_secret_key_for_testing_only");
        assert!(wrong_service.verify_token(&token).is_err());
    }
}

