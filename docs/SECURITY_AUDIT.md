# Security Audit & Vulnerability Assessment Report
# CoreVault — Core Banking System in Rust

> **Date:** October 2026  
> **Status:** Passed & Hardened  
> **Target:** Public Core Crates & Monolith Runtime (`v0.1.0`)  
> **Classification:** Internal Security Review (Phase 4)

---

## 1. Executive Summary

A comprehensive security audit of CoreVault’s core crates and runtime was conducted across five critical threat vectors:
1. **Mathematical Financial Invariants & Float Pollution**
2. **Account Identifier & NUBAN Integrity**
3. **Cryptographic Primitives & Session Security**
4. **Injection Defenses (SQL, Headers, Code)**
5. **State Machine Access Controls & Restriction Enforcement**

**Result:** All 33 automated unit, integration, and adversarial attack tests passed successfully. One moderate vulnerability was discovered in NUBAN validation, patched immediately, and verified with automated test suites.

---

## 2. Threat Model & Vulnerability Findings

### Finding CV-SEC-001: Bank Code Prefix Verification Bypass in NUBAN Validator
* **Severity:** Medium
* **Crate:** `cv-nuban-validator` (`algorithm.rs`)
* **Vector:** Bank Code Spoofing / Single-Digit Transposition
* **Description:** The `validate_nuban(bank_code, account_number)` function calculated the check digit against the provided `bank_code` and the serial substring `account_number[3..9]`, but never asserted that `account_number[0..3] == bank_code`. Consequently, modifying the leading 3 digits of an account number allowed cross-bank spoofing without triggering a checksum error.
* **Remediation:** 
  1. Added explicit ASCII digit validation across all 10 characters.
  2. Added an embedded bank code equality check (`account_number[0..3] == bank_code`).
  3. Introduced a dedicated typed error `NubanError::BankCodeMismatch`.
* **Verification:** Tested against all single-digit permutations across all 10 positions (`tests/security_nuban_test.rs`). **PASSED**.

---

## 3. Threat Vector Security Verification

### 3.1 Financial Invariants & Decimal Math Security
* **Floating Point Prohibition:** The entire financial accounting pipeline (`cv-types`, `cv-double-entry-engine`, `cv-account-manager`) was scanned for IEEE-754 floating-point primitives (`f32`, `f64`). **Zero floating-point variables exist.**
* **Fractional Sub-Cent Money Creation Attacks:** Validated that fractional splits (e.g., ₦100.0001) are strictly rejected as imbalanced by `validate_journal_batch`.
* **Currency Arbitrage & Confusion Attacks:** Validated that debiting one currency (e.g., `NGN`) and crediting another (e.g., `USD`) in a single transaction fails double-entry validation. Currency balance sums are segregated per ISO currency.
* **Zero & Negative Value Posting Injections:** Transactions with negative amounts (`-₦500.00`) or zero amounts (`₦0.00`) are rejected at both API boundary and domain validator with `CoreError::NonPositivePosting`.

### 3.2 Cryptographic Hygiene & Authentication
* **Password Hashing:** `cv-auth` implements **Argon2id** (`v0.5.3`) using parameters compliant with OWASP guidelines:
  - Algorithm: `Argon2id` (v0x13)
  - Memory cost: 64 MiB (`Params::DEFAULT_M_COST`)
  - Time cost: 3 iterations
  - Parallelism: 1 thread
  - Salt: Cryptographically secure pseudo-random number generator (`rand_core::OsRng`).
* **JWT Integrity & Privilege Escalation:**
  - Token signing uses HMAC-SHA256 with minimum 256-bit secret key entropy.
  - Tested that modifying payload claims (e.g., changing `role: TELLER` to `role: SUPER_ADMIN`) causes instant signature rejection.
  - Tested that tokens signed by attacker-controlled keys are rejected.
  - Tested that empty, malformed, or missing tokens return `401 Unauthorized`.

### 3.3 SQL Injection Defenses
* **Query Parameterization:** The codebase was scanned for raw SQL string interpolations (`format!("SELECT ...")`). **Zero string interpolations found.**
* **SQLx Type Safety:** All database queries utilize compile-time verified parameterized bindings (`$1`, `$2`), eliminating SQL injection surfaces.
* **Database Invariant Constraints:** The PostgreSQL schema enforces deferred foreign key constraints and partitioned table boundaries preventing orphaned journal legs.

### 3.4 State Machine & Restriction Enforcement
* **Account Status Protections:**
  - `ACTIVE`: Debits and credits allowed.
  - `POST_NO_DEBIT (PND)`: Debits strictly blocked; incoming credits allowed.
  - `FROZEN`: Both debits and credits strictly blocked.
* **Lien & Balance Hold Safety:**
  - Placing a lien reduces effective available balance (`available_balance + overdraft - lien_amount`) without modifying historical ledger balances.
  - Releasing a lien greater than the currently active lien amount is rejected.

---

## 4. Adversarial Attack Test Suite Results

```
running 4 tests (cv-auth security)
test security_argon2id_timing_resistance_on_wrong_password ... ok
test security_reject_empty_or_malformed_token            ... ok
test security_reject_key_substitution_attack             ... ok
test security_reject_forged_claims_jwt                   ... ok

running 4 tests (cv-double-entry-engine security)
test security_reject_single_leg_self_balancing_attack    ... ok
test security_reject_fractional_split_money_creation     ... ok
test security_reject_zero_or_negative_posting_amounts    ... ok
test security_reject_currency_arbitrage_confusion        ... ok

running 3 tests (cv-nuban-validator security)
test security_reject_bank_code_spoofing                  ... ok
test security_reject_non_numeric_injection_in_nuban      ... ok
test security_detect_all_single_digit_alterations        ... ok

Total Security Tests: 11 | Passed: 11 | Failed: 0
Total Workspace Tests: 33 | Passed: 33 | Failed: 0
```

---

## 5. Security Recommendations for Production

1. **Secret Key Rotation:** Rotate `COREVAULT_MASTER_KEY_HEX` and `JWT_SECRET` periodically via automated pipeline.
2. **Rate Limiting:** Enforce strict per-IP rate limits at the Caddy edge layer (max 100 req/min for general API, max 10 req/min for `/api/v1/auth/login`).
3. **Database SSL:** When deploying outside localhost, configure `sslmode=verify-full` in `DATABASE_URL` with trusted root certificates.
