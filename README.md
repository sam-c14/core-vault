# CoreVault

> **CoreVault** is an enterprise-grade, cloud-native, strictly double-entry core banking platform engineered in **Rust**.
> Built for mathematical correctness, sub-millisecond execution, and 100% free & open-source infrastructure on the Oracle Cloud Infrastructure (OCI) Always Free Tier.

[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/rust-1.81%2B-orange.svg)](https://www.rust-lang.org)
[![PostgreSQL](https://img.shields.io/badge/postgresql-16-blue.svg)](https://www.postgresql.org)
[![Valkey](https://img.shields.io/badge/valkey-7.2-purple.svg)](https://valkey.io)
[![Tests](https://img.shields.io/badge/tests-33%20passed-success.svg)](docs/DEVELOPMENT.md)
[![Newman API](https://img.shields.io/badge/newman%20tests-21%2F21%20passed-success.svg)](postman/README.md)
[![Security Audit](https://img.shields.io/badge/security%20audit-passed%20(CV--SEC--001%20patched)-brightgreen.svg)](docs/SECURITY_AUDIT.md)

---

## 🏛️ Open Core Architecture

CoreVault strictly divides the mathematical primitives of banking from commercial network adapters and monetization features:

* **Public Core (This Repository):** Dual-licensed under **Apache 2.0 / MIT**. Contains the verifiable double-entry ledger engine, NUBAN mod-10 generator, account state machines, staff IAM, and REST transaction APIs.
* **Enterprise Layer (Closed-Source):** Contains proprietary payment switch integrations (NIBSS NIP ISO-8583 bridge), P2P escrow engines, "Undo Transfer" delay queues, smart transaction tagging, and automated CBN/NFIU regulatory reporting.

---

## 📦 Public Workspace Crates

| Crate | Purpose |
| :--- | :--- |
| [`crates/cv-types`](crates/cv-types) | Shared financial primitives (`rust_decimal` Money, Currencies, Account/Transaction IDs, and Extension Traits). |
| [`crates/cv-double-entry-engine`](crates/cv-double-entry-engine) | Mathematical double-entry ledger logic ($\sum \text{Debits} == \sum \text{Credits}$). |
| [`crates/cv-nuban-validator`](crates/cv-nuban-validator) | Nigerian Uniform Bank Account Number (NUBAN) CBN Mod-10 algorithm. |
| [`crates/cv-account-manager`](crates/cv-account-manager) | Account lifecycle state machine (`PENDING`, `ACTIVE`, `PND`, `FROZEN`), holds, and liens. |
| [`crates/cv-platform-config-parser`](crates/cv-platform-config-parser) | Declarative TOML parser for multi-country platformization (e.g. `NG.toml`, `GB.toml`). |
| [`crates/cv-audit`](crates/cv-audit) | Append-only audit logging traits and sinks. |
| [`crates/cv-auth`](crates/cv-auth) | Staff authentication, Argon2id password hashing, and JWT token management. |
| [`crates/cv-transaction-api`](crates/cv-transaction-api) | Axum REST router, controllers, and Valkey-backed idempotency guards. |

---

## 📖 Documentation Index

Complete specifications and guides are available in the [`docs/`](docs/) directory:

| Document | Purpose |
| :--- | :--- |
| [**Architecture Blueprint**](docs/ARCHITECTURE.md) | High-level system design, transaction lifecycles, ledger design, and extension trait interfaces. |
| [**REST API Reference**](docs/API_REFERENCE.md) | Full documentation of all 21 REST endpoints with request/response payloads, headers, and `curl` examples. |
| [**Developer Guide**](docs/DEVELOPMENT.md) | Local environment onboarding, Docker workflows, migrations, coding standards, and troubleshooting. |
| [**Multi-Country Platformization**](docs/PLATFORMIZATION.md) | Adding and customizing country-specific rules, KYC tiers, and currency configurations via TOML. |
| [**Security Audit Report**](docs/SECURITY_AUDIT.md) | Security vulnerability report, threat modeling, and verification of CV-SEC-001 patch. |
| [**Postman & Newman Suite**](postman/README.md) | 21-endpoint automated integration test collection and headless Newman CLI runner. |

---

## 🚀 Quick Start (Local Development)

### Prerequisites
* [Rust 1.81+](https://www.rust-lang.org/tools/install)
* [Docker & Docker Compose](https://docs.docker.com/get-docker/)
* [Newman CLI](https://www.npmjs.com/package/newman) (for automated API testing)

### 1. Spin up PostgreSQL & Valkey
```bash
docker compose up -d postgres valkey
```

### 2. Apply Database Schema Migrations
```bash
psql "postgres://cv_user:cv_password@localhost:5432/corevault_db" \
  -f migrations/20261022000000_init_corevault_schema.sql
```

### 3. Run the CoreVault Server
```bash
export DATABASE_URL="postgres://cv_user:cv_password@localhost:5432/corevault_db"
export VALKEY_URL="redis://localhost:6379"
export PORT=8080

cargo run --bin corevault
```

### 4. Verify Health
```bash
curl http://localhost:8080/health/live
# {"checks":{"database":"UP","valkey":"UP"},"service":"corevault-monolith","status":"READY"}
```

---

## 🧪 Automated Testing & Verification

### Run Rust Workspace Tests
```bash
cargo test --workspace
```
* **Result:** 33 unit and integration tests passing with 0 errors.

### Strict Clippy Verification
```bash
cargo clippy --workspace --tests -- -D warnings
```
* **Result:** Zero warnings or linter errors.

### Execute Newman API Collection
```bash
newman run postman/corevault_collection.json \
  -e postman/corevault_environment.json \
  --bail
```
* **Result:** All 21 endpoints and test assertions pass.

---

## 🛡️ Core Banking Invariants

* **Absolute Zero Floating Point:** Zero occurrences of `f32` or `f64` in core accounting and ledger calculation. All monetary calculations use `rust_decimal::Decimal`.
* **Zero Raw SQL String Concatenation:** All queries use compile-time parameterized statements (`$1`, `$2`), eliminating SQL injection risks.
* **Conservation of Value:** Every financial movement is double-entry verified: $\sum \text{Debits} == \sum \text{Credits}$ per currency batch.
* **Strict State Enforcement:** Accounts restricted with `POST_NO_DEBIT` or `FROZEN` automatically reject debit transactions.
