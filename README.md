# CoreVault

> **CoreVault** is an enterprise-grade, cloud-native, strictly double-entry core banking platform engineered in **Rust**.
> Built for mathematical correctness, sub-millisecond execution, and 100% free & open-source infrastructure on the Oracle Cloud Infrastructure (OCI) Always Free Tier.

[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg)](LICENSE-APACHE)
[![Rust](https://img.shields.io/badge/rust-1.81%2B-orange.svg)](https://www.rust-lang.org)
[![PostgreSQL](https://img.shields.io/badge/postgresql-16-blue.svg)](https://www.postgresql.org)
[![Valkey](https://img.shields.io/badge/valkey-7.2-purple.svg)](https://valkey.io)
[![Tests](https://img.shields.io/badge/tests-53%20passed-success.svg)](docs/DEVELOPMENT.md)
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
│  - Dual Client Login (HTTP Basic Auth + Business Credentials)         │
│  - Multi-Tenant Isolation & x-tenant-id Header Context Routing        │
│  - Pluggable Payment Gateway Router (cv-payment-gateway)               │
│  - Anchor BaaS Rail Adapter for SMEs & Fintechs (cv-rail-anchor)       │
│  - Tamper-Evident Chronological Audit Logging Sink                     │
│  - High-Throughput Axum HTTP REST Monolith Router                      │
├────────────────────────────────────────────────────────────────────────┤
│  ENTERPRISE LAYER (cba-enterprise) — Closed-Source / Proprietary       │
│  - NIBSS NIP Direct Payment Switch Adapter (cv-rail-nip / ISO-8583)    │
│  - 15-30s Cancellable Transfer Delay Queue (cv-undo-transfer)          │
│  - Trustless P2P Escrow Engine with OTP Handshakes (cv-escrow-p2p)     │
│  - AI Transaction Intelligence & Runway Analytics (cv-smart-tagging)   │
│  - Automated CBN Daily Returns & NFIU goAML Reporting (cv-compliance)  │
└────────────────────────────────────────────────────────────────────────┘
```

The extension boundary between the public core and private enterprise crates is governed by standard Rust traits defined in `cv-types`:
* [`PaymentRailAdapter`](crates/cv-types/src/traits.rs): Generic outward and inward inter-bank payment rail protocol. Supported out-of-the-box via Anchor BaaS; direct NIBSS NIP switch provided via `cba-enterprise`.
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
| [`crates/cv-rail-anchor`](crates/cv-rail-anchor) | Anchor Banking-as-a-Service ([getanchor.co](https://getanchor.co)) NIP adapter with Name Enquiry & Transfers. | Default SME payment rail without requiring direct NIBSS banking license. |
| [`crates/cv-payment-gateway`](crates/cv-payment-gateway) | Agnostic multi-rail router with dynamic adapter registration and Anchor default. | Pluggable outward payment rail abstraction. |
| [`crates/cv-transaction-api`](crates/cv-transaction-api) | Axum REST router, controllers, error mappers, and Valkey idempotency guards. | At-most-once execution via `Idempotency-Key`. |

---

## 🤖 Agent & Developer Skills (`.agent/skills/`)

CoreVault equips autonomous AI agents and human contributors with specialized skills in [`.agent/skills/`](.agent/skills/) to enforce code safety, maintain context, and ensure engineering quality:

| Skill | Invocation | Purpose & Guardrails |
| :--- | :--- | :--- |
| [**`commit-code`**](.agent/skills/commit-code/SKILL.md) | `/commit-code` | Consistent Conventional Commits pattern for closing out milestones referenced in [`EXECUTION.md`](EXECUTION.md). Enforces clean working trees, zero secrets, and pre-commit test gates. |
| [**`cba-integration`**](.agent/skills/cba-integration/SKILL.md) | `/cba-integration` | Consumer integration guide for CoreVault CBA — M2M client credentials, Basic Auth client login, `x-tenant-id` header scoping, and Open Core vs. Enterprise feature tiers. |
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
| **Enterprise Commercial Specs** | *Companion Repo* | Full commercial PRDs, monetization roadmaps, and fee specifications are maintained in `cba-enterprise/docs/`. |

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
* **Conservation of Value:** Every financial movement is double-entry verified: $\sum \text{Debits} == \sum \text{Credits}$ per currency batch.# SKILL: Playwright QA & Visual Alignment Engineer

## CONTEXT
You are an expert QA Automation Engineer specializing in Playwright. Your responsibility is to test the Next.js PWA consumer wallet within the Nx monorepo. 

**CRITICAL CONSTRAINT:** The underlying Core Banking App (CBA) and Rust API Gateway are currently in active development. You **MUST NOT** attempt to call real backend endpoints during your tests. Instead, you must heavily rely on Playwright's Network Interception (`page.route()`) to mock all API requests and responses. The frontend is being built with full API integration code (TanStack Query/fetch), but your tests must satisfy those calls with mocks.

## CORE RESPONSIBILITIES

### 1. Visual & Layout Alignment Testing
The design requires a highly responsive, Cowrywise-inspired layout that completely transforms between desktop and mobile. You must write tests that explicitly check for proper rendering across viewports:
*   **Desktop Viewport (>= 1024px):** Validate the presence and alignment of the fixed left-hand navigation sidebar (260px width) and the full-width transaction data tables.
*   **Mobile Viewport (< 1024px):** Validate the presence of the fixed bottom navigation bar (64px height), the top app-bar, and ensure modal dialogs successfully transform into bottom-sheet slide-overs.
*   **Design System Compliance:** Ensure assertions check that the correct design system utilities and brand colors are applied (e.g., verifying the Main Balance Card gradient and touch target sizes are >= 48x48px on mobile).

### 2. Comprehensive API Mocking
Since the CBA is unavailable, you must intercept and mock the following core flows using Playwright's `page.route`:
*   **KYC / Tiered Onboarding:** Mock the `POST /api/v1/auth/verify-otp` (Tier 0), `POST /api/v1/kyc/basic` (Tier 1), and `POST /api/v1/kyc/bvn` (Tier 2) endpoints to simulate successful and failed validation states.
*   **Balances & Ledgers:** Mock `GET /api/v1/wallet/balance` to return JSON payloads mimicking kobo values (e.g., `{ "balance_kobo": 124550000 }`) and ensure the UI correctly formats it to Naira (`₦ 1,245,500.00`).
*   **Transaction History:** Mock paginated/infinite-scroll responses for `GET /api/v1/transactions`.
*   **Name Enquiry & Outbound Transfers:** Intercept NUBAN resolution requests (`GET /api/v1/transfers/resolve`) to return dummy account names, and mock the final `POST /api/v1/transfers/execute`.

### 3. Execution & Integration within Nx
*   Ensure all tests are written in TypeScript and placed within the appropriate `apps/web-e2e/` (or similar) directory generated by Nx.
*   Tests should be runnable via the Nx CLI: `nx run web-e2e:e2e`.
*   Group tests logically using `test.describe()` for "Desktop Layout", "Mobile Layout", "Onboarding Flows", and "Transaction Flows".
* **Strict State Enforcement:** Accounts restricted with `POST_NO_DEBIT` or `FROZEN` automatically reject debit transactions.
* **Idempotent Mutations:** All state-altering API requests require a unique `Idempotency-Key` (UUIDv4) header cached in Valkey.
