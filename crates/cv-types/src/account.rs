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

impl std::fmt::Display for AccountStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PendingOpening => write!(f, "PENDING_OPENING"),
            Self::Active => write!(f, "ACTIVE"),
            Self::Dormant => write!(f, "DORMANT"),
            Self::PostNoDebit => write!(f, "POST_NO_DEBIT"),
            Self::Frozen => write!(f, "FROZEN"),
            Self::Closed => write!(f, "CLOSED"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PndSource {
    ManualClientPnd,
    SystemAutomatedPnd,
}

impl std::fmt::Display for PndSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ManualClientPnd => write!(f, "MANUAL_CLIENT_PND"),
            Self::SystemAutomatedPnd => write!(f, "SYSTEM_AUTOMATED_PND"),
        }
    }
}
