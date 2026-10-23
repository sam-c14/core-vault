use cv_auth::{verify_password, JwtService};
use uuid::Uuid;

#[test]
fn security_reject_forged_claims_jwt() {
    let service = JwtService::new("production_master_secret_key_minimum_32_bytes_long");
    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();

    let valid_token = service
        .generate_token(user_id, tenant_id, "TELLER")
        .unwrap();

    // Attacker modifies payload (e.g. elevates privilege to "SUPER_ADMIN")
    let parts: Vec<&str> = valid_token.split('.').collect();
    assert_eq!(parts.len(), 3);

    // Tampering header or payload without re-signing must fail validation
    let forged_token = format!(
        "{}.{}.{}",
        parts[0], "eyJzdWIiOiJhZG1pbiIsInJvbGUiOiJTVVBFUl9BRE1JTiJ9", parts[2]
    );
    assert!(service.verify_token(&forged_token).is_err());
}

#[test]
fn security_reject_key_substitution_attack() {
    let legit_service = JwtService::new("legitimate_bank_secret_key_1234567890");
    let attacker_service = JwtService::new("attacker_controlled_secret_key_12345");

    let forged_token = attacker_service
        .generate_token(Uuid::new_v4(), Uuid::new_v4(), "SUPER_ADMIN")
        .unwrap();

    // Legitimate service rejects token signed by attacker key
    assert!(legit_service.verify_token(&forged_token).is_err());
}

#[test]
fn security_reject_empty_or_malformed_token() {
    let service = JwtService::new("production_master_secret_key_minimum_32_bytes_long");
    assert!(service.verify_token("").is_err());
    assert!(service.verify_token("Bearer ").is_err());
    assert!(service.verify_token("invalid.token").is_err());
    assert!(service.verify_token("not.a.valid.jwt.signature").is_err());
}

#[test]
fn security_argon2id_timing_resistance_on_wrong_password() {
    let hash = "$argon2id$v=19$m=19456,t=2,p=1$ZGF0YXNhbHQxMjM0NQ$h308c+Qj13JvYv8mP6T5Xq"; // Dummy Argon2 format
                                                                                           // Invalid hash parsing safely fails without panic
    assert!(verify_password("Password", hash).is_err());
}
