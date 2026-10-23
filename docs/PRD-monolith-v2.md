# Product Requirements Document (PRD)
# CoreVault — MVP / V1 Modular Monolith (Open Core Edition)

> **Document Version:** 2.2.0 (MVP Modular Monolith & Open Core Architecture)  
> **Date:** October 2026  
> **Status:** Approved Implementation PRD  
> **Target Deployment:** Oracle Cloud Infrastructure (OCI) Always Free Tier (ARM Ampere A1)  
> **Primary Jurisdiction:** Nigeria (CBN / NIBSS Compliant) + Pluggable Multi-Country Platform  
> **File:** `docs/PRD-monolith-v2.md`

---

## Table of Contents

1. [Executive Summary & Product Vision](#1-executive-summary--product-vision)
2. [Open Core Licensing & Repository Model](#2-open-core-licensing--repository-model)
3. [V1 MVP Constraints & Infrastructure Simplifications](#3-v1-mvp-constraints--infrastructure-simplifications)
4. [Oracle Cloud Always Free Tier Deployment Blueprint](#4-oracle-cloud-always-free-tier-deployment-blueprint)
5. [System Architecture Overview](#5-system-architecture-overview)
6. [Public Core Crates (Open Source)](#6-public-core-crates-open-source)
7. [Enterprise Crates (Closed Source / Commercial)](#7-enterprise-crates-closed-source--commercial)
8. [Domain Modules (V1 Specification)](#8-domain-modules-v1-specification)
   - 8.1 [Authentication, IAM & Admin Module](#81-authentication-iam--admin-module)
   - 8.2 [Customer Profiles & KYC Tiers](#82-customer-profiles--kyc-tiers)
   - 8.3 [Accounts & NUBAN Engine](#83-accounts--nuban-engine)
   - 8.4 [Double-Entry Ledger Engine](#84-double-entry-ledger-engine)
   - 8.5 [Transactions & Journal Processing](#85-transactions--journal-processing)
   - 8.6 [Balance Materialization & Concurrency Strategy](#86-balance-materialization--concurrency-strategy)
9. [Complete PostgreSQL 16 Database Schema](#9-complete-postgresql-16-database-schema)
10. [API Surface Specification (REST & gRPC)](#10-api-surface-specification-rest--grpc)
11. [Security & Secrets Management (V1)](#11-security--secrets-management-v1)
12. [Observability, Health Checks & Logging](#12-observability-health-checks--logging)
13. [Build, Cross-Compilation & Container Packaging](#13-build-cross-compilation--container-packaging)
14. [Non-Functional Requirements & MVP Acceptance Criteria](#14-non-functional-requirements--mvp-acceptance-criteria)
15. [Glossary](#15-glossary)

---

## 1. Executive Summary & Product Vision

**CoreVault** is an enterprise-grade core banking engine built in **Rust** using a **Modular Monolith** architecture for Version 1. It is designed to solve the two biggest failure points of modern core banking projects:
1. **Premature Distributed System Sprawl:** Instead of managing 10 microservices, Kubernetes clusters, Kafka, OpenBao, and multi-node service meshes on day one, V1 compiles into a **single, highly optimized Rust binary** backed by **PostgreSQL 16** and **Valkey 7/8**.
2. **Commercial Viability via Open Core:** CoreVault separates its verifiable, mathematically provable double-entry accounting engine into a public open-source core, while retaining high-value commercial integrations (NIBSS instant settlement, escrow, undo-transfer, regulatory compliance reporting) in a proprietary closed-source enterprise layer.

The entire V1 production deployment fits comfortably inside the **Oracle Cloud Infrastructure (OCI) Always Free Tier** (4 OCPU ARM Ampere A1, 24 GB RAM, 200 GB SSD) with zero cloud hosting bills.

---

## 2. Open Core Licensing & Repository Model

CoreVault strictly divides the mathematical primitives of banking from commercial network adapters and monetization features.

```
┌────────────────────────────────────────────────────────────────────────┐
│ PUBLIC CORE REPOSITORY (Apache 2.0 / MIT)                              │
│ github.com/your-org/corevault                                          │
│                                                                        │
│ • cv-types (Money, AccountId, TxnState)                                │
│ • cv-double-entry-engine (Sum Debits == Sum Credits Invariant)         │
│ • cv-nuban-validator (CBN Mod-10 Check Digit)                         │
│ • cv-account-manager (Account State Machine, Row-Level Locking)        │
│ • cv-transaction-api (REST/gRPC Base Endpoints, Valkey Idempotency)    │
│ • cv-platform-config-parser (TOML Localization Schema Parser)          │
│ • cv-audit (Append-Only Audit Log Trail)                               │
└───────────────────────────────────┬────────────────────────────────────┘
                                    │ Cargo Dependency / Dynamic Trait Impl
┌───────────────────────────────────▼────────────────────────────────────┐
│ PRIVATE ENTERPRISE REPOSITORY (Commercial Proprietary License)        │
│ github.com/your-org/corevault-enterprise                               │
│                                                                        │
│ • cv-nibss-gateway (NIP Inward/Outward ISO-8583 / REST Bridge)         │
│ • cv-escrow-engine (P2P Holding Accounts, OTP Handshakes, Logistics)   │
│ • cv-delayed-settlement ("Undo Transfer" Buffer Queue)                 │
│ • cv-smart-tags (Merchant Binding, Smart Routing, Runway Analytics)   │
│ • cv-compliance-cbn (Automated XML Returns for NFIU / CBN)             │
│ • cv-dispute-resolution (Chargeback Logger, RRN Arbiter)              │
│ • cv-currency-fx (Multi-Currency FX Spread & Conversion Engine)        │
│ • cv-fee-engine (Tiered, Capped, Percentage Fee Schedules)             │
└────────────────────────────────────────────────────────────────────────┘
```

### 2.1 Workspace Structure

```
corevault-monorepo/ (or enterprise wrapper repo)
├── Cargo.toml                          # Workspace root
│
├── crates/                             # ── PUBLIC OPEN SOURCE (Apache 2.0) ──
│   ├── cv-types/                       # Domain primitives (rust_decimal, IDs)
│   ├── cv-double-entry-engine/         # Immutable double-entry ledger logic
│   ├── cv-nuban-validator/             # NUBAN mod-10 generator & validator
│   ├── cv-account-manager/             # Account lifecycles & balance updates
│   ├── cv-transaction-api/             # Axum handlers for standard ledger ops
│   ├── cv-platform-config-parser/      # Declarative jurisdiction TOML parser
│   └── cv-audit/                       # Audit logging traits and PostgreSQL sinks
│
├── crates-enterprise/                  # ── CLOSED SOURCE (Proprietary) ──
│   ├── cv-nibss-gateway/               # NIBSS NIP integration & Name Enquiry
│   ├── cv-escrow-engine/               # Trust accounts, milestones & releases
│   ├── cv-delayed-settlement/          # Cancelable pending transfer buffer
│   ├── cv-smart-tags/                  # AI/rule-based categorization & analytics
│   ├── cv-compliance-cbn/              # CBN monthly return XML & CTR/STR reports
│   ├── cv-dispute-resolution/          # Automated POS/Web dispute arbiter
│   ├── cv-currency-fx/                 # Multi-currency matrix & revaluation
│   └── cv-fee-engine/                  # Configurable fee calculation rules
│
└── src/
    └── main.rs                         # Monolith application assembly & boot
```

### 2.2 Trait-Based Extension Mechanism

The public core defines extension traits. The binary detects feature flags or enterprise plugins at compile/runtime:

```rust
// crates/cv-types/src/traits.rs (PUBLIC)
#[async_trait::async_trait]
pub trait PaymentRailAdapter: Send + Sync {
    async fn name_enquiry(&self, bank_code: &str, account_number: &str) -> Result<AccountEnquiry, RailError>;
    async fn dispatch_outward(&self, transfer: &OutwardTransfer) -> Result<TransferReceipt, RailError>;
}

#[async_trait::async_trait]
pub trait ComplianceReporter: Send + Sync {
    async fn evaluate_transaction(&self, txn: &TransactionRecord) -> Result<ComplianceDisposition, ComplianceError>;
}
```

In the public open-source build, a `MockPaymentRailAdapter` and `NullComplianceReporter` are compiled in. The enterprise binary binds `NibssPaymentRail` and `CbnComplianceEngine`.

---

## 3. V1 MVP Constraints & Infrastructure Simplifications

To avoid the operational burden of a distributed cluster, V1 adopts strict, pragmatic engineering simplifications:

| Distributed Tool (Discarded for V1) | V1 MVP Replacement | Pragmatic Justification |
| :--- | :--- | :--- |
| **Apache Kafka + RabbitMQ** | **PostgreSQL `LISTEN/NOTIFY` + Valkey Streams** | Avoids 500 MB+ JVM footprint and Zookeeper/KRaft brokers. Valkey Streams handle at-least-once task dispatching. |
| **OpenSearch / Elasticsearch** | **PostgreSQL `pg_trgm` + `tsvector`** | Trigram fuzzy search on indexed customer names and BVN handles millions of records in < 5 ms. |
| **HashiCorp Vault / OpenBao** | **Rust Application AES-256-GCM + Linux Env** | Sensitive PII (BVN, NIN) is encrypted at rest using AES-GCM before SQL writes. Master keys injected via protected env vars. |
| **Google Spanner / YugabyteDB** | **PostgreSQL 16 Partitioned Tables** | Date-partitioned transaction tables easily sustain > 5,000 TPS on a single Postgres instance with SSD storage. |
| **Apache APISIX + etcd** | **Caddy Web Server (Apache 2.0)** | Caddy is a single 40 MB binary providing automatic Let's Encrypt TLS, HTTP/2 reverse proxy, and rate limiting with zero external dependencies. |

---

## 4. Oracle Cloud Always Free Tier Deployment Blueprint

The production target for V1 is the **Oracle Cloud Infrastructure (OCI) Always Free Tier**.

### 4.1 OCI Free Tier Allocations vs. CoreVault V1 Footprint

| Resource | OCI Free Tier Limit | CoreVault V1 Allocation | Free Tier Headroom |
| :--- | :--- | :--- | :--- |
| **Compute Architecture** | ARM Ampere A1 (4 OCPU total) | 1 VM with 4 OCPU (or 2 VMs with 2 OCPU) | 100% covered |
| **RAM** | 24 GB total | **8 GB allocated** to VM services | 16 GB remaining |
| **Block Storage** | 200 GB total | 80 GB SSD (PostgreSQL + OS) | 120 GB remaining |
| **Bandwidth** | 10 TB outbound / month | ~50 GB / month estimated for MVP | > 99% remaining |
| **Public IPv4** | 1 Ephemeral / Reserved IP | 1 Reserved Public IPv4 | Free |

### 4.2 Single-Node Host Resource Budget (ARM A1: 4 OCPU / 24 GB RAM)

```
┌────────────────────────────────────────────────────────────────────────┐
│ OCI Always Free ARM Ampere A1 VM (Oracle Linux 9 / Ubuntu 24.04 ARM64)  │
│                                                                        │
│   ┌──────────────────────────┐     ┌───────────────────────────────┐   │
│   │ Caddy Reverse Proxy      │     │ Valkey 7/8 In-Memory Cache    │   │
│   │ • Ports: 80 / 443        │     │ • Port: 6379 (Localhost only) │   │
│   │ • Automatic Let's Encrypt│     │ • RAM: Max 512 MB             │   │
│   │ • RSS: ~35 MB            │     │ • Persistence: RDB snapshots  │   │
│   └────────────┬─────────────┘     └───────────────▲───────────────┘   │
│                │ Proxy pass                        │                   │
│   ┌────────────▼─────────────┐                     │ Lock / Idempotency│
│   │ CoreVault Rust Monolith  │─────────────────────┘                   │
│   │ • Port: 8080 (REST)      │                                         │
│   │ • Port: 50051 (gRPC)     │─────────────────────┐                   │
│   │ • 4 Tokio worker threads │                     │ sqlx connection   │
│   │ • RSS: ~45 MB (Idle)     │                     │ pool (max 25)     │
│   │ • Under load: < 250 MB   │                     │                   │
│   └──────────────────────────┘     ┌───────────────▼───────────────┐   │
│                                    │ PostgreSQL 16 (Local NVMe)    │   │
│                                    │ • Port: 5432 (Localhost only) │   │
│                                    │ • shared_buffers: 4 GB        │   │
│                                    │ • effective_cache_size: 12 GB │   │
│                                    │ • WAL: replica / date-part.   │   │
│                                    └───────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────────┘
```

---

## 5. System Architecture Overview

```mermaid
flowchart TD
    subgraph Clients["Access Channels"]
        WEB["Fintech Web App"]
        MOB["Mobile App (Flutter / React Native)"]
        ADMIN["Internal Ops Web Portal"]
        SWITCH["NIBSS NIP Payment Network"]
    end

    subgraph Perimeter["Edge & Reverse Proxy"]
        CADDY["Caddy Server (TLS 1.3 / Auto Let's Encrypt / Rate Limiting)"]
    end

    subgraph CoreVault["CoreVault Monolith Binary (Rust Axum + Tokio)"]
        direction TB
        ROUTER["Axum HTTP Router & Tonic gRPC Service"]

        subgraph OpenSource["Public Core Crates"]
            AUTH["cv-auth (JWT / Argon2)"]
            NUBAN["cv-nuban-validator"]
            ACC["cv-account-manager"]
            LEDGER["cv-double-entry-engine"]
            AUDIT["cv-audit"]
        end

        subgraph Proprietary["Closed-Source Enterprise Crates"]
            NIP["cv-nibss-gateway"]
            ESCROW["cv-escrow-engine"]
            DELAY["cv-delayed-settlement"]
            TAGS["cv-smart-tags"]
            REPORTS["cv-compliance-cbn"]
        end

        ROUTER --> AUTH
        ROUTER --> ACC
        ROUTER --> LEDGER
        ROUTER --> NIP
        ROUTER --> ESCROW
        ROUTER --> DELAY
    end

    subgraph Persistence["Storage & State (Co-located on VM)"]
        VALKEY[("Valkey 7/8\nIdempotency Keys, Rate Limits, Locks")]
        PG[("PostgreSQL 16\nMaster Balances, Journals, Customers, Audit")]
    end

    Clients --> CADDY
    CADDY --> ROUTER
    CoreVault <--> VALKEY
    CoreVault <--> PG
```

---

## 6. Public Core Crates (Open Source)

The public repository provides the auditable, verifiable financial core:

### 6.1 `crates/cv-types`
* **Currency Precision:** Wraps `rust_decimal::Decimal` with safety invariants. No financial amount can be initialized from a floating-point primitive without explicit truncation.
* **ISO-4217 Currency Representation:** Typed currencies (`NGN`, `USD`, `GBP`, `EUR`) with minor unit definitions.
* **Standard Domain Identifiers:** Strongly-typed IDs using UUIDv7 (`AccountId`, `TenantId`, `TransactionId`, `CustomerId`).

### 6.2 `crates/cv-double-entry-engine`
* Strictly validates financial journal batches before database serialization.
* Enforces:
  $$\sum \text{Debits} == \sum \text{Credits} \quad (\text{per currency})$$
* Rejects any transaction containing negative amounts, zero amounts, or fewer than 2 posting legs.

### 6.3 `crates/cv-nuban-validator`
* **Nigerian NUBAN Algorithm:** Full implementation of CBN Modulo-10 with weights `[3, 7, 3, 3, 7, 3, 3, 7, 3]`.
* Provides both account number generation from serial numbers and check-digit validation.

### 6.4 `crates/cv-account-manager`
* State machine managing account lifecycles: `PENDING_OPENING` $\to$ `ACTIVE` $\leftrightarrow$ `DORMANT` $\leftrightarrow$ `POST_NO_DEBIT` $\leftrightarrow$ `FROZEN` $\to$ `CLOSED`.
* Manages balance inquiries, holds/liens, and overdraft limit checks.

### 6.5 `crates/cv-transaction-api`
* REST and gRPC request handlers for initiating transfers, balance lookups, and account statements.
* Valkey-backed atomic idempotency middleware (`SETNX` with 120-second in-flight lock, 24-hour result cache).

### 6.6 `crates/cv-platform-config-parser`
* Reads declarative TOML files defining country parameters (currency, minor units, account number regex, national identity formats).

### 6.7 `crates/cv-audit`
* Standardized audit logging traits with asynchronous PostgreSQL batch-writing.

---

## 7. Enterprise Crates (Closed Source / Commercial)

The private repository contains high-value market integrations and monetization features:

### 7.1 `crates/cv-nibss-gateway`
* Direct integration with the **Nigeria Inter-Bank Settlement System (NIBSS) Instant Payment (NIP)** network:
  - **Name Enquiry:** Resolves counterparty account names across all Nigerian commercial banks and fintechs.
  - **Funds Transfer (Outward Direct Debit):** Initiates interbank settlements with ISO-8583 message packing and unpacking.
  - **Inward Transaction Webhook Receiver:** Validates incoming NIBSS notifications, checks transaction reference uniqueness, and posts to customer accounts.

### 7.2 `crates/cv-escrow-engine`
* Trust and escrow transaction engine for P2P and merchant commerce:
  - Funds locked in an internal `SUSPENSE_ESCROW` general ledger account.
  - Releases funds upon cryptographic OTP verification, buyer confirmation, or logistics webhook triggers.
  - Automated refund state machine on dispute timeout.

### 7.3 `crates/cv-delayed-settlement` ("Undo Transfer")
* Consumer protection feature:
  - Holds outward transfer funds in a `PENDING_OUTBOX` state for a configurable window (e.g., 30–120 seconds).
  - Allows customer cancellation via mobile app before the transfer is dispatched to NIBSS.
  - Background worker thread flushes matured transfers to the payment switch.

### 7.4 `crates/cv-smart-tags` & Financial Intelligence
* Rule-based and semantic tagging engine for transaction narration strings.
* Real-time categorization (e.g., `#groceries`, `#utilities`, `#salary`).
* Automatic merchant identity binding and monthly runway calculations.

### 7.5 `crates/cv-compliance-cbn`
* Generates mandatory Central Bank of Nigeria (CBN) and Nigerian Financial Intelligence Unit (NFIU) regulatory returns:
  - **Currency Transaction Reports (CTR):** Mandatory reporting for transactions exceeding ₦5,000,000 (individual) or ₦10,000,000 (corporate).
  - **Suspicious Transaction Reports (STR):** Automated XML generation with PGP encryption for direct upload.

### 7.6 `crates/cv-dispute-resolution`
* Handles POS dispense errors, ATM failed cash dispensations, and interbank transfer reversals.
* Packages Retrieval Reference Numbers (RRN) and initiates automated claims against the NIBSS Arbitration platform.

### 7.7 `crates/cv-currency-fx`
* Multi-currency conversion matrix with spread calculation, rate lock windows, and unrealized FX gain/loss revaluations.

### 7.8 `crates/cv-fee-engine`
* Dynamic fee calculation engine supporting:
  - Flat fees (e.g., ₦10 per transfer)
  - Percentage with floor and cap (e.g., 0.5% min ₦50, max ₦1,000)
  - Tiered transaction volume discounts
  - Automatic split-routing of VAT (7.5%) to statutory tax accounts

---

## 8. Domain Modules (V1 Specification)

### 8.1 Authentication, IAM & Admin Module

* **Admin Roles:** `SUPER_ADMIN`, `OPERATIONS_OFFICER`, `COMPLIANCE_OFFICER`, `BRANCH_MANAGER`, `AUDITOR`.
* **Password Hashing:** Argon2id with minimum memory cost 64 MB, 3 iterations.
* **Session Tokens:** Stateless JWTs (Ed25519 signature, 15-minute access expiry, 24-hour refresh token stored in Valkey).
* **Maker-Checker Engine:** Operations with financial impact > ₦1,000,000 generate a `MakerCheckerRequest` ticket. The initiating user cannot approve their own ticket. Approval requires a separate user with checker permissions.

### 8.2 Customer Profiles & KYC Tiers

Complies strictly with the CBN Three-Tier KYC Framework:

| Tier | Cumulative Max Balance | Single Deposit Max | Requirements | Validation Method |
| :--- | :--- | :--- | :--- | :--- |
| **Tier 1** | ₦300,000.00 | ₦50,000.00 | Phone number, First/Last Name | SMS OTP verification |
| **Tier 2** | ₦500,000.00 | ₦200,000.00 | Tier 1 + BVN or NIN | NIBSS / NIMC API verification |
| **Tier 3** | **Unlimited** | **Unlimited** | Tier 2 + Valid Govt ID + Verified Address | Document OCR via Tesseract + Utility bill upload |

* **Application-Level PII Encryption:** BVN, NIN, and home addresses are encrypted using `AES-256-GCM` before database insertion. Only SHA-256 blind indices are stored for lookup queries.

### 8.3 Accounts & NUBAN Engine

* **Account Types:** `SAVINGS`, `CURRENT`, `DOMICILIARY`, `FIXED_DEPOSIT`, `LOAN`, `SUSPENSE`, `INTERNAL_GL`.
* **NUBAN Generation Formula:**
  $$\text{NUBAN} = \text{BankCode (3 digits)} + \text{Serial (6 digits)} + \text{CheckDigit (1 digit)}$$

```rust
// crates/cv-nuban-validator/src/lib.rs (PUBLIC CORE)
pub const NUBAN_WEIGHTS: [u32; 9] = [3, 7, 3, 3, 7, 3, 3, 7, 3];

pub fn calculate_check_digit(bank_code: &str, serial: &str) -> Result<u8, &'static str> {
    if bank_code.len() != 3 || serial.len() != 6 {
        return Err("Invalid bank code or serial length");
    }
    let mut sum = 0;
    for (ch, &weight) in bank_code.chars().chain(serial.chars()).zip(NUBAN_WEIGHTS.iter()) {
        let digit = ch.to_digit(10).ok_or("Non-numeric character found")?;
        sum += digit * weight;
    }
    let remainder = sum % 10;
    Ok(if remainder == 0 { 0 } else { (10 - remainder) as u8 })
}
```

### 8.4 Double-Entry Ledger Engine

Every balance modification is backed by a balanced journal entry:

```rust
// crates/cv-double-entry-engine/src/lib.rs (PUBLIC CORE)
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType { Debit, Credit }

#[derive(Debug, Clone)]
pub struct PostingLeg {
    pub account_id: uuid::Uuid,
    pub entry_type: EntryType,
    pub amount: Decimal,
    pub currency: String,
}

pub fn validate_batch(legs: &[PostingLeg]) -> Result<(), &'static str> {
    if legs.len() < 2 {
        return Err("Transaction must have at least 2 legs");
    }
    let mut balance_map: HashMap<&str, (Decimal, Decimal)> = HashMap::new();
    for leg in legs {
        if leg.amount <= Decimal::ZERO {
            return Err("Posting amounts must be strictly positive");
        }
        let entry = balance_map.entry(&leg.currency).or_insert((Decimal::ZERO, Decimal::ZERO));
        match leg.entry_type {
            EntryType::Debit => entry.0 += leg.amount,
            EntryType::Credit => entry.1 += leg.amount,
        }
    }
    for (currency, (dr, cr)) in balance_map {
        if dr != cr {
            return Err("Imbalanced journal: Debits do not equal Credits");
        }
    }
    Ok(())
}
```

### 8.5 Transactions & Journal Processing

* **Idempotency Guarantee:**
  1. Client sends `Idempotency-Key: {uuid}` header.
  2. Axum middleware runs `SET idempotency:{key} PENDING NX EX 120` in Valkey.
  3. If key exists: returns cached response or `409 Conflict`.
  4. On successful commit: stores final HTTP status and response payload with 24-hour TTL.
* **Transaction States:** `PENDING` $\to$ `COMMITTED` $\to$ `REVERSED` (Reversals create a new offsetting journal entry; no records are updated or deleted).

### 8.6 Balance Materialization & Concurrency Strategy

To ensure zero balance inconsistency without full table locking:
1. **Optimistic Locking via Version Counters:** Each row in the `accounts` table contains a `version BIGINT` column.
2. **Atomic Balance Updates:**
   ```sql
   UPDATE accounts 
   SET ledger_balance = ledger_balance - $amount, 
       available_balance = available_balance - $amount,
       version = version + 1,
       updated_at = NOW()
   WHERE id = $account_id AND version = $expected_version AND (available_balance - $amount >= -overdraft_limit);
   ```
3. If zero rows are returned by the update, the transaction aborts and retries with exponential backoff (up to 3 attempts), preventing deadlocks.
4. **Periodic Reconciliation Worker:** A background task runs every night at 00:00 UTC, summing all `journal_entries` per account and verifying equality with the materialized `accounts.ledger_balance`.

---

## 9. Complete PostgreSQL 16 Database Schema

```sql
-- CoreVault V1 Production DDL Schema (PostgreSQL 16)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- 1. Tenants Table
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(16) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    country_code CHAR(2) NOT NULL DEFAULT 'NG',
    base_currency CHAR(3) NOT NULL DEFAULT 'NGN',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Staff Users & IAM
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    role VARCHAR(32) NOT NULL CHECK (role IN ('SUPER_ADMIN', 'OPERATIONS_OFFICER', 'COMPLIANCE_OFFICER', 'BRANCH_MANAGER', 'AUDITOR')),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3. Customer Profiles
CREATE TABLE customers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    customer_number VARCHAR(32) UNIQUE NOT NULL,
    customer_type VARCHAR(16) NOT NULL CHECK (customer_type IN ('INDIVIDUAL', 'SME', 'CORPORATE')),
    kyc_tier SMALLINT NOT NULL DEFAULT 1 CHECK (kyc_tier IN (1, 2, 3)),
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    phone_number VARCHAR(20) NOT NULL,
    email VARCHAR(255),
    bvn_encrypted BYTEA,              -- Encrypted AES-256-GCM
    bvn_hash CHAR(64),                -- SHA-256 for deterministic duplicate checks
    nin_encrypted BYTEA,
    nin_hash CHAR(64),
    date_of_birth DATE,
    address_encrypted BYTEA,
    is_pep BOOLEAN NOT NULL DEFAULT false,
    status VARCHAR(16) NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('PENDING', 'ACTIVE', 'BLOCKED')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_customers_bvn_hash ON customers(bvn_hash);
CREATE INDEX idx_customers_phone ON customers(phone_number);
CREATE INDEX idx_customers_search ON customers USING gin ((first_name || ' ' || last_name) gin_trgm_ops);

-- 4. Chart of Accounts (COA)
CREATE TABLE chart_of_accounts (
    gl_code VARCHAR(16) PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    category VARCHAR(16) NOT NULL CHECK (category IN ('ASSET', 'LIABILITY', 'EQUITY', 'INCOME', 'EXPENSE')),
    is_active BOOLEAN NOT NULL DEFAULT true
);

-- 5. Bank Accounts
CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    customer_id UUID REFERENCES customers(id), -- NULL for internal GL accounts
    account_number VARCHAR(10) UNIQUE NOT NULL,
    account_name VARCHAR(255) NOT NULL,
    gl_code VARCHAR(16) NOT NULL REFERENCES chart_of_accounts(gl_code),
    account_type VARCHAR(20) NOT NULL CHECK (account_type IN ('SAVINGS', 'CURRENT', 'DOMICILIARY', 'LOAN', 'INTERNAL_GL', 'ESCROW')),
    currency_code CHAR(3) NOT NULL DEFAULT 'NGN',
    ledger_balance NUMERIC(20, 4) NOT NULL DEFAULT 0.0000,
    available_balance NUMERIC(20, 4) NOT NULL DEFAULT 0.0000,
    lien_amount NUMERIC(20, 4) NOT NULL DEFAULT 0.0000 CHECK (lien_amount >= 0),
    overdraft_limit NUMERIC(20, 4) NOT NULL DEFAULT 0.0000 CHECK (overdraft_limit >= 0),
    status VARCHAR(20) NOT NULL DEFAULT 'ACTIVE' CHECK (status IN ('PENDING_OPENING', 'ACTIVE', 'DORMANT', 'POST_NO_DEBIT', 'FROZEN', 'CLOSED')),
    version BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_accounts_customer ON accounts(customer_id);

-- 6. Transactions Header Table (Partitioned by created_at monthly)
CREATE TABLE transactions (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id),
    transaction_ref VARCHAR(64) NOT NULL,
    idempotency_key VARCHAR(128) NOT NULL,
    channel VARCHAR(16) NOT NULL CHECK (channel IN ('INTERNAL', 'API', 'NIP', 'POS', 'WEB', 'MOBILE')),
    status VARCHAR(16) NOT NULL DEFAULT 'COMMITTED' CHECK (status IN ('PENDING', 'COMMITTED', 'REVERSED', 'FAILED')),
    total_amount NUMERIC(20, 4) NOT NULL CHECK (total_amount > 0),
    currency_code CHAR(3) NOT NULL DEFAULT 'NGN',
    narration TEXT NOT NULL,
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at),
    UNIQUE (tenant_id, transaction_ref, created_at)
) PARTITION BY RANGE (created_at);

-- Example partition for 2026 Q4
CREATE TABLE transactions_2026_10 PARTITION OF transactions
    FOR VALUES FROM ('2026-10-01 00:00:00+00') TO ('2026-11-01 00:00:00+00');
CREATE TABLE transactions_2026_11 PARTITION OF transactions
    FOR VALUES FROM ('2026-11-01 00:00:00+00') TO ('2026-12-01 00:00:00+00');

-- 7. Immutable Journal Entries Table (Partitioned by created_at monthly)
CREATE TABLE journal_entries (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    transaction_id UUID NOT NULL,
    account_id UUID NOT NULL,
    gl_code VARCHAR(16) NOT NULL REFERENCES chart_of_accounts(gl_code),
    entry_type CHAR(2) NOT NULL CHECK (entry_type IN ('DR', 'CR')),
    amount NUMERIC(20, 4) NOT NULL CHECK (amount > 0),
    currency_code CHAR(3) NOT NULL DEFAULT 'NGN',
    running_balance NUMERIC(20, 4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

CREATE TABLE journal_entries_2026_10 PARTITION OF journal_entries
    FOR VALUES FROM ('2026-10-01 00:00:00+00') TO ('2026-11-01 00:00:00+00');
CREATE TABLE journal_entries_2026_11 PARTITION OF journal_entries
    FOR VALUES FROM ('2026-11-01 00:00:00+00') TO ('2026-12-01 00:00:00+00');

CREATE INDEX idx_journal_account_date ON journal_entries(account_id, created_at DESC);

-- 8. Audit Logs Table
CREATE TABLE audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    actor_id UUID NOT NULL,
    actor_type VARCHAR(16) NOT NULL CHECK (actor_type IN ('USER', 'SYSTEM', 'API_CLIENT')),
    action VARCHAR(64) NOT NULL,
    entity_name VARCHAR(64) NOT NULL,
    entity_id UUID NOT NULL,
    before_state JSONB,
    after_state JSONB,
    ip_address INET,
    trace_id VARCHAR(64),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_entity ON audit_logs(entity_name, entity_id);
```

---

## 10. API Surface Specification (REST & gRPC)

### 10.1 Core REST Endpoints (Axum HTTP/JSON)

| Method | Endpoint | Description | Auth Required |
| :--- | :--- | :--- | :--- |
| `POST` | `/api/v1/auth/login` | Staff authentication; returns JWT pair | None |
| `POST` | `/api/v1/customers` | Onboard customer (Tier 1/2/3) | Bearer Token |
| `GET` | `/api/v1/customers/{id}` | Retrieve customer profile | Bearer Token |
| `GET` | `/api/v1/customers/search?q={query}` | Trigram search by name, phone, or BVN hash | Bearer Token |
| `POST` | `/api/v1/accounts` | Create new account with generated NUBAN | Bearer Token |
| `GET` | `/api/v1/accounts/{nuban}` | Balance inquiry & account metadata | Bearer Token |
| `POST` | `/api/v1/transfers/intra` | Post internal transfer (Sender $\to$ Receiver) | Bearer + Idempotency |
| `GET` | `/api/v1/accounts/{nuban}/statement` | Paginated transaction history | Bearer Token |
| `POST` | `/api/v1/admin/maker-checker/requests` | Submit an action for 4-eyes approval | Maker Role |
| `POST` | `/api/v1/admin/maker-checker/{id}/approve` | Sign off on pending administrative action | Checker Role |
| `GET` | `/health/live` | Kubernetes / Load Balancer Liveness | Public |
| `GET` | `/health/ready` | Database & Valkey connectivity check | Public |
| `GET` | `/metrics` | Prometheus scraped metrics endpoint | Private / Internal |

### 10.2 Enterprise gRPC Service Contract (`proto/ledger.proto`)

```protobuf
syntax = "proto3";
package corevault.v1;

service LedgerService {
  rpc PostTransaction (TransactionRequest) returns (TransactionResponse);
  rpc QueryBalance (BalanceRequest) returns (BalanceResponse);
}

message TransactionLeg {
  string account_id = 1;
  string entry_type = 2; // "DR" or "CR"
  string amount = 3;     // String representation of Decimal
  string gl_code = 4;
}

message TransactionRequest {
  string idempotency_key = 1;
  string transaction_ref = 2;
  string narration = 3;
  string currency_code = 4;
  repeated TransactionLeg legs = 5;
}

message TransactionResponse {
  string transaction_id = 1;
  string status = 2;
  string created_at = 3;
}

message BalanceRequest {
  string account_number = 1;
}

message BalanceResponse {
  string account_number = 1;
  string ledger_balance = 2;
  string available_balance = 3;
  string currency_code = 4;
  string status = 5;
}
```

---

## 11. Security & Secrets Management (V1)

1. **No External Vault Dependency:** Eliminates OpenBao/HashiCorp Vault complexity for V1.
2. **Envelope Encryption for Sensitive Data:**
   - The master encryption key (`COREVAULT_MASTER_KEY_HEX`, 256-bit) is injected as an environment variable into the binary.
   - PII fields (BVN, NIN, Address) are encrypted using authenticated `AES-256-GCM` with a fresh 96-bit random nonce per record.
   - Nonce and ciphertext are concatenated and stored as binary `BYTEA` in PostgreSQL.
3. **Defense Against Memory Dumps:** Key memory buffers in Rust implement the `zeroize` crate to erase cryptographic keys from heap memory on drop.
4. **Caddy Edge Protections:**
   - Rate limiting: Max 100 requests/minute per client IP for public endpoints; max 10 requests/minute for login.
   - Automatic TLS 1.3 with modern curve preferences (X25519).

---

## 12. Observability, Health Checks & Logging

### 12.1 Structured Logging via `tracing`
All logs are emitted to standard output in structured JSON format with contextual fields:
```json
{
  "timestamp": "2026-10-22T21:00:00.123Z",
  "level": "INFO",
  "target": "cv_account_manager::service",
  "trace_id": "cba-02c43ce8-2a96-43da-ab13-8afcae5023ce",
  "tenant_id": "7b2e3532-...",
  "account_number": "0123456789",
  "action": "balance_update",
  "amount": "50000.00",
  "duration_ms": 3.4
}
```

### 12.2 Prometheus Metrics Exposed at `/metrics`
* `corevault_transactions_total{status="committed|failed", channel="api|nip"}`: Counter
* `corevault_transaction_latency_seconds`: Histogram (p50, p95, p99)
* `corevault_db_pool_connections{state="active|idle"}`: Gauge
* `corevault_ledger_reconciliation_drift_cents`: Gauge (Must remain strictly 0)

---

## 13. Build, Cross-Compilation & Container Packaging

### 13.1 Cross-Compilation for ARM64 (OCI Ampere A1)

Since development often occurs on x86-64 machines or Apple Silicon, builds target the ARM Ampere architecture (`aarch64-unknown-linux-gnu`):

```bash
# Add ARM64 target
rustup target add aarch64-unknown-linux-gnu

# Build optimized release binary
cargo build --release --target aarch64-unknown-linux-gnu
```

### 13.2 Production Dockerfile (Multi-Stage with `cargo-chef`)

```dockerfile
# Stage 1: Cargo Chef Planner
FROM rust:1.81-bookworm AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: Recipe Extraction
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Builder
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin corevault-monolith

# Stage 4: Minimal Distroless Runtime
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libpq5 libssl3 tzdata \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 10001 corevault && \
    useradd -u 10001 -g corevault -s /bin/false -M corevault

WORKDIR /app
COPY --from=builder /app/target/release/corevault-monolith /app/corevault

USER 10001:10001
ENV TZ=Africa/Lagos
ENV RUST_LOG=info,corevault=debug

EXPOSE 8080 50051
ENTRYPOINT ["/app/corevault"]
```

### 13.3 Production `docker-compose.yml` for OCI Free Tier

```yaml
version: '3.8'

services:
  caddy:
    image: caddy:2-alpine
    restart: always
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./Caddyfile:/etc/caddy/Caddyfile:ro
      - caddy_data:/data
      - caddy_config:/config
    depends_on:
      - corevault

  corevault:
    build:
      context: .
      dockerfile: Dockerfile
    restart: always
    environment:
      - DATABASE_URL=postgres://cv_user:${DB_PASSWORD}@postgres:5432/corevault_db
      - VALKEY_URL=redis://valkey:6379
      - COREVAULT_MASTER_KEY_HEX=${MASTER_KEY_HEX}
      - JWT_SECRET=${JWT_SECRET}
    ports:
      - "127.0.0.1:8080:8080"
      - "127.0.0.1:50051:50051"
    depends_on:
      postgres:
        condition: service_healthy
      valkey:
        condition: service_healthy

  postgres:
    image: postgres:16-alpine
    restart: always
    environment:
      POSTGRES_DB: corevault_db
      POSTGRES_USER: cv_user
      POSTGRES_PASSWORD: ${DB_PASSWORD}
    command: >
      postgres 
      -c shared_buffers=4GB 
      -c effective_cache_size=12GB 
      -c work_mem=64MB 
      -c maintenance_work_mem=512MB 
      -c wal_level=replica 
      -c checkpoint_completion_target=0.9
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U cv_user -d corevault_db"]
      interval: 5s
      timeout: 5s
      retries: 5

  valkey:
    image: valkey/valkey:7.2-alpine
    restart: always
    command: valkey-server --maxmemory 512mb --maxmemory-policy allkeys-lru
    healthcheck:
      test: ["CMD", "valkey-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

volumes:
  pgdata:
  caddy_data:
  caddy_config:
```

---

## 14. Non-Functional Requirements & MVP Acceptance Criteria

| Metric | Target | Verification Method |
| :--- | :--- | :--- |
| **Double-Entry Accuracy** | $100.00\%$ invariant compliance | Zero reconciliation mismatch on nightly ledger audits |
| **Intra-Bank Transfer Latency** | $\text{p99} < 40\text{ ms}$ | `wrk` benchmarking against `/api/v1/transfers/intra` |
| **Throughput Target (V1)** | $\ge 2,500\text{ TPS}$ | Load test with 50 concurrent connections on OCI Ampere A1 |
| **Cold Startup Time** | $< 1.5\text{ seconds}$ | Time from container start to `/health/ready` 200 OK |
| **Binary Memory RSS** | $< 80\text{ MB}$ under normal operation | Measured via Linux `cgroups` / `top` |
| **Monthly Hosting Cost** | **$0.00** | Exclusively within Oracle Cloud Always Free Tier limits |

---

## 15. Glossary

* **NUBAN:** Nigerian Uniform Bank Account Number (10-digit account standard mandated by the CBN).
* **BVN:** Bank Verification Number (11-digit biometric customer identifier managed by NIBSS).
* **NIP:** NIBSS Instant Payments (real-time settlement protocol across all licensed Nigerian banks).
* **Open Core:** A business model where core infrastructure is open source, and enterprise network connectors are commercial.
* **Modular Monolith:** A single-process application built with strict internal boundaries (crates/packages) that can be split into microservices without rewriting business logic.
* **Valkey:** High-performance, BSD 3-Clause in-memory store maintained under the Linux Foundation as an open-source continuation of Redis.
* **Posting Leg:** An individual debit or credit entry belonging to a balanced accounting transaction.
