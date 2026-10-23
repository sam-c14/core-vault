use crate::errors::CoreResult;
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountEnquiry {
    pub account_number: String,
    pub account_name: String,
    pub bank_code: String,
    pub kyc_tier: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutwardTransfer {
    pub transaction_ref: String,
    pub source_account: String,
    pub destination_account: String,
    pub destination_bank_code: String,
    pub amount: Decimal,
    pub narration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferReceipt {
    pub session_id: String,
    pub response_code: String,
    pub message: String,
}

/// Extension point for payment rails (e.g. NIBSS NIP in enterprise, or mock in public core)
#[async_trait]
pub trait PaymentRailAdapter: Send + Sync {
    async fn name_enquiry(
        &self,
        bank_code: &str,
        account_number: &str,
    ) -> CoreResult<AccountEnquiry>;
    async fn dispatch_outward(&self, transfer: &OutwardTransfer) -> CoreResult<TransferReceipt>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceDisposition {
    Approved,
    FlaggedForReview,
    Rejected,
}

/// Extension point for regulatory compliance checks (e.g. CBN / NFIU in enterprise)
#[async_trait]
pub trait ComplianceReporter: Send + Sync {
    async fn evaluate_transaction(
        &self,
        sender_id: &str,
        recipient_id: &str,
        amount: Decimal,
    ) -> CoreResult<ComplianceDisposition>;
}
