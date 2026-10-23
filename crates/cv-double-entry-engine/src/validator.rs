use crate::batch::PostingLeg;
use cv_types::{CoreError, CoreResult, Currency, EntryType};
use rust_decimal::Decimal;
use std::collections::HashMap;

pub fn validate_journal_batch(legs: &[PostingLeg]) -> CoreResult<()> {
    if legs.len() < 2 {
        return Err(CoreError::InsufficientPostings);
    }

    let mut balance_map: HashMap<Currency, (Decimal, Decimal)> = HashMap::new();

    for leg in legs {
        if leg.amount <= Decimal::ZERO {
            return Err(CoreError::NonPositivePosting(leg.amount));
        }

        let entry = balance_map
            .entry(leg.currency)
            .or_insert((Decimal::ZERO, Decimal::ZERO));

        match leg.entry_type {
            EntryType::Debit => entry.0 += leg.amount,
            EntryType::Credit => entry.1 += leg.amount,
        }
    }

    for (currency, (dr, cr)) in balance_map {
        if dr != cr {
            return Err(CoreError::ImbalancedJournal {
                currency: currency.code().to_string(),
                debits: dr,
                credits: cr,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    #[test]
    fn test_valid_double_entry_transfer() {
        let legs = vec![
            PostingLeg {
                account_id: Uuid::new_v4(),
                gl_code: "2110001".to_string(),
                currency: Currency::NGN,
                entry_type: EntryType::Debit,
                amount: dec!(50000.00),
            },
            PostingLeg {
                account_id: Uuid::new_v4(),
                gl_code: "2110002".to_string(),
                currency: Currency::NGN,
                entry_type: EntryType::Credit,
                amount: dec!(50000.00),
            },
        ];

        assert!(validate_journal_batch(&legs).is_ok());
    }

    #[test]
    fn test_imbalanced_transfer_rejected() {
        let legs = vec![
            PostingLeg {
                account_id: Uuid::new_v4(),
                gl_code: "2110001".to_string(),
                currency: Currency::NGN,
                entry_type: EntryType::Debit,
                amount: dec!(50000.00),
            },
            PostingLeg {
                account_id: Uuid::new_v4(),
                gl_code: "2110002".to_string(),
                currency: Currency::NGN,
                entry_type: EntryType::Credit,
                amount: dec!(49999.99),
            },
        ];

        assert!(matches!(
            validate_journal_batch(&legs),
            Err(CoreError::ImbalancedJournal { .. })
        ));
    }

    #[test]
    fn test_multi_leg_transfer_with_fees_balanced() {
        let legs = vec![
            // Customer A debited full amount: ₦100,000.00
            PostingLeg {
                account_id: Uuid::new_v4(),
                gl_code: "2110001".to_string(), // Savings Deposit
                currency: Currency::NGN,
                entry_type: EntryType::Debit,
                amount: dec!(100000.00),
            },
            // Customer B credited net amount: ₦99,900.00
            PostingLeg {
                account_id: Uuid::new_v4(),
                gl_code: "2110002".to_string(), // Current Deposit
                currency: Currency::NGN,
                entry_type: EntryType::Credit,
                amount: dec!(99900.00),
            },
            // Bank earns transaction fee: ₦100.00
            PostingLeg {
                account_id: Uuid::new_v4(),
                gl_code: "4210001".to_string(), // Fee Income
                currency: Currency::NGN,
                entry_type: EntryType::Credit,
                amount: dec!(100.00),
            },
        ];

        // 100,000 DR == 99,900 CR + 100 CR
        assert!(validate_journal_batch(&legs).is_ok());
    }
}
