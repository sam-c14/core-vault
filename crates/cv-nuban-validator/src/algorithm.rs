use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum NubanError {
    #[error("Invalid bank code length: expected 3 digits")]
    InvalidBankCode,
    #[error("Invalid serial length: expected 6 digits")]
    InvalidSerial,
    #[error("Invalid account number length: expected 10 digits")]
    InvalidLength,
    #[error("Non-numeric character found in account digits")]
    NonNumericDigit,
    #[error("Bank code mismatch: expected {expected}, found {found} in account number")]
    BankCodeMismatch { expected: String, found: String },
    #[error("Check digit mismatch: calculated {calculated}, got {provided}")]
    ChecksumFailed { calculated: u8, provided: u8 },
}

pub const NUBAN_WEIGHTS: [u32; 9] = [3, 7, 3, 3, 7, 3, 3, 7, 3];

pub fn calculate_check_digit(bank_code: &str, serial: &str) -> Result<u8, NubanError> {
    if bank_code.len() != 3 || !bank_code.chars().all(|c| c.is_ascii_digit()) {
        return Err(NubanError::InvalidBankCode);
    }
    if serial.len() != 6 || !serial.chars().all(|c| c.is_ascii_digit()) {
        return Err(NubanError::InvalidSerial);
    }

    let mut sum = 0;
    for (ch, &weight) in bank_code.chars().chain(serial.chars()).zip(NUBAN_WEIGHTS.iter()) {
        let digit = ch.to_digit(10).ok_or(NubanError::NonNumericDigit)?;
        sum += digit * weight;
    }

    let remainder = sum % 10;
    Ok(if remainder == 0 { 0 } else { (10 - remainder) as u8 })
}

pub fn generate_nuban(bank_code: &str, serial_num: u32) -> Result<String, NubanError> {
    let serial = format!("{:06}", serial_num);
    let check_digit = calculate_check_digit(bank_code, &serial)?;
    Ok(format!("{}{}{}", bank_code, serial, check_digit))
}

pub fn validate_nuban(bank_code: &str, account_number: &str) -> Result<(), NubanError> {
    if account_number.len() != 10 {
        return Err(NubanError::InvalidLength);
    }
    if !account_number.chars().all(|c| c.is_ascii_digit()) {
        return Err(NubanError::NonNumericDigit);
    }

    let embedded_bank_code = &account_number[0..3];
    if embedded_bank_code != bank_code {
        return Err(NubanError::BankCodeMismatch {
            expected: bank_code.to_string(),
            found: embedded_bank_code.to_string(),
        });
    }

    let serial = &account_number[3..9];
    let provided_check: u8 = account_number[9..10]
        .parse()
        .map_err(|_| NubanError::NonNumericDigit)?;
    let calculated_check = calculate_check_digit(bank_code, serial)?;

    if provided_check != calculated_check {
        return Err(NubanError::ChecksumFailed {
            calculated: calculated_check,
            provided: provided_check,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_nuban_flow() {
        let nuban = generate_nuban("058", 12345).unwrap();
        assert_eq!(nuban.len(), 10);
        assert!(validate_nuban("058", &nuban).is_ok());
    }

    #[test]
    fn test_tampered_nuban_fails() {
        let nuban = generate_nuban("058", 12345).unwrap();
        let mut tampered = nuban.clone();
        tampered.replace_range(9..10, "9");
        if tampered != nuban {
            assert!(validate_nuban("058", &tampered).is_err());
        }
    }
}
