---
name: platform-rules
description: Validates, tests, and parses multi-country platformization TOML profiles (in config/jurisdictions/) for legal frameworks, account numbering rules, KYC tiers, and tax levy configurations.
---

# Multi-Country Platform Rules Skill

Use this skill when adding, modifying, or auditing country-specific banking rules in `config/jurisdictions/` (e.g. `NG.toml`, `GB.toml`, `GH.toml`).

---

## TOML Schema Requirements

Every jurisdiction file (`config/jurisdictions/{ISO_CODE}.toml`) must define:

1. **`[jurisdiction]`:**
   * `country_code`: 2-letter ISO 3166-1 alpha-2 code (uppercase).
   * `country_name`: Official country name string.
   * `currency_iso`: 3-letter ISO 4217 currency code.
   * `currency_symbol`: Visual currency symbol (e.g. `₦`, `£`, `GH₵`).
   * `minor_unit`: Decimal precision scale (typically `2`).

2. **`[account_standard]`:**
   * `format`: String identifier (`NUBAN`, `SORT_CODE_AND_ACCOUNT`, `IBAN`).
   * `length`: Total character length of account number.
   * `bank_code_length`: Optional length of institution identifier prefix.
   * `serial_length`: Optional length of account serial number.
   * `check_digit_algorithm`: Algorithm identifier (`CBN_MOD_10`, `MOD_11`, `MOD_97`).

3. **`[kyc_tiers.<tier_name>]`:**
   * `max_single_deposit`: Maximum credit allowed in one transaction.
   * `max_cumulative_balance`: Total allowable balance (`0.0` = Unlimited).
   * `required_documents`: Array of required verification document identifiers.

---

## Validation Procedure

1. Verify file syntax with `cv-platform-config-parser`:
   ```bash
   cargo test -p cv-platform-config-parser
   ```
2. Verify API serving:
   Ensure the REST endpoint `/api/v1/config/jurisdictions/:code` parses and returns the profile without error:
   ```bash
   curl -s http://127.0.0.1:8080/api/v1/config/jurisdictions/NG | jq .
   ```

## Definition of Done
* Profile parses with zero deserialization errors.
* Currency minor unit and account length match national central bank guidelines.
* Unit test in `cv-platform-config-parser` validates the new jurisdiction.
