use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Configuration for Paystack Payment Rail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackConfig {
    pub secret_key: String,
    pub base_url: String,
    pub is_sandbox: bool,
}

impl Default for PaystackConfig {
    fn default() -> Self {
        Self {
            secret_key: "sk_test_paystack_sandbox_seed_key_demo".to_string(),
            base_url: "https://api.paystack.co".to_string(),
            is_sandbox: true,
        }
    }
}

// ============================================================================
// Zero-Floating-Point Precision Conversion Helpers
// ============================================================================

/// Converts a Naira decimal to minor currency unit (Kobo integer: 1 NGN = 100 Kobo).
/// Guarantees zero floating point error by operating strictly with Decimal and integer parsing.
pub fn decimal_to_kobo(amount: Decimal) -> u64 {
    if amount <= Decimal::ZERO {
        return 0;
    }
    let kobo_dec = amount * Decimal::from(100);
    let s = kobo_dec.to_string();
    let int_part = s.split('.').next().unwrap_or("0");
    int_part.parse::<u64>().unwrap_or(0)
}

/// Converts Kobo integer to Naira Decimal (100 Kobo = 1.00 NGN).
/// Operates strictly with Decimal arithmetic (no f32 / f64).
pub fn kobo_to_decimal(kobo: u64) -> Decimal {
    Decimal::from(kobo) / Decimal::from(100)
}

// ============================================================================
// Paystack Identity Verification & Account Resolution DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackResolveAccountResponse {
    pub status: bool,
    pub message: String,
    pub data: Option<PaystackAccountResolveData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackAccountResolveData {
    pub account_number: String,
    pub account_name: String,
    #[serde(default)]
    pub bank_id: Option<u64>,
}

// ============================================================================
// Paystack Transfer Recipient DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransferRecipientRequest {
    #[serde(rename = "type")]
    pub recipient_type: String, // e.g. "nuban"
    pub name: String,
    pub account_number: String,
    pub bank_code: String,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

fn default_currency() -> String {
    "NGN".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackRecipientResponse {
    pub status: bool,
    pub message: String,
    pub data: Option<PaystackRecipientData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackRecipientData {
    pub recipient_code: String,
    pub active: bool,
    pub name: String,
    pub currency: String,
    #[serde(rename = "type")]
    pub recipient_type: String,
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub details: Option<PaystackRecipientDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackRecipientDetails {
    pub account_number: String,
    #[serde(default)]
    pub account_name: Option<String>,
    pub bank_code: String,
    #[serde(default)]
    pub bank_name: Option<String>,
}

// ============================================================================
// Paystack Single Transfer DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackTransferRequest {
    pub source: String,    // "balance"
    pub amount: u64,       // Smallest currency unit (Kobo for NGN)
    pub recipient: String, // e.g. "RCP_2x5j67o2gxuvgah"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackTransferResponse {
    pub status: bool,
    pub message: String,
    pub data: Option<PaystackTransferData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackTransferData {
    #[serde(default)]
    pub transfer_code: String,
    #[serde(default)]
    pub reference: String,
    pub amount: u64,
    #[serde(default = "default_currency")]
    pub currency: String,
    pub status: String, // "success", "pending", "failed", "reversed"
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub recipient: Option<serde_json::Value>,
    #[serde(default)]
    pub transferred_at: Option<String>,
    #[serde(default)]
    pub failures: Option<serde_json::Value>,
}

// ============================================================================
// Paystack Balance API DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackBalanceResponse {
    pub status: bool,
    pub message: String,
    pub data: Option<Vec<PaystackBalanceItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackBalanceItem {
    pub currency: String,
    pub balance: u64, // In minor unit (Kobo for NGN)
}

// ============================================================================
// Paystack Bank List DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackBankResponse {
    pub status: bool,
    pub message: String,
    pub data: Option<Vec<PaystackBankItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackBankItem {
    pub name: String,
    pub code: String,
    pub active: bool,
    #[serde(default)]
    pub slug: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
}

// ============================================================================
// Paystack Finalize & Bulk Transfer DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackFinalizeTransferRequest {
    pub transfer_code: String,
    pub otp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackBulkTransferItem {
    pub amount: u64,
    pub recipient: String,
    pub reference: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaystackBulkTransferRequest {
    #[serde(default = "default_currency")]
    pub currency: String,
    pub source: String, // "balance"
    pub transfers: Vec<PaystackBulkTransferItem>,
}
