use chrono::Utc;
use cv_account_manager::AccountRecord;
use cv_audit::MemoryAuditSink;
use cv_auth::{hash_password, JwtService};
use cv_types::{
    AccountId, AccountStatus, AccountType, Currency, TenantPndRule, TenantRecord, TenantStatus,
};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub jwt_service: Arc<JwtService>,
    pub audit_sink: Arc<MemoryAuditSink>,
    pub tenants: Arc<RwLock<HashMap<Uuid, TenantRecord>>>,
    pub tenants_by_client_id: Arc<RwLock<HashMap<String, Uuid>>>,
    pub accounts: Arc<RwLock<HashMap<String, AccountRecord>>>, // Keyed by NUBAN
    pub accounts_by_id: Arc<RwLock<HashMap<Uuid, AccountRecord>>>, // Keyed by Account UUID
    pub pnd_rules: Arc<RwLock<Vec<TenantPndRule>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "super_secret_jwt_key_change_in_production_123456".to_string());
        let jwt_service = Arc::new(JwtService::new(&jwt_secret));
        let audit_sink = Arc::new(MemoryAuditSink::new());

        let mut tenants = HashMap::new();
        let mut tenants_by_client_id = HashMap::new();

        // Seed native wallet app tenant
        let wallet_tenant_id = Uuid::parse_str("a0000000-0000-0000-0000-000000000001").unwrap();
        let wallet_client_id = "client_wallet_app_seed001".to_string();
        let wallet_raw_secret = "sec_live_wallet_app_seed_key_2026";
        let wallet_secret_hash = hash_password(wallet_raw_secret).unwrap_or_default();

        let now = Utc::now();
        let wallet_tenant = TenantRecord {
            id: wallet_tenant_id,
            tenant_code: "WALLET_APP".to_string(),
            tenant_name: "CoreVault Native Wallet App".to_string(),
            base_currency: Currency::NGN,
            client_id: Some(wallet_client_id.clone()),
            client_secret_hash: Some(wallet_secret_hash),
            status: TenantStatus::Active,
            allowed_ips: vec!["127.0.0.1".to_string(), "::1".to_string()],
            scopes: vec![
                "transfers:write".to_string(),
                "accounts:read".to_string(),
                "accounts:write".to_string(),
                "admin:read".to_string(),
            ],
            blacklist_reason: None,
            violation_count: 0,
            created_at: now,
            updated_at: now,
        };

        tenants.insert(wallet_tenant_id, wallet_tenant);
        tenants_by_client_id.insert(wallet_client_id, wallet_tenant_id);

        let mut accounts = HashMap::new();
        let mut accounts_by_id = HashMap::new();

        // Seed 2 default active accounts for intra-transfer baseline
        let acc1_id = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let acc2_id = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();

        let acc1 = AccountRecord {
            id: AccountId(acc1_id),
            tenant_id: wallet_tenant_id,
            customer_id: None,
            account_number: "0581111111".to_string(),
            account_name: "Source Account".to_string(),
            gl_code: "2110001".to_string(),
            account_type: AccountType::Savings,
            currency: Currency::NGN,
            ledger_balance: Decimal::from(10_000_000),
            available_balance: Decimal::from(10_000_000),
            lien_amount: Decimal::ZERO,
            overdraft_limit: Decimal::ZERO,
            status: AccountStatus::Active,
            pnd_source: None,
            pnd_reason: None,
            version: 1,
            created_at: now,
            updated_at: now,
        };

        let acc2 = AccountRecord {
            id: AccountId(acc2_id),
            tenant_id: wallet_tenant_id,
            customer_id: None,
            account_number: "0582222222".to_string(),
            account_name: "Destination Account".to_string(),
            gl_code: "2110002".to_string(),
            account_type: AccountType::Savings,
            currency: Currency::NGN,
            ledger_balance: Decimal::from(5_000_000),
            available_balance: Decimal::from(5_000_000),
            lien_amount: Decimal::ZERO,
            overdraft_limit: Decimal::ZERO,
            status: AccountStatus::Active,
            pnd_source: None,
            pnd_reason: None,
            version: 1,
            created_at: now,
            updated_at: now,
        };

        accounts.insert(acc1.account_number.clone(), acc1.clone());
        accounts.insert(acc2.account_number.clone(), acc2.clone());
        accounts_by_id.insert(acc1_id, acc1);
        accounts_by_id.insert(acc2_id, acc2);

        Self {
            jwt_service,
            audit_sink,
            tenants: Arc::new(RwLock::new(tenants)),
            tenants_by_client_id: Arc::new(RwLock::new(tenants_by_client_id)),
            accounts: Arc::new(RwLock::new(accounts)),
            accounts_by_id: Arc::new(RwLock::new(accounts_by_id)),
            pnd_rules: Arc::new(RwLock::new(Vec::new())),
        }
    }
}
