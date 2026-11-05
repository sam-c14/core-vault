use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorConfig {
    pub api_key: String,
    pub base_url: String,
    pub source_deposit_account_id: String,
    pub is_sandbox: bool,
}

impl Default for AnchorConfig {
    fn default() -> Self {
        Self {
            api_key: "sandbox_anchor_key_demo".to_string(),
            base_url: "https://api.sandbox.getanchor.co/api/v1".to_string(),
            source_deposit_account_id: "anc_acc_default_01".to_string(),
            is_sandbox: true,
        }
    }
}

// --- Anchor API Request DTOs (Matching JSON-API Standard) ---

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCounterpartyRequest {
    pub data: CounterpartyRequestData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CounterpartyRequestData {
    pub r#type: String, // "CounterParty"
    pub attributes: CounterpartyRequestAttributes,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CounterpartyRequestAttributes {
    #[serde(rename = "bankCode")]
    pub bank_code: String,
    #[serde(rename = "accountName")]
    pub account_name: String,
    #[serde(rename = "accountNumber")]
    pub account_number: String,
    #[serde(rename = "verifyName")]
    pub verify_name: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateTransferRequest {
    pub data: TransferRequestData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferRequestData {
    pub r#type: String, // "NIPTransfer"
    pub attributes: TransferRequestAttributes,
    pub relationships: TransferRequestRelationships,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferRequestAttributes {
    pub amount: u64, // Smallest currency unit (Kobo for NGN)
    pub currency: String,
    pub reason: String,
    pub reference: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferRequestRelationships {
    pub account: RelationshipItem,
    #[serde(rename = "counterParty")]
    pub counter_party: RelationshipItem,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RelationshipItem {
    pub data: RelationshipData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RelationshipData {
    pub id: String,
    pub r#type: String,
}

// --- Helper Functions ---

/// Converts a Naira decimal to minor currency unit (Kobo integer)
pub fn decimal_to_kobo(amount: Decimal) -> u64 {
    if amount <= Decimal::ZERO {
        return 0;
    }
    let kobo_dec = amount * Decimal::from(100);
    let s = kobo_dec.to_string();
    let int_part = s.split('.').next().unwrap_or("0");
    int_part.parse::<u64>().unwrap_or(0)
}
