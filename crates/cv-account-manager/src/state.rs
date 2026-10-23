use chrono::{DateTime, Utc};
use cv_types::{AccountId, AccountStatus, AccountType, Currency, PndSource};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountRecord {
    pub id: AccountId,
    pub tenant_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub account_number: String,
    pub account_name: String,
    pub gl_code: String,
    pub account_type: AccountType,
    pub currency: Currency,
    pub ledger_balance: Decimal,
    pub available_balance: Decimal,
    pub lien_amount: Decimal,
    pub overdraft_limit: Decimal,
    pub status: AccountStatus,
    pub pnd_source: Option<PndSource>,
    pub pnd_reason: Option<String>,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl AccountRecord {
    pub fn effective_available_funds(&self) -> Decimal {
        self.available_balance + self.overdraft_limit - self.lien_amount
    }

    pub fn can_withdraw(&self, amount: Decimal) -> bool {
        self.status.can_debit() && self.effective_available_funds() >= amount
    }
}
