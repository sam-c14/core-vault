use crate::money::Currency;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TenantStatus {
    Pending,
    Active,
    Blacklisted,
    Suspended,
}

impl TenantStatus {
    pub fn is_active(&self) -> bool {
        matches!(self, TenantStatus::Active)
    }

    pub fn is_blacklisted(&self) -> bool {
        matches!(self, TenantStatus::Blacklisted)
    }
}

impl std::fmt::Display for TenantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "PENDING"),
            Self::Active => write!(f, "ACTIVE"),
            Self::Blacklisted => write!(f, "BLACKLISTED"),
            Self::Suspended => write!(f, "SUSPENDED"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PndRuleType {
    KycTierLimit,
    VelocityBurst,
    DailyDebitCap,
    SuspiciousTransfer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantPndRule {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub rule_name: String,
    pub rule_type: PndRuleType,
    pub max_single_amount: Option<Decimal>,
    pub max_daily_cumulative: Option<Decimal>,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRecord {
    pub id: Uuid,
    pub tenant_code: String,
    pub tenant_name: String,
    pub base_currency: Currency,
    pub client_id: Option<String>,
    pub client_secret_hash: Option<String>,
    pub status: TenantStatus,
    pub allowed_ips: Vec<String>,
    pub scopes: Vec<String>,
    pub blacklist_reason: Option<String>,
    pub violation_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TenantRecord {
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    pub fn is_blacklisted(&self) -> bool {
        self.status.is_blacklisted()
    }

    pub fn whitelist(&mut self) {
        self.status = TenantStatus::Active;
        self.blacklist_reason = None;
        self.updated_at = Utc::now();
    }

    pub fn blacklist(&mut self, reason: &str) {
        self.status = TenantStatus::Blacklisted;
        self.blacklist_reason = Some(reason.to_string());
        self.updated_at = Utc::now();
    }

    pub fn record_violation(&mut self, reason: &str) {
        self.violation_count += 1;
        self.status = TenantStatus::Blacklisted;
        self.blacklist_reason = Some(format!("Platform automated blacklist: {reason}"));
        self.updated_at = Utc::now();
    }
}
