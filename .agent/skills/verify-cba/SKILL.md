---
name: verify-cba
description: Executes the full test and code quality verification pipeline for CoreVault public core: cargo workspace tests, clippy strict linting (-D warnings), cargo fmt check, and Newman automated REST API collection runs.
---

# Verify CoreVault Public Core (CBA) Skill

Use this skill before opening a pull request, closing an `EXECUTION.md` milestone, or running `/commit-code`. A commit must never be created if any step in this verification pipeline fails.

---

## Pre-requisites
* Rust stable toolchain (`cargo`, `clippy`, `rustfmt`)
* Node.js & Newman CLI (for automated API collection tests)
* PostgreSQL 16 & Valkey 7.2 active (`docker compose up -d postgres valkey`)

---

## Step-by-Step Procedure

### 1. Code Formatting Gate
Verify that all source files conform to Rust standard formatting:
```bash
cargo fmt --all -- --check
```
*If this fails, run `cargo fmt --all` and re-verify.*

### 2. Strict Compiler & Clippy Linter Gate
Ensure zero compiler warnings across all crates and integration test suites:
```bash
cargo clippy --workspace --tests -- -D warnings
```
*Treat all linter suggestions as mandatory fixes.*

### 3. Cargo Workspace Unit & Integration Tests
Execute all automated test suites across every workspace crate:
```bash
cargo test --workspace
```
*Expected: 33+ unit and integration tests passing with 0 failures.*

### 4. Database Schema Integrity Check
Verify that PostgreSQL migrations compile cleanly and contain no unpartitioned ledger tables:
```bash
# Verify schema script syntax
cat migrations/20261022000000_init_corevault_schema.sql | grep -E "CREATE TABLE"
```

### 5. Automated REST API Verification (Newman CLI)
With the CoreVault server running locally on port 8080 (`cargo run --bin corevault`):
```bash
newman run postman/corevault_collection.json \
  -e postman/corevault_environment.json \
  --bail
```
*Expected: All 21 API scenarios pass (Health, Auth, Accounts, Controls, Transfers, Config, Audit).*

---

## Definition of Done
- `cargo fmt` passes with 0 changes.
- `cargo clippy` passes with 0 warnings.
- `cargo test --workspace` passes with 100% success rate.
- Newman test collection runs cleanly against live endpoints.
