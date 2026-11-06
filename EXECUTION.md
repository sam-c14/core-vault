# CoreVault Execution Plan & Milestones

This document tracks execution milestones for CoreVault (Core Banking Application in Rust).

---

## Milestone Progress

- [x] **#1 — Dev Environment & Workspace Scaffolding**
  - Initialize Cargo workspace with modular crates.
  - Setup Docker Compose for PostgreSQL 16 and Valkey 7.2.
  - Provision partitioned database schema and General Ledger chart of accounts.
  - Setup multi-country configuration directory and environment templates.

- [x] **#2 — Core Banking Crates & Modular Monolith Implementation**
  - Implement `cv-types`: Exact financial math with `rust_decimal`, typed IDs, ISO currencies, extension traits.
  - Implement `cv-double-entry-engine`: Strict ledger invariant validation ($\sum \text{Debits} == \sum \text{Credits}$).
  - Implement `cv-nuban-validator`: CBN Mod-10 check digit algorithm and NUBAN generator.
  - Implement `cv-account-manager`: Account lifecycle state machine (`PENDING`, `ACTIVE`, `POST_NO_DEBIT`, `FROZEN`), balance holds, and liens.
  - Implement `cv-platform-config-parser`: TOML multi-country rule parser (`NG.toml`, `GB.toml`).
  - Implement `cv-audit`: Tamper-evident, chronological audit logging sink.
  - Implement `cv-auth`: Staff authentication, Argon2id password hashing, and JWT issuance/verification.
  - Implement `cv-transaction-api` & `corevault` binary: Axum REST controllers and server entrypoint.

- [x] **#3 — Postman & Newman API Test Suite**
  - Build comprehensive Postman collection covering 21 API scenarios across 7 functional areas.
  - Setup environment variables for local development (`http://127.0.0.1:8080`).
  - Add dynamic pre-request scripts for automatic idempotency key injection and random NUBAN generation.
  - Provide Newman CLI automated headless test runner documentation.

- [x] **#4 — Security Validation & Vulnerability Hardening**
  - Run security and invariant regression test suite.
  - Identify and remediate vulnerability `CV-SEC-001` (NUBAN bank code spoofing) in `cv-nuban-validator`.
  - Validate Zero Floating Point rule across all accounting crates.
  - Verify compile-time SQL parameterization to eliminate SQL injection vectors.
  - Publish formal security audit report (`docs/SECURITY_AUDIT.md`).

- [x] **#5 — Technical Documentation Suite**
  - Write `docs/ARCHITECTURE.md`: High-level architecture, transaction lifecycle, ledger schema design, and extension traits.
  - Write `docs/API_REFERENCE.md`: Complete documentation for all 21 REST endpoints with request/response schemas and curl examples.
  - Write `docs/DEVELOPMENT.md`: Developer onboarding, local environment setup, testing commands, and invariants.
  - Write `docs/PLATFORMIZATION.md`: Multi-country localization tutorial and TOML schema specification.
  - Update `README.md` with badges, documentation index, and verification metrics.

- [ ] **#6 — Private Enterprise Repository & Payment Rails**
  - Scaffold private repository for closed-source enterprise modules.
  - Implement `PaymentRailAdapter` for NIBSS NIP (ISO-8583 bridge) and agnostic Anchor rail.
  - Implement P2P Escrow Engine and Undo Transfer delay queue in Valkey.
  - Implement `ComplianceReporter` for CBN and NFIU automated XML generation.

- [x] **#7 — M2M Client Authentication, Admin Whitelisting & Automated PND Engine**
  - Tenant-as-Client domain unification: M2M API credentials (`client_id`, Argon2id `client_secret_hash`, `allowed_ips`, `scopes`) stored directly on the `tenants` table.
  - M2M client authentication endpoint (`POST /api/v1/auth/token`) issuing scoped JWTs; strict 403 Forbidden enforcement on `PENDING`, `BLACKLISTED`, or `SUSPENDED` clients.
  - Admin client management endpoints (`/api/v1/admin/tenants`): onboarding, whitelisting (`/whitelist`), blacklisting (`/blacklist`), and platform-level automatic violation blacklisting (`/simulate-violation`).
  - Tenant custom Post-No-Debit (PND) rule engine (`/api/v1/tenants/pnd-rules`) supporting KYC tier limits, daily debit caps, and velocity bursts.
  - Transfer route rule interception automatically applying `SYSTEM_AUTOMATED_PND` on rule breach (409 Conflict).
  - Post-No-Debit lifting endpoint (`POST /api/v1/accounts/:nuban/pnd/lift`) allowing tenants to remove both client manual and system automated PNDs.

- [x] **#8 — End-to-End PostgreSQL ACID Transactions & Complete Route Gating**
  - Replace in-memory mock checks with real PostgreSQL database queries and transactions.
  - Staff login (`POST /api/v1/auth/login`) verified against `users` table with Argon2id password hash and `is_active` check.
  - M2M token issuance (`POST /api/v1/auth/token`) verified against `tenants` table with Argon2id client secret hash and lifecycle status check.
  - Intra-bank transfers (`POST /api/v1/transfers/intra`) executed inside atomic PostgreSQL transactions (`BEGIN...COMMIT`) with row-level locks (`FOR UPDATE`), double-entry ledger validation, dual-leg `journal_entries` inserts (DR/CR), `transactions` record, and `audit_logs` record.
  - Account creation and lifecycle controls (`freeze`, `pnd`, `pnd/lift`, `activate`, `liens`, `liens/release`) executed as atomic DB transactions.
  - Strict authentication & authorization gating across all endpoints (`AuthClaims` extractor) enforcing Bearer JWT validation, role checks (`SUPER_ADMIN`, `OPERATIONS_OFFICER`, etc.), and scope checks (`transfers:write`, `accounts:read`, `accounts:write`, `admin:read`).
  - Purged all in-memory mock balance returns (e.g. 404 returned when account not found).

- [x] **#9 — Tenant Header Architecture, Client Basic Auth Login & Strict Cross-Tenant Isolation**
  - Extracted tenant identification to `x-tenant-id` HTTP header via axum extractor `TenantHeader`, removing `tenant_id` from JSON payloads and URL parameters.
  - Implemented Client Login (`POST /api/v1/auth/client-login` and `POST /api/v1/auth/login` with `Authorization: Basic base64(client_id:client_secret)`) accepting tenant credentials in HTTP Basic Auth and business user `email` and `password` in JSON body, with strict Argon2id hash verification and tenant scoping.
  - Guaranteed multi-tenant client-facing API isolation across all routes:
    - Account lookup (`GET /api/v1/accounts/:nuban`) returns 404 Not Found if queried by another tenant.
    - Account controls (`freeze`, `activate`, `pnd`, `pnd/lift`, `liens`, `liens/release`) return 404 Not Found on cross-tenant attempts.
    - Intra-bank and outward transfers return 403 Forbidden if a caller attempts to debit an account belonging to another tenant.
    - Custom tenant PND rules scoped strictly to the authenticated tenant.
  - Created end-to-end integration test suite `tenant_header_and_isolation_test.rs` validating client Basic Auth login, header extraction, and rejection of all cross-tenant access attempts.

- [x] **#10 — Paystack Payment Rail Adapter, Pluggable Rail Router & Postman Documentation Suite**
  - Implemented `cv-rail-paystack` crate:
    - Zero Floating Point precision conversions (`decimal_to_kobo`, `kobo_to_decimal`).
    - Paystack DTOs for account resolution (`/bank/resolve`), transfer recipient creation (`/transferrecipient`), single transfer initiation (`/transfer`), balance check (`/balance`), transfer status verification (`/transfer/verify/:reference`), commercial bank routing directory (`/bank`), and transfer finalization (`/transfer/finalize_transfer`).
    - Implemented `PaymentRailAdapter` for `PaystackRailAdapter` with deterministic sandbox simulation mode and live credential support.
  - Enhanced `cv-payment-gateway` and `PaymentGateway`:
    - Added first-class `PaymentRailProvider::Paystack` enum variant.
    - Added `check_balance` and `verify_transfer` default methods to `PaymentRailAdapter` trait.
    - Supported runtime payment rail switching between `Anchor` and `Paystack` via `POST /api/v1/admin/payment-rails/active`.
  - Expanded `cv-transaction-api` with multi-rail endpoints:
    - `GET /api/v1/payment-rails/balance`: Real-time operational settlement account balance check across active rails.
    - `GET /api/v1/transfers/verify/:reference`: External transaction reference verification on active rail.
    - `POST /api/v1/transfers/recipients`: Payout recipient registration returning `RCP_...` code.
    - `GET /api/v1/transfers/banks`: Commercial bank directory and CBN routing codes.
  - Integration and unit test suites:
    - Unit tests in `cv-rail-paystack` (conversions, name enquiry, recipient creation, transfer dispatch, balance, verify, bank listing).
    - Gateway tests in `cv-payment-gateway` (routing to Paystack, switching between Anchor and Paystack).
    - Comprehensive lifecycle integration tests in `crates/cv-transaction-api/tests/payment_rails_integration_test.rs`.
  - Comprehensive Postman & Newman Suite Update:
    - Re-organized collection into 10 domains with 45 requests, detailed descriptions for every folder, and standardized `mode: "raw"` JSON formatting on every POST, PUT, and PATCH request.
    - Embedded `x-tenant-id` header injection and Client Basic Auth login request.
    - Updated `postman/corevault_environment.json` and `postman/README.md`.
