---
name: banking-invariants
description: Enforces mathematical correctness and core banking invariants across all accounting crates: Zero Floating Point (f32/f64 prohibition), strictly balanced double-entry (Sum Debits == Sum Credits), compile-time parameterized SQL, and immutable append-only ledger operations.
---

# Banking Invariants & Correctness Skill

Use this skill before committing or completing any financial, ledger, account, or transaction feature to guarantee that CoreVault's mathematical banking invariants remain unbroken.

---

## Non-Negotiable Invariants

### 1. Absolute Zero Floating Point Rule
* **Rule:** Never use `f32` or `f64` in accounting logic, monetary representation, fee computation, or ledger storage.
* **Requirement:** Always use `rust_decimal::Decimal` and the typed `Money` struct from `cv_types`.
* **Verification Command:**
  ```bash
  grep -rnE "\b(f32|f64)\b" crates/cv-types/ crates/cv-double-entry-engine/ crates/cv-account-manager/ crates/cv-transaction-api/
  ```
  Ensure zero matches in financial calculation and storage code.

### 2. Strictly Balanced Double-Entry Rule
* **Rule:** Every committed financial transaction must satisfy the fundamental accounting equation:
  $$\sum_{\text{currency}} \text{Debits} == \sum_{\text{currency}} \text{Credits}$$
* **Requirement:** All multi-leg transactions must be validated through `cv_double_entry_engine::validate_journal_batch()` prior to committing.
* **Posting Legs:** Every posting leg amount must be strictly positive (`amount > Decimal::ZERO`). Negative posting legs are forbidden; use inverse entry types (`Debit` vs `Credit`).
* **Verification Command:**
  ```bash
  cargo test -p cv-double-entry-engine
  cargo test --test security_invariants_test
  ```

### 3. Compile-Time Parameterized SQL Rule
* **Rule:** Never interpolate, concatenate, or format raw SQL queries with user input.
* **Requirement:** Always use SQLx compile-time parameterized placeholders (`$1`, `$2`, `$3`).
* **Verification Command:**
  ```bash
  grep -rnE 'format!\(.*SELECT|format!\(.*INSERT|format!\(.*UPDATE|format!\(.*DELETE' src/ crates/
  ```
  Ensure zero matches.

### 4. Account Lifecycle & Restriction Enforcement
* **Rule:** An account restricted with `POST_NO_DEBIT` or `FROZEN` must never allow debit transactions.
* **Requirement:** Check account withdrawal permissions via `cv_account_manager::AccountStatus::can_debit()` before executing any debit leg.
* **Verification Command:**
  ```bash
  cargo test -p cv-account-manager
  ```

### 5. Immutable Append-Only Ledger
* **Rule:** Records in `cv_ledger_journal_entries` and `cv_ledger_postings` are immutable. Never execute `UPDATE` or `DELETE` on committed journal entries or postings.
* **Requirement:** Financial corrections must be performed exclusively via reversing entries.

---

## Verification Procedure

1. Run the floating point grep check.
2. Run SQL injection pattern check.
3. Run double-entry engine tests:
   ```bash
   cargo test -p cv-double-entry-engine -- --nocapture
   ```
4. Run integration invariant tests:
   ```bash
   cargo test --test security_invariants_test -- --nocapture
   ```

## Definition of Done
* Zero occurrences of `f32`/`f64` in financial paths.
* Zero raw SQL concatenations.
* All double-entry invariant tests pass with 0 errors.
