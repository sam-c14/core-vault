pub mod adapter;
pub mod client;

pub use adapter::AnchorRailAdapter;
pub use client::{decimal_to_kobo, AnchorConfig};

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
    }

    #[tokio::test]
    async fn test_anchor_sandbox_name_enquiry() {
        let adapter = AnchorRailAdapter::sandbox();
        let enquiry = adapter.name_enquiry("044", "0440001237").await.unwrap();
        assert_eq!(enquiry.account_name, "COREVAULT RETAIL CUSTOMER");
        assert_eq!(enquiry.bank_code, "044");
    }

    #[tokio::test]
    async fn test_anchor_sandbox_outward_transfer() {
        let adapter = AnchorRailAdapter::sandbox();
        let transfer = OutwardTransfer {
            transaction_ref: "ANCHOR-TRX-001".to_string(),
            source_account: "0440001237".to_string(),
            destination_account: "0440001230".to_string(),
            destination_bank_code: "044".to_string(),
            amount: dec!(15000.00),
            narration: "Logistics payment".to_string(),
        };

        let receipt = adapter.dispatch_outward(&transfer).await.unwrap();
        assert_eq!(receipt.response_code, "00");
        assert!(receipt.session_id.starts_with("tra_"));
    }
}
