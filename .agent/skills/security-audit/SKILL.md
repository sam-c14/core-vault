---
name: security-audit
description: Conducts security vulnerability scans and cryptographic compliance checks: Argon2id password hashing strength, timing resistance, JWT signing and revocation, NUBAN bank code spoofing verification (CV-SEC-001 regression tests), and staged secrets scanning.
---

# Security Audit & Verification Skill

Use this skill to audit authentication flows, cryptographic primitives, input validation routines, and staged commits for security vulnerabilities and data leakage.

---

## Security Domains & Checkpoints

### 1. NUBAN Generation & Validation (CV-SEC-001 Regression)
* **Vulnerability Context:** Check-digit algorithms that omit bank-code prefix verification allow cross-bank account number spoofing.
* **Invariant:** `account_number[0..3]` MUST equal `bank_code`. Only numeric ASCII characters (`0-9`) are permitted.
* **Verification Command:**
  ```bash
  cargo test --test security_nuban_test -- --nocapture
  ```
  Ensures all single-digit tampering, bank code spoofing, and alphanumeric injection attacks are rejected.

### 2. Password Hashing & Timing Resistance (Argon2id)
* **Standard:** Argon2id (`v0.5.3`) with OWASP-recommended minimum memory (64 MB) and iterations (3).
* **Invariant:** Constant-time password verification preventing side-channel timing attacks.
* **Verification Command:**
  ```bash
  cargo test -p cv-auth
  cargo test --test security_auth_test -- security_argon2id_timing_resistance --nocapture
  ```

### 3. JWT Signing & Tamper Resistance
* **Standard:** HMAC-SHA256 with minimum 256-bit secret key entropy.
* **Invariant:** Forged claims, expired tokens, key substitution attacks, and malformed authorization headers must be rejected with HTTP 401.
* **Verification Command:**
  ```bash
  cargo test --test security_auth_test -- security_reject_forged_claims_jwt --nocapture
  cargo test --test security_auth_test -- security_reject_key_substitution_attack --nocapture
  ```

### 4. Zero Secrets in History Gate
* **Rule:** Never commit live API keys, private certificates, database credentials, or real JWT secrets.
* **Verification Command:**
  ```bash
  git diff --cached | grep -iE "PRIVATE KEY|BEGIN CERTIFICATE|AKIA[0-9A-Z]{16}|bearer [a-zA-Z0-9_\-\.]{30,}"
  ```
  Ensure zero matches in staged changes.

---

## Definition of Done
* All security integration tests pass (`security_auth_test`, `security_nuban_test`, `security_invariants_test`).
* No unhashed credentials or hardcoded private secrets exist in the codebase.
* Staged diff passes secrets grep.
