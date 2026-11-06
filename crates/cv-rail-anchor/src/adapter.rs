use crate::client::{decimal_to_kobo, AnchorConfig};
use async_trait::async_trait;
use cv_types::traits::{AccountEnquiry, OutwardTransfer, PaymentRailAdapter, TransferReceipt};
use cv_types::{CoreError, CoreResult};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct AnchorRailAdapter {
    pub config: AnchorConfig,
}

impl AnchorRailAdapter {
    pub fn new(config: AnchorConfig) -> Self {
        Self { config }
    }

    pub fn sandbox() -> Self {
        Self::new(AnchorConfig::default())
    }
}

#[async_trait]
impl PaymentRailAdapter for AnchorRailAdapter {
    async fn name_enquiry(
        &self,
        bank_code: &str,
        account_number: &str,
    ) -> CoreResult<AccountEnquiry> {
        if account_number.len() != 10 {
            return Err(CoreError::InvalidAccountNumber(
                "Anchor Name Enquiry requires a 10-digit NUBAN".to_string(),
            ));
        }

        if self.config.is_sandbox {
            // Anchor Sandbox Deterministic Simulation
            let verified_name = match account_number {
                acc if acc.ends_with('0') => "CHUKWUEMEKA OBI",
                acc if acc.ends_with('1') => "BABATUNDE ADENIYI",
                acc if acc.ends_with('2') => "FATIMA MOHAMMED",
                acc if acc.ends_with('7') => "COREVAULT RETAIL CUSTOMER",
                _ => "ANCHOR VERIFIED RECIPIENT",
            };

            return Ok(AccountEnquiry {
                account_number: account_number.to_string(),
                account_name: verified_name.to_string(),
                bank_code: bank_code.to_string(),
                kyc_tier: Some(3),
            });
        }

        // Live Anchor API call via HTTP POST /api/v1/counterparties
        Err(CoreError::PaymentRailError(
            "Live Anchor network transport requires valid x-anchor-key".to_string(),
        ))
    }

    async fn dispatch_outward(&self, transfer: &OutwardTransfer) -> CoreResult<TransferReceipt> {
        if transfer.amount <= Decimal::ZERO {
            return Err(CoreError::PaymentRailError(
                "Outward transfer amount must be strictly positive".to_string(),
            ));
        }

        let kobo_amount = decimal_to_kobo(transfer.amount);
        if kobo_amount == 0 {
            return Err(CoreError::PaymentRailError(
                "Transfer amount in minor units cannot be zero".to_string(),
            ));
        }

        if self.config.is_sandbox {
            // Anchor Sandbox Dispatch Simulation
            let anchor_tx_id = format!("tra_{}", uuid::Uuid::new_v4().simple());
            return Ok(TransferReceipt {
                session_id: anchor_tx_id,
                response_code: "00".to_string(), // Success
                message: format!(
                    "Anchor NIP transfer initiated successfully for {} kobo (ref: {})",
                    kobo_amount, transfer.transaction_ref
                ),
            });
        }

        // Live Anchor API call via HTTP POST /api/v1/transfers
        Err(CoreError::PaymentRailError(
            "Live Anchor transfer dispatch requires active credentials".to_string(),
        ))
    }

    async fn check_balance(&self, _currency: &str) -> CoreResult<Decimal> {
        if self.config.is_sandbox {
            // Anchor Sandbox Settlement Balance (50,000,000.00 NGN)
            return Ok(Decimal::from(50_000_000));
        }

        Err(CoreError::PaymentRailError(
            "Live Anchor balance enquiry requires active credentials".to_string(),
        ))
    }

    async fn verify_transfer(&self, reference: &str) -> CoreResult<TransferReceipt> {
        if self.config.is_sandbox {
            return Ok(TransferReceipt {
                session_id: format!("tra_ver_{}", reference),
                response_code: "00".to_string(),
                message: format!(
                    "Anchor transfer reference '{}' verified successfully with status 'success'",
                    reference
                ),
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Anchor transfer verification requires active credentials".to_string(),
        ))
    }
}
