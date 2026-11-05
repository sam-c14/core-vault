pub mod router;

pub use router::{PaymentGateway, PaymentRailProvider};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use cv_types::traits::{AccountEnquiry, OutwardTransfer, PaymentRailAdapter, TransferReceipt};
    use cv_types::{CoreError, CoreResult};
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    struct MockCustomAdapter;

    #[async_trait]
    impl PaymentRailAdapter for MockCustomAdapter {
        async fn name_enquiry(
            &self,
            bank_code: &str,
            account_number: &str,
        ) -> CoreResult<AccountEnquiry> {
            Ok(AccountEnquiry {
                account_number: account_number.to_string(),
                account_name: "CUSTOM ADAPTER USER".to_string(),
                bank_code: bank_code.to_string(),
                kyc_tier: Some(2),
            })
        }

        async fn dispatch_outward(
            &self,
            transfer: &OutwardTransfer,
        ) -> CoreResult<TransferReceipt> {
            Ok(TransferReceipt {
                session_id: format!("custom_{}", transfer.transaction_ref),
                response_code: "00".to_string(),
                message: "Custom rail dispatch successful".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn test_default_gateway_routes_to_anchor() {
        let gateway = PaymentGateway::anchor_default();
        assert_eq!(gateway.active_provider(), PaymentRailProvider::Anchor);

        // Name enquiry through Anchor
        let enquiry = gateway.name_enquiry("044", "0440001237").await.unwrap();
        assert_eq!(enquiry.account_name, "COREVAULT RETAIL CUSTOMER");

        // Outward transfer through Anchor
        let transfer = OutwardTransfer {
            transaction_ref: "GW-TXN-001".to_string(),
            source_account: "0440001237".to_string(),
            destination_account: "0440001230".to_string(),
            destination_bank_code: "044".to_string(),
            amount: dec!(20000.00),
            narration: "Vendor payment via Anchor".to_string(),
        };

        let receipt = gateway.dispatch_outward(&transfer).await.unwrap();
        assert_eq!(receipt.response_code, "00");
        assert!(receipt.session_id.starts_with("tra_")); // Anchor format
    }

    #[tokio::test]
    async fn test_register_and_switch_to_custom_adapter() {
        let gateway = PaymentGateway::anchor_default();

        let custom_provider = PaymentRailProvider::Custom("PaystackDirect".to_string());
        gateway.register_provider(custom_provider.clone(), Arc::new(MockCustomAdapter));

        gateway
            .set_active_provider(custom_provider.clone())
            .unwrap();
        assert_eq!(gateway.active_provider(), custom_provider);

        let enquiry = gateway.name_enquiry("058", "0581111111").await.unwrap();
        assert_eq!(enquiry.account_name, "CUSTOM ADAPTER USER");

        let transfer = OutwardTransfer {
            transaction_ref: "GW-CUSTOM-001".to_string(),
            source_account: "0581111111".to_string(),
            destination_account: "0582222222".to_string(),
            destination_bank_code: "058".to_string(),
            amount: dec!(10000.00),
            narration: "Custom rail test".to_string(),
        };

        let receipt = gateway.dispatch_outward(&transfer).await.unwrap();
        assert_eq!(receipt.response_code, "00");
        assert_eq!(receipt.session_id, "custom_GW-CUSTOM-001");
    }

    #[test]
    fn test_unregistered_nibss_direct_switch_returns_clear_error() {
        let gateway = PaymentGateway::anchor_default();
        let err = gateway
            .set_active_provider(PaymentRailProvider::NibssDirect)
            .unwrap_err();

        match err {
            CoreError::PaymentRailError(msg) => {
                assert!(msg.contains("requires cba-enterprise cv-rail-nip"));
            }
            _ => panic!("Expected PaymentRailError"),
        }
    }
}
