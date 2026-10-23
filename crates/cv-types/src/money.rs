use crate::errors::{CoreError, CoreResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Currency {
    NGN,
    USD,
    GBP,
    EUR,
}

impl Currency {
    pub fn code(&self) -> &'static str {
        match self {
            Currency::NGN => "NGN",
            Currency::USD => "USD",
            Currency::GBP => "GBP",
            Currency::EUR => "EUR",
        }
    }

    pub fn minor_units(&self) -> u32 {
        match self {
            Currency::NGN | Currency::USD | Currency::GBP | Currency::EUR => 2,
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl std::str::FromStr for Currency {
    type Err = CoreError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "NGN" => Ok(Currency::NGN),
            "USD" => Ok(Currency::USD),
            "GBP" => Ok(Currency::GBP),
            "EUR" => Ok(Currency::EUR),
            other => Err(CoreError::InvalidCurrency(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    amount: Decimal,
    currency: Currency,
}

impl Money {
    pub fn new(amount: Decimal, currency: Currency) -> CoreResult<Self> {
        if amount < Decimal::ZERO {
            return Err(CoreError::NegativeAmount(amount));
        }
        Ok(Self { amount, currency })
    }

    pub fn zero(currency: Currency) -> Self {
        Self {
            amount: Decimal::ZERO,
            currency,
        }
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn currency(&self) -> Currency {
        self.currency
    }

    pub fn add(&self, other: &Money) -> CoreResult<Money> {
        if self.currency != other.currency {
            return Err(CoreError::CurrencyMismatch {
                expected: self.currency.code().to_string(),
                found: other.currency.code().to_string(),
            });
        }
        Ok(Money {
            amount: self.amount + other.amount,
            currency: self.currency,
        })
    }

    pub fn subtract(&self, other: &Money) -> CoreResult<Money> {
        if self.currency != other.currency {
            return Err(CoreError::CurrencyMismatch {
                expected: self.currency.code().to_string(),
                found: other.currency.code().to_string(),
            });
        }
        if self.amount < other.amount {
            return Err(CoreError::InsufficientFunds {
                available: self.amount,
                required: other.amount,
            });
        }
        Ok(Money {
            amount: self.amount - other.amount,
            currency: self.currency,
        })
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_money_addition_same_currency() {
        let m1 = Money::new(dec!(1000.50), Currency::NGN).unwrap();
        let m2 = Money::new(dec!(500.25), Currency::NGN).unwrap();
        let sum = m1.add(&m2).unwrap();
        assert_eq!(sum.amount(), dec!(1500.75));
        assert_eq!(sum.currency(), Currency::NGN);
    }

    #[test]
    fn test_money_subtraction_same_currency() {
        let m1 = Money::new(dec!(1000.50), Currency::NGN).unwrap();
        let m2 = Money::new(dec!(500.50), Currency::NGN).unwrap();
        let diff = m1.subtract(&m2).unwrap();
        assert_eq!(diff.amount(), dec!(500.00));
    }

    #[test]
    fn test_insufficient_funds_rejected() {
        let m1 = Money::new(dec!(100.00), Currency::NGN).unwrap();
        let m2 = Money::new(dec!(200.00), Currency::NGN).unwrap();
        assert!(matches!(
            m1.subtract(&m2),
            Err(CoreError::InsufficientFunds { .. })
        ));
    }

    #[test]
    fn test_currency_mismatch_rejected() {
        let ngn = Money::new(dec!(1000.00), Currency::NGN).unwrap();
        let usd = Money::new(dec!(50.00), Currency::USD).unwrap();
        assert!(matches!(
            ngn.add(&usd),
            Err(CoreError::CurrencyMismatch { .. })
        ));
    }

    #[test]
    fn test_negative_money_creation_rejected() {
        assert!(matches!(
            Money::new(dec!(-10.00), Currency::NGN),
            Err(CoreError::NegativeAmount(_))
        ));
    }
}
