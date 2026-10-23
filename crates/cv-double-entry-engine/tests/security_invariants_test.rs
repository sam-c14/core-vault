use cv_double_entry_engine::{validate_journal_batch, PostingLeg};
use cv_types::{CoreError, Currency, EntryType};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use uuid::Uuid;

#[test]
fn security_reject_fractional_split_money_creation_attack() {
    // Attack scenario: Attacker attempts to create fractional cents through floating roundoff
    let legs = vec![
        PostingLeg {
            account_id: Uuid::new_v4(),
            gl_code: "2110001".to_string(),
            currency: Currency::NGN,
            entry_type: EntryType::Debit,
            amount: dec!(100.0001), // Extra fractional sub-cent
        },
        PostingLeg {
            account_id: Uuid::new_v4(),
            gl_code: "2110002".to_string(),
            currency: Currency::NGN,
            entry_type: EntryType::Credit,
            amount: dec!(100.0000),
        },
    ];

    // Must be rejected as imbalanced
    let res = validate_journal_batch(&legs);
    assert!(matches!(res, Err(CoreError::ImbalancedJournal { .. })));
}

#[test]
fn security_reject_currency_arbitrage_confusion_attack() {
    // Attack scenario: Attacker attempts to debit 100 NGN and credit 100 USD in a single transaction
    let legs = vec![
        PostingLeg {
            account_id: Uuid::new_v4(),
            gl_code: "2110001".to_string(),
            currency: Currency::NGN,
            entry_type: EntryType::Debit,
            amount: dec!(100.00),
        },
        PostingLeg {
            account_id: Uuid::new_v4(),
            gl_code: "2110002".to_string(),
            currency: Currency::USD, // Different currency
            entry_type: EntryType::Credit,
            amount: dec!(100.00),
        },
    ];

    // Ledger engine groups by currency, so NGN will have 100 DR / 0 CR -> Imbalanced
    let res = validate_journal_batch(&legs);
    assert!(matches!(res, Err(CoreError::ImbalancedJournal { .. })));
}

#[test]
fn security_reject_zero_or_negative_posting_amounts() {
    let acc1 = Uuid::new_v4();
    let acc2 = Uuid::new_v4();

    // Zero amount
    let zero_legs = vec![
        PostingLeg {
            account_id: acc1,
            gl_code: "2110001".to_string(),
            currency: Currency::NGN,
            entry_type: EntryType::Debit,
            amount: Decimal::ZERO,
        },
        PostingLeg {
            account_id: acc2,
            gl_code: "2110002".to_string(),
            currency: Currency::NGN,
            entry_type: EntryType::Credit,
            amount: Decimal::ZERO,
        },
    ];
    assert!(matches!(
        validate_journal_batch(&zero_legs),
        Err(CoreError::NonPositivePosting(_))
    ));

    // Negative amount
    let negative_legs = vec![
        PostingLeg {
            account_id: acc1,
            gl_code: "2110001".to_string(),
            currency: Currency::NGN,
            entry_type: EntryType::Debit,
            amount: dec!(-500.00),
        },
        PostingLeg {
            account_id: acc2,
            gl_code: "2110002".to_string(),
            currency: Currency::NGN,
            entry_type: EntryType::Credit,
            amount: dec!(-500.00),
        },
    ];
    assert!(matches!(
        validate_journal_batch(&negative_legs),
        Err(CoreError::NonPositivePosting(_))
    ));
}

#[test]
fn security_reject_single_leg_self_balancing_attack() {
    // Attack scenario: Attacker attempts to post a single-leg transaction
    let single_leg = vec![PostingLeg {
        account_id: Uuid::new_v4(),
        gl_code: "2110001".to_string(),
        currency: Currency::NGN,
        entry_type: EntryType::Debit,
        amount: dec!(50000.00),
    }];

    assert!(matches!(
        validate_journal_batch(&single_leg),
        Err(CoreError::InsufficientPostings)
    ));
}
