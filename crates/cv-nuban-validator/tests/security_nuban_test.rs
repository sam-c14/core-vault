use cv_nuban_validator::{generate_nuban, validate_nuban, NubanError};

#[test]
fn security_reject_non_numeric_injection_in_nuban() {
    let injections = [
        "058123456'",
        "058123456a",
        "058 234560",
        "058-234560",
        "'; DROP--",
    ];

    for attack in injections {
        let res = validate_nuban("058", attack);
        assert!(res.is_err(), "Expected attack string '{}' to be rejected", attack);
    }
}

#[test]
fn security_detect_all_single_digit_alterations() {
    let valid_nuban = generate_nuban("058", 345678).unwrap();
    assert!(validate_nuban("058", &valid_nuban).is_ok());

    // Tampering any digit in the account number must be caught by checksum
    for pos in 0..10 {
        let original_char = valid_nuban.chars().nth(pos).unwrap();
        for altered_digit in 0..10 {
            let altered_char = char::from_digit(altered_digit, 10).unwrap();
            if altered_char != original_char {
                let mut tampered = valid_nuban.clone();
                tampered.replace_range(pos..pos + 1, &altered_char.to_string());
                let result = validate_nuban("058", &tampered);
                assert!(
                    result.is_err(),
                    "Failed to catch single-digit modification at position {}: altered {} to {}",
                    pos,
                    original_char,
                    altered_char
                );
            }
        }
    }
}

#[test]
fn security_reject_bank_code_spoofing() {
    // Account generated for bank code 058 (GTBank)
    let gtbank_nuban = generate_nuban("058", 123456).unwrap();

    // Attacker claims this account belongs to bank code 044 (Access Bank)
    let res = validate_nuban("044", &gtbank_nuban);
    assert!(
        matches!(res, Err(NubanError::BankCodeMismatch { .. })),
        "Bank code spoofing was not detected by check digit validation"
    );
}
