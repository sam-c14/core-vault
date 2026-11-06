use crate::client::{
    decimal_to_kobo, kobo_to_decimal, CreateTransferRecipientRequest, PaystackAccountResolveData,
    PaystackBalanceItem, PaystackBankItem, PaystackConfig, PaystackFinalizeTransferRequest,
    PaystackRecipientData, PaystackRecipientDetails, PaystackTransferData, PaystackTransferRequest,
};
use async_trait::async_trait;
use cv_types::traits::{AccountEnquiry, OutwardTransfer, PaymentRailAdapter, TransferReceipt};
use cv_types::{CoreError, CoreResult};
use rust_decimal::Decimal;

#[derive(Debug, Clone)]
pub struct PaystackRailAdapter {
    pub config: PaystackConfig,
}

impl PaystackRailAdapter {
    pub fn new(config: PaystackConfig) -> Self {
        Self { config }
    }

    pub fn sandbox() -> Self {
        Self::new(PaystackConfig::default())
    }

    // ========================================================================
    // Direct Paystack Consumer Methods
    // ========================================================================

    /// Resolves account number to verify account name (GET /bank/resolve).
    pub async fn resolve_account(
        &self,
        bank_code: &str,
        account_number: &str,
    ) -> CoreResult<PaystackAccountResolveData> {
        if account_number.len() != 10 {
            return Err(CoreError::InvalidAccountNumber(
                "Paystack account resolution requires a 10-digit NUBAN".to_string(),
            ));
        }

        if self.config.is_sandbox {
            let enquiry = self.name_enquiry(bank_code, account_number).await?;
            return Ok(PaystackAccountResolveData {
                account_number: enquiry.account_number,
                account_name: enquiry.account_name,
                bank_id: Some(9),
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack account resolution requires valid secret key and HTTP transport"
                .to_string(),
        ))
    }

    /// Creates a transfer recipient for future payouts (POST /transferrecipient).
    pub async fn create_transfer_recipient(
        &self,
        req: &CreateTransferRecipientRequest,
    ) -> CoreResult<PaystackRecipientData> {
        if req.account_number.len() != 10 {
            return Err(CoreError::InvalidAccountNumber(
                "Transfer recipient requires a 10-digit NUBAN account number".to_string(),
            ));
        }

        if req.bank_code.trim().is_empty() {
            return Err(CoreError::PaymentRailError(
                "Bank code cannot be empty when creating transfer recipient".to_string(),
            ));
        }

        if self.config.is_sandbox {
            let recipient_code = format!("RCP_ps_{}", uuid::Uuid::new_v4().simple());
            return Ok(PaystackRecipientData {
                recipient_code,
                active: true,
                name: req.name.clone(),
                currency: req.currency.clone(),
                recipient_type: req.recipient_type.clone(),
                id: Some(rand_id()),
                details: Some(PaystackRecipientDetails {
                    account_number: req.account_number.clone(),
                    account_name: Some(req.name.clone()),
                    bank_code: req.bank_code.clone(),
                    bank_name: Some(lookup_bank_name(&req.bank_code)),
                }),
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack recipient creation requires valid secret key".to_string(),
        ))
    }

    /// Initiates a single outward transfer to a registered recipient (POST /transfer).
    pub async fn initiate_single_transfer(
        &self,
        req: &PaystackTransferRequest,
    ) -> CoreResult<PaystackTransferData> {
        if req.amount == 0 {
            return Err(CoreError::PaymentRailError(
                "Transfer amount in minor units cannot be zero".to_string(),
            ));
        }

        if !req.recipient.starts_with("RCP_") {
            return Err(CoreError::PaymentRailError(
                "Paystack transfer requires a valid recipient code starting with 'RCP_'"
                    .to_string(),
            ));
        }

        let ref_code = req
            .reference
            .clone()
            .unwrap_or_else(|| format!("ref_ps_{}", uuid::Uuid::new_v4().simple()));

        if self.config.is_sandbox {
            let transfer_code = format!("TRF_ps_{}", uuid::Uuid::new_v4().simple());
            return Ok(PaystackTransferData {
                transfer_code,
                reference: ref_code,
                amount: req.amount,
                currency: req.currency.clone(),
                status: "success".to_string(),
                id: Some(rand_id()),
                reason: req.reason.clone(),
                recipient: Some(serde_json::json!({
                    "recipient_code": req.recipient
                })),
                transferred_at: Some(chrono::Utc::now().to_rfc3339()),
                failures: None,
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack transfer initiation requires valid secret key".to_string(),
        ))
    }

    /// Verifies transfer status by reference (GET /transfer/verify/:reference).
    pub async fn verify_transfer_status(
        &self,
        reference: &str,
    ) -> CoreResult<PaystackTransferData> {
        if reference.trim().is_empty() {
            return Err(CoreError::PaymentRailError(
                "Reference cannot be empty when verifying Paystack transfer".to_string(),
            ));
        }

        if self.config.is_sandbox {
            return Ok(PaystackTransferData {
                transfer_code: format!("TRF_ver_{}", reference),
                reference: reference.to_string(),
                amount: 50_000,
                currency: "NGN".to_string(),
                status: "success".to_string(),
                id: Some(rand_id()),
                reason: Some("Verified sandbox transfer".to_string()),
                recipient: None,
                transferred_at: Some(chrono::Utc::now().to_rfc3339()),
                failures: None,
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack transfer verification requires active credentials".to_string(),
        ))
    }

    /// Fetches transfer by transfer code or ID (GET /transfer/:id_or_code).
    pub async fn fetch_transfer(&self, id_or_code: &str) -> CoreResult<PaystackTransferData> {
        if id_or_code.trim().is_empty() {
            return Err(CoreError::PaymentRailError(
                "Transfer identifier cannot be empty".to_string(),
            ));
        }

        if self.config.is_sandbox {
            return Ok(PaystackTransferData {
                transfer_code: id_or_code.to_string(),
                reference: format!("ref_fetch_{}", id_or_code),
                amount: 100_000,
                currency: "NGN".to_string(),
                status: "success".to_string(),
                id: Some(rand_id()),
                reason: Some("Fetched transfer record".to_string()),
                recipient: None,
                transferred_at: Some(chrono::Utc::now().to_rfc3339()),
                failures: None,
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack fetch transfer requires active credentials".to_string(),
        ))
    }

    /// Fetches all currency balances on Paystack account (GET /balance).
    pub async fn fetch_balances(&self) -> CoreResult<Vec<PaystackBalanceItem>> {
        if self.config.is_sandbox {
            return Ok(vec![
                PaystackBalanceItem {
                    currency: "NGN".to_string(),
                    balance: 2_500_000_000, // 25,000,000.00 NGN in Kobo
                },
                PaystackBalanceItem {
                    currency: "USD".to_string(),
                    balance: 1_000_000, // 10,000.00 USD in Cents
                },
            ]);
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack balance fetch requires active credentials".to_string(),
        ))
    }

    /// Lists supported commercial banks and financial institutions (GET /bank).
    pub async fn list_banks(&self, _country: Option<&str>) -> CoreResult<Vec<PaystackBankItem>> {
        if self.config.is_sandbox {
            return Ok(vec![
                PaystackBankItem {
                    name: "Access Bank".to_string(),
                    code: "044".to_string(),
                    active: true,
                    slug: Some("access-bank".to_string()),
                    country: Some("Nigeria".to_string()),
                    currency: Some("NGN".to_string()),
                },
                PaystackBankItem {
                    name: "Guaranty Trust Bank".to_string(),
                    code: "058".to_string(),
                    active: true,
                    slug: Some("guaranty-trust-bank".to_string()),
                    country: Some("Nigeria".to_string()),
                    currency: Some("NGN".to_string()),
                },
                PaystackBankItem {
                    name: "Zenith Bank".to_string(),
                    code: "057".to_string(),
                    active: true,
                    slug: Some("zenith-bank".to_string()),
                    country: Some("Nigeria".to_string()),
                    currency: Some("NGN".to_string()),
                },
                PaystackBankItem {
                    name: "First Bank of Nigeria".to_string(),
                    code: "011".to_string(),
                    active: true,
                    slug: Some("first-bank-of-nigeria".to_string()),
                    country: Some("Nigeria".to_string()),
                    currency: Some("NGN".to_string()),
                },
                PaystackBankItem {
                    name: "United Bank For Africa".to_string(),
                    code: "033".to_string(),
                    active: true,
                    slug: Some("united-bank-for-africa".to_string()),
                    country: Some("Nigeria".to_string()),
                    currency: Some("NGN".to_string()),
                },
            ]);
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack bank listing requires active credentials".to_string(),
        ))
    }

    /// Finalizes a transfer requiring OTP (POST /transfer/finalize_transfer).
    pub async fn finalize_transfer(
        &self,
        req: &PaystackFinalizeTransferRequest,
    ) -> CoreResult<PaystackTransferData> {
        if req.otp.trim().is_empty() {
            return Err(CoreError::PaymentRailError(
                "OTP cannot be empty when finalizing transfer".to_string(),
            ));
        }

        if self.config.is_sandbox {
            return Ok(PaystackTransferData {
                transfer_code: req.transfer_code.clone(),
                reference: format!("ref_fin_{}", req.transfer_code),
                amount: 50_000,
                currency: "NGN".to_string(),
                status: "success".to_string(),
                id: Some(rand_id()),
                reason: Some("Finalized with OTP".to_string()),
                recipient: None,
                transferred_at: Some(chrono::Utc::now().to_rfc3339()),
                failures: None,
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack transfer finalization requires active credentials".to_string(),
        ))
    }
}

// ============================================================================
// PaymentRailAdapter Core Implementation
// ============================================================================

#[async_trait]
impl PaymentRailAdapter for PaystackRailAdapter {
    async fn name_enquiry(
        &self,
        bank_code: &str,
        account_number: &str,
    ) -> CoreResult<AccountEnquiry> {
        if account_number.len() != 10 {
            return Err(CoreError::InvalidAccountNumber(
                "Paystack Name Enquiry requires a 10-digit NUBAN".to_string(),
            ));
        }

        if self.config.is_sandbox {
            // Paystack Sandbox Deterministic Simulation
            let verified_name = match account_number {
                acc if acc.ends_with('0') => "CHUKWUEMEKA OBI",
                acc if acc.ends_with('1') => "BABATUNDE ADENIYI",
                acc if acc.ends_with('2') => "FATIMA MOHAMMED",
                acc if acc.ends_with('7') => "COREVAULT PAYSTACK BENEFICIARY",
                _ => "PAYSTACK VERIFIED ACCOUNT",
            };

            return Ok(AccountEnquiry {
                account_number: account_number.to_string(),
                account_name: verified_name.to_string(),
                bank_code: bank_code.to_string(),
                kyc_tier: Some(3),
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack network transport requires valid sk_live key".to_string(),
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
            // Paystack Sandbox Dispatch Simulation
            let transfer_code = format!("TRF_ps_{}", uuid::Uuid::new_v4().simple());
            return Ok(TransferReceipt {
                session_id: transfer_code,
                response_code: "00".to_string(), // Success
                message: format!(
                    "Paystack single transfer dispatched successfully for {} kobo (ref: {})",
                    kobo_amount, transfer.transaction_ref
                ),
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack transfer dispatch requires active credentials".to_string(),
        ))
    }

    async fn check_balance(&self, currency: &str) -> CoreResult<Decimal> {
        if self.config.is_sandbox {
            let balances = self.fetch_balances().await?;
            if let Some(item) = balances
                .iter()
                .find(|b| b.currency.eq_ignore_ascii_case(currency))
            {
                return Ok(kobo_to_decimal(item.balance));
            }
            return Ok(Decimal::ZERO);
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack balance enquiry requires active credentials".to_string(),
        ))
    }

    async fn verify_transfer(&self, reference: &str) -> CoreResult<TransferReceipt> {
        if self.config.is_sandbox {
            return Ok(TransferReceipt {
                session_id: format!("TRF_ver_{}", reference),
                response_code: "00".to_string(),
                message: format!(
                    "Paystack transfer reference '{}' verified successfully with status 'success'",
                    reference
                ),
            });
        }

        Err(CoreError::PaymentRailError(
            "Live Paystack transfer verification requires active credentials".to_string(),
        ))
    }
}

// ============================================================================
// Internal Helper Functions
// ============================================================================

fn rand_id() -> u64 {
    let u = uuid::Uuid::new_v4();
    (u.as_u128() % 1_000_000) as u64 + 100_000
}

fn lookup_bank_name(bank_code: &str) -> String {
    match bank_code {
        "044" => "Access Bank".to_string(),
        "058" => "Guaranty Trust Bank".to_string(),
        "057" => "Zenith Bank".to_string(),
        "011" => "First Bank of Nigeria".to_string(),
        "033" => "United Bank For Africa".to_string(),
        _ => "Commercial Bank".to_string(),
    }
}
