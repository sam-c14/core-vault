use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AccountId(pub Uuid);

impl AccountId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl Default for AccountId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountType {
    Savings,
    Current,
    Domiciliary,
    Loan,
    InternalGl,
    Escrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AccountStatus {
    PendingOpening,
    Active,
    Dormant,
    PostNoDebit,
    Frozen,
    Closed,
}

impl AccountStatus {
    pub fn can_debit(&self) -> bool {
        matches!(self, AccountStatus::Active)
    }

    pub fn can_credit(&self) -> bool {
        matches!(
            self,
            AccountStatus::Active | AccountStatus::Dormant | AccountStatus::PostNoDebit
        )
    }
}
