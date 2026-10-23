# CoreVault Platformization & Multi-Country Localization Guide

CoreVault is engineered with a **Platformization First** architecture. The core accounting engine, double-entry mathematical invariants, and balance ledger are decoupled from country-specific regulatory, tax, KYC, and account numbering rules.

All jurisdiction-specific rules are defined declaratively in TOML files located in `config/jurisdictions/` and parsed at runtime by the `cv-platform-config-parser` crate.

---

## 🌍 1. Architecture Overview

```
                      ┌────────────────────────────────────┐
                      │    CoreVault Monolith Engine       │
                      │  - Mathematical Invariants        │
                      │  - Immutable Double-Entry Ledger   │
                      │  - Generic Account State Machine   │
                      └─────────────────┬──────────────────┘
                                        │
           Loads dynamically via cv-platform-config-parser
                                        │
             ┌──────────────────────────┴──────────────────────────┐
             ▼                                                     ▼
┌─────────────────────────┐                               ┌─────────────────────────┐
│ config/jurisdictions/   │                               │ config/jurisdictions/   │
│         NG.toml         │                               │         GB.toml         │
│  - Currency: NGN (₦)    │                               │  - Currency: GBP (£)    │
│  - NUBAN Mod-10 Check   │                               │  - Sort Code + Mod-11   │
│  - CBN Tier 1/2/3 KYC   │                               │  - FCA UK Regulations   │
└─────────────────────────┘                               └─────────────────────────┘
```

By decoupling these configurations:
1. **Zero Recompilation:** New jurisdictions can be added or KYC tiers updated without altering or recompiling core accounting logic.
2. **Audit Compliance:** Every country profile can be version-controlled, code-reviewed, and cryptographically verified.
3. **Pluggable Payment Rails:** Private enterprise payment rails (`PaymentRailAdapter`) bind directly to the country code configured in the active profile.

---

## 📜 2. TOML Configuration Schema

Every jurisdiction file (`config/jurisdictions/{ISO_CODE}.toml`) must adhere to the schema below:

### `[jurisdiction]` Section
Basic metadata and currency settings:

| Field | Type | Description | Example |
| :--- | :--- | :--- | :--- |
| `country_code` | String | ISO 3166-1 alpha-2 country code | `"NG"`, `"GB"`, `"GH"` |
| `country_name` | String | Official country name | `"Nigeria"`, `"United Kingdom"` |
| `currency_iso` | String | ISO 4217 3-letter currency code | `"NGN"`, `"GBP"`, `"GHS"` |
| `currency_symbol` | String | Currency symbol for display | `"₦"`, `"£"`, `"GH₵"` |
| `minor_unit` | Integer | Subunit scale (e.g. 2 for 100 kobo / pence) | `2` |

### `[account_standard]` Section
Account numbering structure and check-digit rules:

| Field | Type | Description | Example |
| :--- | :--- | :--- | :--- |
| `format` | String | Account standard identifier | `"NUBAN"`, `"SORT_CODE_AND_ACCOUNT"` |
| `length` | Integer | Total length of account string | `10` (Nigeria), `14` (UK) |
| `bank_code_length`| Integer (Opt)| Leading institution/sort code length | `3` (Nigeria), `6` (UK) |
| `serial_length` | Integer (Opt)| Account serial number length | `6` (Nigeria), `8` (UK) |
| `check_digit_algorithm` | String | Algorithm name for validation | `"CBN_MOD_10"`, `"MOD_11"`, `"MOD_97"` |

### `[kyc_tiers.<tier_name>]` Section
Configures regulatory KYC tiers, single deposit caps, cumulative balance limits, and required verification documents:

| Field | Type | Description | Example |
| :--- | :--- | :--- | :--- |
| `max_single_deposit` | Float/Decimal | Maximum credit allowed in a single transaction | `50000.00` |
| `max_cumulative_balance` | Float/Decimal | Maximum total balance permitted (`0.00` = Unlimited) | `300000.00` |
| `required_documents` | Array of Strings | Required KYC documents for this tier | `["PHONE_NUMBER"]` |

---

## 🛠️ 3. Step-by-Step: Adding a New Country Jurisdiction

Suppose we want to add support for **Ghana (GH)** with Ghanaian Cedi (`GHS`) and Bank of Ghana KYC Tier regulations.

### Step 1: Create `config/jurisdictions/GH.toml`
Create a new configuration file in `config/jurisdictions/GH.toml`:

```toml
[jurisdiction]
country_code = "GH"
country_name = "Ghana"
currency_iso = "GHS"
currency_symbol = "GH₵"
minor_unit = 2

[account_standard]
format = "GH_BANK_ACCOUNT"
length = 13
bank_code_length = 3
serial_length = 9
check_digit_algorithm = "MOD_10"

[kyc_tiers.tier_1]
max_single_deposit = 1000.00
max_cumulative_balance = 5000.00
required_documents = ["MOBILE_NUMBER", "PASSPORT_PHOTO"]

[kyc_tiers.tier_2]
max_single_deposit = 10000.00
max_cumulative_balance = 20000.00
required_documents = ["GHANA_CARD"]

[kyc_tiers.tier_3]
max_single_deposit = 50000.00
max_cumulative_balance = 0.00 # Unlimited
required_documents = ["GHANA_CARD", "PROOF_OF_ADDRESS", "TIN"]
```

### Step 2: Validate via Parser Unit Test
Add a test in `crates/cv-platform-config-parser/src/model.rs` or run an ad-hoc test:

```rust
#[test]
fn test_parse_ghana_config() {
    let path = std::path::Path::new("config/jurisdictions/GH.toml");
    let cfg = PlatformConfigParser::parse_file(path).expect("Failed to parse Ghana configuration");
    assert_eq!(cfg.jurisdiction.country_code, "GH");
    assert_eq!(cfg.jurisdiction.currency_iso, "GHS");
    assert_eq!(cfg.kyc_tiers.unwrap().len(), 3);
}
```

### Step 3: Verify via HTTP API
Once placed in `config/jurisdictions/`, the endpoint `/api/v1/config/jurisdictions/GH` immediately resolves and serves the configuration:

```bash
curl -s http://127.0.0.1:8080/api/v1/config/jurisdictions/GH | jq .
```

Expected JSON response:
```json
{
  "configuration": {
    "account_standard": {
      "bank_code_length": 3,
      "check_digit_algorithm": "MOD_10",
      "format": "GH_BANK_ACCOUNT",
      "length": 13,
      "serial_length": 9
    },
    "jurisdiction": {
      "country_code": "GH",
      "country_name": "Ghana",
      "currency_iso": "GHS",
      "currency_symbol": "GH₵",
      "minor_unit": 2
    },
    "kyc_tiers": {
      "tier_1": {
        "max_cumulative_balance": 5000.0,
        "max_single_deposit": 1000.0,
        "required_documents": [
          "MOBILE_NUMBER",
          "PASSPORT_PHOTO"
        ]
      },
      "tier_2": {
        "max_cumulative_balance": 20000.0,
        "max_single_deposit": 10000.0,
        "required_documents": [
          "GHANA_CARD"
        ]
      },
      "tier_3": {
        "max_cumulative_balance": 0.0,
        "max_single_deposit": 50000.0,
        "required_documents": [
          "GHANA_CARD",
          "PROOF_OF_ADDRESS",
          "TIN"
        ]
      }
    }
  },
  "status": "SUCCESS"
}
```

---

## 🔌 4. Enterprise Extension Trait Integration

When developing closed-source enterprise payment rails (e.g. Ghana GhIPSS instant transfers or UK Faster Payments), the enterprise crate implements the public `PaymentRailAdapter` trait provided by `cv-types`:

```rust
use async_trait::async_trait;
use cv_types::{PaymentRailAdapter, TransferRequest, TransferResponse, RailError};

pub struct GhipssRailAdapter {
    pub api_key: String,
    pub endpoint: String,
}

#[async_trait]
impl PaymentRailAdapter for GhipssRailAdapter {
    fn rail_code(&self) -> &'static str {
        "GH_GHIPSS_INSTANT"
    }

    async fn execute_outward_transfer(
        &self,
        request: TransferRequest,
    ) -> Result<TransferResponse, RailError> {
        // Dispatches payload over TLS to the national payment switch
        todo!("Dispatched to GhIPSS gateway")
    }
}
```

This ensures full architectural isolation: open-source accounting logic remains clean and untainted, while regional rails plug in cleanly through standard Rust traits.
