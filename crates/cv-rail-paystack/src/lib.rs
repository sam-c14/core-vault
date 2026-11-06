pub mod adapter;
pub mod client;

pub use adapter::PaystackRailAdapter;
pub use client::{
    decimal_to_kobo, kobo_to_decimal, CreateTransferRecipientRequest, PaystackAccountResolveData,
    PaystackBalanceItem, PaystackBankItem, PaystackBulkTransferItem, PaystackBulkTransferRequest,
    PaystackConfig, PaystackFinalizeTransferRequest, PaystackRecipientData,
    PaystackRecipientDetails, PaystackTransferData, PaystackTransferRequest,
};

#[cfg(test)]
mod tests {
    use super::*;
    use cv_types::traits::{OutwardTransfer, PaymentRailAdapter};
    use rust_decimal_macros::dec;

    #[test]
    fn test_decimal_to_kobo_conversion() {
        assert_eq!(decimal_to_kobo(dec!(100.00)), 10000);
        assert_eq!(decimal_to_kobo(dec!(1550.50)), 155050);
        assert_eq!(decimal_to_kobo(dec!(0.05)), 5);
        assert_eq!(decimal_to_kobo(dec!(0.00)), 0);
        assert_eq!(decimal_to_kobo(dec!(-50.00)), 0);
    }

    #[test]
    fn test_kobo_to_decimal_conversion() {
        assert_eq!(kobo_to_decimal(10000), dec!(100.00));
        assert_eq!(kobo_to_decimal(155050), dec!(1550.50));
        assert_eq!(kobo_to_decimal(5), dec!(0.05));
        assert_eq!(kobo_to_decimal(2_500_000_000), dec!(25000000.00));
    }

    #[tokio::test]
    async fn test_paystack_sandbox_name_enquiry() {
        let adapter = PaystackRailAdapter::sandbox();

        let enquiry1 = adapter.name_enquiry("044", "0440001237").await.unwrap();
        assert_eq!(enquiry1.account_name, "COREVAULT PAYSTACK BENEFICIARY");
        assert_eq!(enquiry1.account_number, "0440001237");
        assert_eq!(enquiry1.bank_code, "044");

        let enquiry2 = adapter.name_enquiry("058", "0580001230").await.unwrap();
        assert_eq!(enquiry2.account_name, "CHUKWUEMEKA OBI");

        // Invalid account number length
        let err = adapter.name_enquiry("058", "12345").await.unwrap_err();
        assert!(matches!(err, cv_types::CoreError::InvalidAccountNumber(_)));
    }

    #[tokio::test]
    async fn test_paystack_sandbox_outward_transfer() {
        let adapter = PaystackRailAdapter::sandbox();
        let transfer = OutwardTransfer {
            transaction_ref: "PS-OUT-001".to_string(),
            source_account: "0581111111".to_string(),
            destination_account: "0582222222".to_string(),
            destination_bank_code: "058".to_string(),
            amount: dec!(50000.00),
            narration: "Vendor payment via Paystack".to_string(),
        };

        let receipt = adapter.dispatch_outward(&transfer).await.unwrap();
        assert_eq!(receipt.response_code, "00");
        assert!(receipt.session_id.starts_with("TRF_ps_"));
        assert!(receipt.message.contains("5000000 kobo"));

        // Zero or negative amounts fail
        let mut invalid_transfer = transfer.clone();
        invalid_transfer.amount = dec!(0.00);
        assert!(adapter.dispatch_outward(&invalid_transfer).await.is_err());
    }

    #[tokio::test]
    async fn test_paystack_create_recipient_and_single_transfer() {
        let adapter = PaystackRailAdapter::sandbox();

        // 1. Create Recipient
        let recipient_req = CreateTransferRecipientRequest {
            recipient_type: "nuban".to_string(),
            name: "John Doe".to_string(),
            account_number: "0581111111".to_string(),
            bank_code: "058".to_string(),
            currency: "NGN".to_string(),
            description: Some("Vendor Payout".to_string()),
            metadata: None,
        };

        let recipient_res = adapter
            .create_transfer_recipient(&recipient_req)
            .await
            .unwrap();
        assert!(recipient_res.recipient_code.starts_with("RCP_ps_"));
        assert_eq!(recipient_res.name, "John Doe");
        assert_eq!(
            recipient_res.details.as_ref().unwrap().bank_name.as_deref(),
            Some("Guaranty Trust Bank")
        );

        // 2. Initiate Single Transfer
        let transfer_req = PaystackTransferRequest {
            source: "balance".to_string(),
            amount: 7500000, // 75,000.00 NGN in Kobo
            recipient: recipient_res.recipient_code.clone(),
            reason: Some("Invoice settlement".to_string()),
            currency: "NGN".to_string(),
            reference: Some("INV-PS-9988".to_string()),
        };

        let transfer_res = adapter
            .initiate_single_transfer(&transfer_req)
            .await
            .unwrap();
        assert_eq!(transfer_res.status, "success");
        assert!(transfer_res.transfer_code.starts_with("TRF_ps_"));
        assert_eq!(transfer_res.reference, "INV-PS-9988");
        assert_eq!(transfer_res.amount, 7500000);
    }

    #[tokio::test]
    async fn test_paystack_balance_and_verify_transfer() {
        let adapter = PaystackRailAdapter::sandbox();

        // Trait check_balance
        let balance = adapter.check_balance("NGN").await.unwrap();
        assert_eq!(balance, dec!(25000000.00));

        // Direct fetch_balances
        let balances = adapter.fetch_balances().await.unwrap();
        assert!(balances
            .iter()
            .any(|b| b.currency == "NGN" && b.balance == 2_500_000_000));

        // Trait verify_transfer
        let receipt = adapter.verify_transfer("INV-PS-9988").await.unwrap();
        assert_eq!(receipt.response_code, "00");
        assert_eq!(receipt.session_id, "TRF_ver_INV-PS-9988");

        // Direct verify_transfer_status
        let status = adapter.verify_transfer_status("INV-PS-9988").await.unwrap();
        assert_eq!(status.status, "success");
        assert_eq!(status.reference, "INV-PS-9988");
    }

    #[tokio::test]
    async fn test_paystack_banks_and_finalize_transfer() {
        let adapter = PaystackRailAdapter::sandbox();

        let banks = adapter.list_banks(None).await.unwrap();
        assert!(banks.len() >= 5);
        assert!(banks
            .iter()
            .any(|b| b.code == "058" && b.name == "Guaranty Trust Bank"));

        let finalize_req = PaystackFinalizeTransferRequest {
            transfer_code: "TRF_ps_sample123".to_string(),
            otp: "123456".to_string(),
        };

        let final_res = adapter.finalize_transfer(&finalize_req).await.unwrap();
        assert_eq!(final_res.status, "success");
        assert_eq!(final_res.transfer_code, "TRF_ps_sample123");
    }
}
