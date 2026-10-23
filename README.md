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

## 🏛️ Open Core Architecture Model

CoreVault strictly divides generic, verifiable banking primitives from commercial network adapters and monetization features:

```
┌────────────────────────────────────────────────────────────────────────┐
│                      COREVAULT OPEN CORE ECOSYSTEM                     │
├────────────────────────────────────────────────────────────────────────┤
│  PUBLIC OPEN-SOURCE CORE (This Repository) — Apache 2.0 / MIT          │
│  - Mathematical Double-Entry Engine (rust_decimal, Zero Floats)        │
│  - NUBAN Mod-10 Algorithm & Prefix Validation (CV-SEC-001 Hardened)    │
│  - Account Lifecycle State Machine (ACTIVE, PND, FROZEN, CLOSED)       │
│  - Financial Hold & Lien Management                                    │
│  - Declarative Multi-Country Platformization TOML Engine               │
│  - Staff IAM & Argon2id Password Hashing with JWT Tokens               │
│  - Tamper-Evident Chronological Audit Logging Sink                     │
│  - High-Throughput Axum HTTP REST Monolith Router                      │
├────────────────────────────────────────────────────────────────────────┤
│  ENTERPRISE LAYER (cba-enterprise) — Closed-Source / Proprietary       │
│  - Agnostic Multi-Rail Payment Gateway (cv-payment-gateway)            │
│  - Anchor BaaS Rail Adapter for SMEs (cv-rail-anchor / getanchor.co)   │
│  - NIBSS NIP Direct Payment Switch Adapter (cv-rail-nip / ISO-8583)    │
│  - 15-30s Cancellable Transfer Delay Queue (cv-undo-transfer)          │
│  - Trustless P2P Escrow Engine with OTP Handshakes (cv-escrow-p2p)     │
│  - AI Transaction Intelligence & Runway Analytics (cv-smart-tagging)   │
│  - Automated CBN Daily Returns & NFIU goAML Reporting (cv-compliance)  │
└────────────────────────────────────────────────────────────────────────┘
```

The extension boundary between the public core and private enterprise crates is governed by standard Rust traits defined in `cv-types`:
* [`PaymentRailAdapter`](crates/cv-types/src/traits.rs): Generic outward and inward inter-bank payment rail protocol.
* [`ComplianceReporter`](crates/cv-types/src/traits.rs): Transaction threshold screening and regulatory export interface.

---

## 📦 Public Workspace Crates

| Crate | Purpose | Core Invariant |
| :--- | :--- | :--- |
| [`crates/cv-types`](crates/cv-types) | Shared domain primitives (`Money`, `Currency`, `AccountId`, `TransactionId`) and enterprise extension traits. | Absolute zero `f32`/`f64` usage. |
| [`crates/cv-double-entry-engine`](crates/cv-double-entry-engine) | Mathematical double-entry ledger logic. | $\sum \text{Debits} == \sum \text{Credits}$ per currency batch. |
| [`crates/cv-nuban-validator`](crates/cv-nuban-validator) | Nigerian Uniform Bank Account Number (NUBAN) CBN Mod-10 algorithm & generator. | Bank-code prefix enforcement & check-digit verification. |
| [`crates/cv-account-manager`](crates/cv-account-manager) | Account lifecycle state machine (`PENDING`, `ACTIVE`, `PND`, `FROZEN`), holds, and liens. | Strict debit restrictions on `PND` and `FROZEN` accounts. |
| [`crates/cv-platform-config-parser`](crates/cv-platform-config-parser) | Declarative TOML parser for multi-country platformization (e.g. `NG.toml`, `GB.toml`). | Zero-recompilation country profile loading. |
| [`crates/cv-audit`](crates/cv-audit) | Append-only audit logging traits and in-memory/persistent sinks. | Chronological tamper-evident audit trail. |
| [`crates/cv-auth`](crates/cv-auth) | Staff authentication, Argon2id password hashing, and JWT token management. | Constant-time password verification & 256-bit entropy. |
| [`crates/cv-transaction-api`](crates/cv-transaction-api) | Axum REST router, controllers, error mappers, and Valkey idempotency guards. | At-most-once execution via `Idempotency-Key`. |

---

## 🤖 Agent & Developer Skills (`.agent/skills/`)

CoreVault equips autonomous AI agents and human contributors with specialized skills in [`.agent/skills/`](.agent/skills/) to enforce code safety, maintain context, and ensure engineering quality:

| Skill | Invocation | Purpose & Guardrails |
| :--- | :--- | :--- |
| [**`commit-code`**](.agent/skills/commit-code/SKILL.md) | `/commit-code` | Consistent Conventional Commits pattern for closing out milestones referenced in [`EXECUTION.md`](EXECUTION.md). Enforces clean working trees, zero secrets, and pre-commit test gates. |
| [**`banking-invariants`**](.agent/skills/banking-invariants/SKILL.md) | `/banking-invariants` | Audits code for non-negotiable financial invariants: zero floating point (`f32`/`f64` prohibition), double-entry balance equality, parameterized SQL, and account debit permissions. |
| [**`verify-cba`**](.agent/skills/verify-cba/SKILL.md) | `/verify-cba` | Executes the complete verification pipeline: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --workspace` (33 tests), and Newman CLI API test runs. |
| [**`security-audit`**](.agent/skills/security-audit/SKILL.md) | `/security-audit` | Runs automated security regression tests: Argon2id timing resistance, JWT tamper resistance, NUBAN check-digit tampering prevention (CV-SEC-001), and secrets scanning. |
| [**`platform-rules`**](.agent/skills/platform-rules/SKILL.md) | `/platform-rules` | Validates, tests, and parses multi-country localization TOML files in `config/jurisdictions/` for currency precision, account formats, KYC limits, and tax levies. |

---

## 📖 Comprehensive Documentation Index

All architectural blueprints, PRDs, API specs, and development guides are maintained in [`docs/`](docs/) and the root directory:

| Document | File Link | Focus & Highlights |
| :--- | :--- | :--- |
| **System Architecture** | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | High-level system design, transaction lifecycle sequence diagrams, database schema model, and extension traits. |
| **REST API Reference** | [`docs/API_REFERENCE.md`](docs/API_REFERENCE.md) | Complete specification for all 21 REST endpoints with request/response schemas, HTTP status codes, headers, and `curl` examples. |
| **Developer Onboarding** | [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) | Local environment setup, Docker workflows, PostgreSQL migrations, banking invariants, and troubleshooting guide. |
| **Platformization Guide** | [`docs/PLATFORMIZATION.md`](docs/PLATFORMIZATION.md) | Multi-country localization tutorial explaining TOML schemas (`[jurisdiction]`, `[account_standard]`, `[kyc_tiers]`). |
| **Security Audit Report** | [`docs/SECURITY_AUDIT.md`](docs/SECURITY_AUDIT.md) | Formal security audit detailing threat modeling, zero-float verification, and remediation of vulnerability `CV-SEC-001`. |
| **Milestone Execution** | [`EXECUTION.md`](EXECUTION.md) | Completed and active development milestones tracking Conventional Commits and feature progress. |
| **Postman & Newman Suite** | [`postman/README.md`](postman/README.md) | Automated 21-scenario integration test suite, environment variables, and headless Newman CLI runner instructions. |
| **Modular Monolith PRD** | [`docs/PRD-monolith-v2.md`](docs/PRD-monolith-v2.md) | Definitive Modular Monolith PRD and OCI Always Free Tier deployment architecture. |
| **Open Core Boundary Spec** | [`docs/PRD-proposed-diff-for-public-and-private-items.md`](docs/PRD-proposed-diff-for-public-and-private-items.md) | Detailed separation boundary between public core crates and private enterprise crates. |
| **Architecture Review** | [`docs/PRD-monolith-review.md`](docs/PRD-monolith-review.md) | Review of monolith architectural patterns, trade-offs, and microservices transition triggers. |
| **Original System PRD** | [`docs/PRD.md`](docs/PRD.md) | Original CoreVault core banking platform functional and regulatory specification. |

---

## 🚀 Quick Start (Local Development)

### Prerequisites
* [Rust 1.81+](https://www.rust-lang.org/tools/install)
* [Docker & Docker Compose](https://docs.docker.com/get-docker/)
* [PostgreSQL Client (`psql`)](https://www.postgresql.org)
* [Newman CLI](https://www.npmjs.com/package/newman) (optional, for automated API testing)

### 1. Spin up PostgreSQL 16 & Valkey 7.2
```bash
docker compose up -d postgres valkey
```

### 2. Apply Database Schema & Seed Data
```bash
psql "postgres://cv_user:cv_password@localhost:5432/corevault_db" \
  -f migrations/20261022000000_init_corevault_schema.sql
```

### 3. Run the Monolith HTTP Server
```bash
export DATABASE_URL="postgres://cv_user:cv_password@localhost:5432/corevault_db"
export VALKEY_URL="redis://localhost:6379"
export PORT=8080
export JWT_SECRET="super_secret_jwt_key_change_in_production_123456"

cargo run --bin corevault
```

### 4. Verify System Health
```bash
curl -s http://127.0.0.1:8080/health/live | jq .
curl -s http://127.0.0.1:8080/health/ready | jq .
```

---

## 🧪 Automated Testing & Verification

### Run Rust Workspace Tests
```bash
cargo test --workspace
```
* **Result:** 33 unit, integration, and security tests passing with 0 errors.

### Strict Clippy Verification
```bash
cargo clippy --workspace --tests -- -D warnings
```
* **Result:** Zero warnings or linter errors.

### Execute Headless Newman API Suite
With the server running on `http://127.0.0.1:8080`:
```bash
newman run postman/corevault_collection.json \
  -e postman/corevault_environment.json \
  --bail
```
* **Result:** All 21 endpoints and test assertions pass.

---

## 🛡️ Non-Negotiable Core Banking Invariants

* **Absolute Zero Floating Point:** Zero occurrences of `f32` or `f64` in core accounting and ledger calculation. All monetary calculations use `rust_decimal::Decimal`.
* **Zero Raw SQL String Concatenation:** All queries use compile-time parameterized statements (`$1`, `$2`), eliminating SQL injection risks.
* **Conservation of Value:** Every financial movement is double-entry verified: $\sum \text{Debits} == \sum \text{Credits}$ per currency batch.
* **Strict State Enforcement:** Accounts restricted with `POST_NO_DEBIT` or `FROZEN` automatically reject debit transactions.
* **Idempotent Mutations:** All state-altering API requests require a unique `Idempotency-Key` (UUIDv4) header cached in Valkey.
