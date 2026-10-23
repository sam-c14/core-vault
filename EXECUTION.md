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
  - Implement `PaymentRailAdapter` for NIBSS NIP (ISO-8583 bridge).
  - Implement P2P Escrow Engine and Undo Transfer delay queue in Valkey.
  - Implement `ComplianceReporter` for CBN and NFIU automated XML generation.
