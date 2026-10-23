use crate::state::AccountRecord;
use chrono::Utc;
use cv_nuban_validator::generate_nuban;
use cv_types::{AccountId, AccountStatus, AccountType, CoreError, CoreResult, Currency};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAccountDto {
    pub tenant_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub account_name: String,
    pub gl_code: String,
    pub account_type: AccountType,
    pub currency: Currency,
    pub bank_code: String,
    pub serial_number: u32,
    pub overdraft_limit: Option<Decimal>,
}

pub struct AccountManager;

impl AccountManager {
    pub fn build_new_account(dto: CreateAccountDto) -> CoreResult<AccountRecord> {
        let account_number = generate_nuban(&dto.bank_code, dto.serial_number)
            .map_err(|e| CoreError::InvalidAccountNumber(e.to_string()))?;

        let now = Utc::now();
        Ok(AccountRecord {
            id: AccountId::new(),
            tenant_id: dto.tenant_id,
            customer_id: dto.customer_id,
            account_number,
            account_name: dto.account_name,
            gl_code: dto.gl_code,
            account_type: dto.account_type,
            currency: dto.currency,
            ledger_balance: Decimal::ZERO,
            available_balance: Decimal::ZERO,
            lien_amount: Decimal::ZERO,
            overdraft_limit: dto.overdraft_limit.unwrap_or(Decimal::ZERO),
            status: AccountStatus::Active,
            version: 0,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn place_lien(account: &mut AccountRecord, lien: Decimal) -> CoreResult<()> {
        if lien <= Decimal::ZERO {
            return Err(CoreError::NonPositivePosting(lien));
        }
        account.lien_amount += lien;
        account.version += 1;
        account.updated_at = Utc::now();
        Ok(())
    }

    pub fn release_lien(account: &mut AccountRecord, lien: Decimal) -> CoreResult<()> {
        if lien <= Decimal::ZERO || lien > account.lien_amount {
            return Err(CoreError::NegativeAmount(lien));
        }
        account.lien_amount -= lien;
        account.version += 1;
        account.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_create_account_with_valid_nuban() {
        let dto = CreateAccountDto {
            tenant_id: Uuid::new_v4(),
            customer_id: Some(Uuid::new_v4()),
            account_name: "Chukwudi Okafor".to_string(),
            gl_code: "2110001".to_string(),
            account_type: AccountType::Savings,
            currency: Currency::NGN,
            bank_code: "058".to_string(),
            serial_number: 123456,
            overdraft_limit: Some(dec!(10000.00)),
        };

        let account = AccountManager::build_new_account(dto).unwrap();
        assert_eq!(account.account_number.len(), 10);
        assert_eq!(account.status, AccountStatus::Active);
        assert_eq!(account.overdraft_limit, dec!(10000.00));
        assert!(cv_nuban_validator::validate_nuban("058", &account.account_number).is_ok());
    }

    #[test]
    fn test_lien_placement_and_release() {
        let mut account = AccountManager::build_new_account(CreateAccountDto {
            tenant_id: Uuid::new_v4(),
            customer_id: None,
            account_name: "Merchant Escrow".to_string(),
            gl_code: "2110003".to_string(),
            account_type: AccountType::Escrow,
            currency: Currency::NGN,
            bank_code: "058".to_string(),
            serial_number: 654321,
            overdraft_limit: None,
        })
        .unwrap();

        account.available_balance = dec!(100000.00);
        assert_eq!(account.effective_available_funds(), dec!(100000.00));

        // Place a lien of 30,000
        AccountManager::place_lien(&mut account, dec!(30000.00)).unwrap();
        assert_eq!(account.lien_amount, dec!(30000.00));
        assert_eq!(account.effective_available_funds(), dec!(70000.00));
        assert!(account.can_withdraw(dec!(70000.00)));
        assert!(!account.can_withdraw(dec!(70000.01)));

        // Release 10,000 lien
        AccountManager::release_lien(&mut account, dec!(10000.00)).unwrap();
        assert_eq!(account.lien_amount, dec!(20000.00));
        assert_eq!(account.effective_available_funds(), dec!(80000.00));

        // Attempt to release more than active lien
        assert!(AccountManager::release_lien(&mut account, dec!(50000.00)).is_err());
    }

    #[test]
    fn test_account_status_withdrawal_permissions() {
        let mut account = AccountManager::build_new_account(CreateAccountDto {
            tenant_id: Uuid::new_v4(),
            customer_id: None,
            account_name: "Restricted Account".to_string(),
            gl_code: "2110001".to_string(),
            account_type: AccountType::Savings,
            currency: Currency::NGN,
            bank_code: "058".to_string(),
            serial_number: 111111,
            overdraft_limit: None,
        })
        .unwrap();

        account.available_balance = dec!(50000.00);

        // Active account can withdraw
        assert!(account.can_withdraw(dec!(10000.00)));

        // Post No Debit (PND) cannot debit, but can receive credit
        account.status = AccountStatus::PostNoDebit;
        assert!(!account.can_withdraw(dec!(10000.00)));
        assert!(account.status.can_credit());
        assert!(!account.status.can_debit());

        // Frozen account cannot debit and cannot credit
        account.status = AccountStatus::Frozen;
        assert!(!account.can_withdraw(dec!(10000.00)));
        assert!(!account.status.can_credit());
        assert!(!account.status.can_debit());
    }
}

