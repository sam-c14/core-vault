use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to parse TOML config: {0}")]
    ParseError(#[from] toml::de::Error),
    #[error("IO error reading configuration: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionInfo {
    pub country_code: String,
    pub country_name: String,
    pub currency_iso: String,
    pub currency_symbol: String,
    pub minor_unit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountStandardConfig {
    pub format: String,
    pub length: u32,
    pub bank_code_length: Option<u32>,
    pub serial_length: Option<u32>,
    pub check_digit_algorithm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycTierConfig {
    pub max_single_deposit: Option<f64>,
    pub max_cumulative_balance: Option<f64>,
    pub required_documents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionConfig {
    pub jurisdiction: JurisdictionInfo,
    pub account_standard: AccountStandardConfig,
    pub kyc_tiers: Option<HashMap<String, KycTierConfig>>,
}

pub struct PlatformConfigParser;

impl PlatformConfigParser {
    pub fn parse_str(raw: &str) -> Result<JurisdictionConfig, ConfigError> {
        let config: JurisdictionConfig = toml::from_str(raw)?;
        Ok(config)
    }

    pub fn parse_file(path: &std::path::Path) -> Result<JurisdictionConfig, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_str(&content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nigeria_config() {
        let raw = r#"
        [jurisdiction]
        country_code = "NG"
        country_name = "Nigeria"
        currency_iso = "NGN"
        currency_symbol = "₦"
        minor_unit = 2

        [account_standard]
        format = "NUBAN"
        length = 10
        bank_code_length = 3
        serial_length = 6
        check_digit_algorithm = "CBN_MOD_10"
        "#;

        let cfg = PlatformConfigParser::parse_str(raw).unwrap();
        assert_eq!(cfg.jurisdiction.country_code, "NG");
        assert_eq!(cfg.account_standard.format, "NUBAN");
    }
}
