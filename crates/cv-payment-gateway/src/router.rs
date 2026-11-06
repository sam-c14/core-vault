use async_trait::async_trait;
use cv_rail_anchor::AnchorRailAdapter;
use cv_rail_paystack::PaystackRailAdapter;
use cv_types::traits::{AccountEnquiry, OutwardTransfer, PaymentRailAdapter, TransferReceipt};
use cv_types::{CoreError, CoreResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PaymentRailProvider {
    /// Anchor Banking-as-a-Service (Default for MVP, fintechs, & SMEs)
    Anchor,
    /// Paystack Transfers & Banking Infrastructure (First-class payment rail)
    Paystack,
    /// Direct NIBSS Instant Payments switch (Available via cba-enterprise cv-rail-nip)
    NibssDirect,
    /// Custom third-party or proprietary payment rail adapter
    Custom(String),
}

impl std::fmt::Display for PaymentRailProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Anchor => write!(f, "Anchor"),
            Self::Paystack => write!(f, "Paystack"),
            Self::NibssDirect => write!(f, "NibssDirect"),
            Self::Custom(name) => write!(f, "Custom({})", name),
        }
    }
}

#[derive(Clone)]
pub struct PaymentGateway {
    active_provider: Arc<RwLock<PaymentRailProvider>>,
    adapters: Arc<RwLock<HashMap<PaymentRailProvider, Arc<dyn PaymentRailAdapter>>>>,
}

impl PaymentGateway {
    /// Creates a gateway initialized with an initial provider and adapter.
    pub fn new(
        initial_provider: PaymentRailProvider,
        initial_adapter: Arc<dyn PaymentRailAdapter>,
    ) -> Self {
        let mut adapters = HashMap::new();
        adapters.insert(initial_provider.clone(), initial_adapter);
        Self {
            active_provider: Arc::new(RwLock::new(initial_provider)),
            adapters: Arc::new(RwLock::new(adapters)),
        }
    }

    /// Creates a gateway configured for Anchor in sandbox mode by default.
    pub fn anchor_default() -> Self {
        let anchor = Arc::new(AnchorRailAdapter::sandbox());
        Self::new(PaymentRailProvider::Anchor, anchor)
    }

    /// Creates a gateway configured for Paystack in sandbox mode by default.
    pub fn paystack_default() -> Self {
        let paystack = Arc::new(PaystackRailAdapter::sandbox());
        Self::new(PaymentRailProvider::Paystack, paystack)
    }

    /// Registers a payment rail provider and its adapter.
    pub fn register_provider(
        &self,
        provider: PaymentRailProvider,
        adapter: Arc<dyn PaymentRailAdapter>,
    ) {
        let mut adapters = self.adapters.write().unwrap();
        tracing::info!(provider = %provider, "Registered payment rail adapter");
        adapters.insert(provider, adapter);
    }

    /// Sets the active payment rail provider.
    ///
    /// Returns an error if the specified provider has not been registered.
    pub fn set_active_provider(&self, provider: PaymentRailProvider) -> CoreResult<()> {
        let adapters = self.adapters.read().unwrap();
        if !adapters.contains_key(&provider) {
            let msg = match &provider {
                PaymentRailProvider::NibssDirect => {
                    "Payment rail provider 'NibssDirect' is not registered. Direct NIBSS switch requires cba-enterprise cv-rail-nip.".to_string()
                }
                other => format!("Payment rail provider '{}' has not been registered in PaymentGateway.", other),
            };
            return Err(CoreError::PaymentRailError(msg));
        }

        let mut active = self.active_provider.write().unwrap();
        *active = provider.clone();
        tracing::info!(provider = %provider, "Payment gateway active rail updated");
        Ok(())
    }

    /// Returns the currently active payment rail provider.
    pub fn active_provider(&self) -> PaymentRailProvider {
        self.active_provider.read().unwrap().clone()
    }

    /// Returns a list of all currently registered providers.
    pub fn registered_providers(&self) -> Vec<PaymentRailProvider> {
        let adapters = self.adapters.read().unwrap();
        adapters.keys().cloned().collect()
    }

    /// Returns the active adapter instance.
    fn current_adapter(&self) -> CoreResult<Arc<dyn PaymentRailAdapter>> {
        let active = self.active_provider();
        let adapters = self.adapters.read().unwrap();
        adapters.get(&active).cloned().ok_or_else(|| {
            CoreError::PaymentRailError(format!(
                "Active payment rail provider '{}' has no registered adapter",
                active
            ))
        })
    }
}

#[async_trait]
impl PaymentRailAdapter for PaymentGateway {
    async fn name_enquiry(
        &self,
        bank_code: &str,
        account_number: &str,
    ) -> CoreResult<AccountEnquiry> {
        let adapter = self.current_adapter()?;
        adapter.name_enquiry(bank_code, account_number).await
    }

    async fn dispatch_outward(&self, transfer: &OutwardTransfer) -> CoreResult<TransferReceipt> {
        let adapter = self.current_adapter()?;
        adapter.dispatch_outward(transfer).await
    }

    async fn check_balance(&self, currency: &str) -> CoreResult<Decimal> {
        let adapter = self.current_adapter()?;
        adapter.check_balance(currency).await
    }

    async fn verify_transfer(&self, reference: &str) -> CoreResult<TransferReceipt> {
        let adapter = self.current_adapter()?;
        adapter.verify_transfer(reference).await
    }
}
