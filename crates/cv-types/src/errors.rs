use rust_decimal::Decimal;
use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Error, Debug, PartialEq)]
pub enum CoreError {
    #[error("Negative amount not allowed: {0}")]
    NegativeAmount(Decimal),

    #[error("Currency mismatch: expected {expected}, found {found}")]
    CurrencyMismatch { expected: String, found: String },

    #[error("Invalid currency code: {0}")]
    InvalidCurrency(String),

    #[error("Insufficient funds: available {available}, required {required}")]
    InsufficientFunds { available: Decimal, required: Decimal },

    #[error("Account not active: account {0}")]
    AccountInactive(String),

    #[error("Post No Debit active on account: {0}")]
    PostNoDebit(String),

    #[error("Double-entry violation: total debits ({debits}) != total credits ({credits}) for currency {currency}")]
    ImbalancedJournal {
        currency: String,
        debits: Decimal,
        credits: Decimal,
    },

    #[error("Transaction must have at least 2 posting legs")]
    InsufficientPostings,

    #[error("Posting amount must be strictly positive: {0}")]
    NonPositivePosting(Decimal),

    #[error("Invalid account number: {0}")]
    InvalidAccountNumber(String),

    #[error("Payment rail error: {0}")]
    PaymentRailError(String),

    #[error("Compliance evaluation error: {0}")]
    ComplianceError(String),

    #[error("Internal database error: {0}")]
    DatabaseError(String),

    #[error("Resource not found: {0}")]
    NotFound(String),
}
