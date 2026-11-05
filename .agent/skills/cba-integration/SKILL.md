---
name: cba-integration
description: Consumer integration guide for CoreVault CBA — covers M2M authentication, account lifecycle management, double-entry intra-bank transfers, PND/lien compliance, automated rule enforcement, and enterprise extension points (PaymentRailAdapter, ComplianceReporter). Use when onboarding a new integration partner, scaffolding an SDK wrapper, generating API docs, or reasoning about feature availability across Open Core vs. Enterprise tiers.
---

# CoreVault CBA — Consumer Integration Skill

This skill is the authoritative reference for any agent or developer integrating an external application with CoreVault's Core Banking API. Read it in full before generating client code, writing SDK wrappers, or advising on feature availability.

---

## Architecture Overview

```
Consumer App
    │
    ├─ M2M Client Credentials (client_id / client_secret)
    │       ↓
    ├─ POST /api/v1/auth/token  ──→  Bearer JWT
    │
    └─ All subsequent requests carry:
           Authorization: Bearer <token>
```

CoreVault runs as a single Axum HTTP server (`cv-transaction-api`). All business logic is implemented in discrete workspace crates:

| Crate | Responsibility |
|---|---|
| `cv-auth` | JWT issuance, M2M credential validation, staff login |
| `cv-account-manager` | Account lifecycle, NUBAN generation, PND/lien logic |
| `cv-double-entry-engine` | Journal batch validation, double-entry posting |
| `cv-nuban-validator` | CBN Mod-10 check-digit (NUBAN standard) |
| `cv-audit` | SHA-256 tamper-evident append-only audit trail |
| `cv-platform-config-parser` | TOML jurisdiction profiles (KYC tiers, account standards) |
| `cv-types` | Shared types: `Currency`, `EntryType`, `Money`, extension traits |

---

## Feature Tiers

### Open Core (Public Repo — Free)

All features below are available in the public repository with no additional licensing.

| Feature | Endpoints |
|---|---|
| M2M authentication (client credentials) | `POST /api/v1/auth/token` |
| Staff login | `POST /api/v1/auth/login` |
| Account opening with CBN NUBAN generation | `POST /api/v1/accounts` |
| Account enquiry (single + list) | `GET /api/v1/accounts`, `GET /api/v1/accounts/:nuban` |
| Account freeze / unfreeze | `POST /api/v1/accounts/:nuban/freeze`, `POST /api/v1/accounts/:nuban/activate` |
| PND placement (manual) | `POST /api/v1/accounts/:nuban/pnd` |
| PND lift | `POST /api/v1/accounts/:nuban/pnd/lift` |
| Lien placement | `POST /api/v1/accounts/:nuban/liens` |
| Lien release | `POST /api/v1/accounts/:nuban/liens/release` |
| Intra-bank (on-us) double-entry transfer | `POST /api/v1/transfers/intra` |
| Inter-bank name enquiry (Anchor default / pluggable) | `POST /api/v1/transfers/enquiry` |
| Outward inter-bank transfer (Anchor BaaS default) | `POST /api/v1/transfers/outward` |
| Pluggable payment rails inspection & switching | `GET /api/v1/payment-rails`, `POST /api/v1/admin/payment-rails/active` |
| Automated tenant PND rules engine | `POST /api/v1/tenants/pnd-rules`, `GET /api/v1/tenants/:id/pnd-rules` |
| Tenant onboarding & whitelist/blacklist | `POST /api/v1/admin/tenants`, `POST /api/v1/admin/tenants/:id/whitelist` |
| Immutable SHA-256 audit trail | `GET /api/v1/audit/logs` |
| Jurisdiction / platform config (NG, GB) | `GET /api/v1/config/jurisdictions`, `GET /api/v1/config/jurisdictions/:country` |
| Health probes (liveness + readiness) | `GET /health/live`, `GET /health/ready` |
| Multi-currency accounts (NGN, USD, EUR, GBP) | Included in account creation |
| KYC Tier enforcement (Tier 1 / 2 / 3 limits) | Enforced automatically via platform config |

### Enterprise Tier (Private Crates / Higher Subscription)

The following capabilities are provided in `cba-enterprise`. Public Core ships with the pluggable `PaymentGateway` and `AnchorRailAdapter` by default; direct NIBSS integration and regulatory compliance engines live in `cba-enterprise`.

> **Confirmation:** CoreVault Open Core allows registering any adapter implementing `PaymentRailAdapter`. Direct NIBSS Instant Payments switch (`cv-rail-nip`) is exclusive to `cba-enterprise` for licensed commercial banks requiring 30-digit session IDs and ISO-8583 banking protocols.

| Feature | Trait / Extension Point | Notes |
|---|---|---|
| Direct NIBSS NIP Switch | `NipRailAdapter` (`cv-rail-nip`) | Direct ISO-8583 bridge to NIBSS switching network (30-digit session ID) |
| Valkey undo-delay queue (15-30s recall) | `cv-undo-transfer` | Cancellable outbox window before NIP settlement |
| CBN/NFIU regulatory compliance reporting | `ComplianceReporter` (`cv-compliance-reporting`) | AML screening, CTR threshold checks, goAML XML export |
| P2P Escrow & Milestone Engine | `cv-escrow-p2p` | Buyer/seller escrow contracts with OTP release |
| AI Transaction Intelligence & Categorization | `cv-smart-tagging` | Merchant taxonomy, runway calculation, MCC categorization |

To activate enterprise features, contact CoreVault for a commercial license. You will receive a private crates registry endpoint and credentials to add to your `Cargo.toml`.

---

## Authentication

### M2M (Machine-to-Machine) — Recommended for integrations

```http
POST /api/v1/auth/token
Content-Type: application/json

{
  "client_id": "your-client-id",
  "client_secret": "your-client-secret"
}
```

**Response:**
```json
{
  "access_token": "eyJ...",
  "token_type": "Bearer",
  "expires_in": 3600
}
```

Use the returned `access_token` as a Bearer token on all subsequent requests:
```
Authorization: Bearer eyJ...
```

> **Important:** Your tenant must be in `ACTIVE` (whitelisted) status. A `403 Forbidden` response means the tenant has been blacklisted or is pending activation.

### Staff Login (Admin Portal)

```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "email": "admin@example.com",
  "password": "your-password"
}
```

---

## Account Management

### Open an Account

```http
POST /api/v1/accounts
Authorization: Bearer <token>
Content-Type: application/json

{
  "account_name": "Jane Doe",
  "currency": "NGN",
  "bank_code": "058",
  "kyc_tier": 1
}
```

**Response (201 Created):**
```json
{
  "status": "SUCCESS",
  "account": {
    "id": "uuid-v4",
    "account_number": "0123456789",
    "account_name": "Jane Doe",
    "currency": "NGN",
    "status": "ACTIVE",
    "ledger_balance": "0.00",
    "available_balance": "0.00",
    "kyc_tier": 1
  }
}
```

The `account_number` is a CBN-compliant 10-digit NUBAN, generated using the Mod-10 algorithm with your `bank_code`.

### KYC Tier Limits (Nigeria)

| Tier | Max Single Deposit | Max Cumulative Balance | Required Docs |
|---|---|---|---|
| 1 | ₦50,000 | ₦300,000 | Phone number only |
| 2 | ₦200,000 | ₦500,000 | BVN or NIN + basic info |
| 3 | ₦5,000,000 | Unlimited | BVN + NIN + Govt ID + Utility bill |

### List Accounts

```http
GET /api/v1/accounts
Authorization: Bearer <token>
```

### Get Account by NUBAN

```http
GET /api/v1/accounts/0123456789
Authorization: Bearer <token>
```

**Response fields:** `account_number`, `status`, `pnd_source`, `pnd_reason`, `ledger_balance`, `available_balance`, `effective_available_funds`

Note: `effective_available_funds` = `available_balance` minus active liens.

---

## Account Restrictions — PND & Freeze

### Place Post-No-Debit (PND)

```http
POST /api/v1/accounts/0123456789/pnd
Authorization: Bearer <token>
Content-Type: application/json

{
  "reason": "Regulatory hold pending KYC update",
  "source": "MANUAL_OPERATOR"
}
```

### Lift PND

```http
POST /api/v1/accounts/0123456789/pnd/lift
Authorization: Bearer <token>
```

### Freeze Account

```http
POST /api/v1/accounts/0123456789/freeze
Authorization: Bearer <token>
```

A `FROZEN` account blocks both debits and credits. A `POST_NO_DEBIT` account blocks debits only.

### Activate (Unfreeze) Account

```http
POST /api/v1/accounts/0123456789/activate
Authorization: Bearer <token>
```

---

## Liens

Liens reduce `effective_available_funds` without reducing `available_balance`. They are used to reserve funds for pending obligations.

### Place Lien

```http
POST /api/v1/accounts/0123456789/liens
Authorization: Bearer <token>
Content-Type: application/json

{
  "amount": "5000.00",
  "currency": "NGN",
  "reference": "LIEN-REF-001",
  "reason": "Pending cheque clearance"
}
```

### Release Lien

```http
POST /api/v1/accounts/0123456789/liens/release
Authorization: Bearer <token>
Content-Type: application/json

{
  "reference": "LIEN-REF-001"
}
```

---

## Transfers

### Intra-Bank Transfer (On-Us)

Moves funds between two accounts held within the same CoreVault instance. Uses a strictly-balanced double-entry journal (Debit source, Credit destination) enforced by `cv-double-entry-engine`.

```http
POST /api/v1/transfers/intra
Authorization: Bearer <token>
Content-Type: application/json

{
  "source_account_id": "uuid-of-source-account",
  "destination_account_id": "uuid-of-destination-account",
  "amount": "10000.00",
  "currency": "NGN",
  "narration": "Salary payment — November 2026"
}
```

> **Note:** Use account **UUIDs** (not NUBANs) for intra-bank transfers. Retrieve the UUID from the account creation response or the account enquiry response.

**Transfer is rejected if:**
- Source account `status` is `FROZEN` or `POST_NO_DEBIT`
- Transfer amount breaches an active tenant PND rule (system auto-applies PND)
- Double-entry batch fails validation (debits ≠ credits)

**Success Response (200 OK):**
```json
{
  "status": "SUCCESS",
  "journal_id": "uuid",
  "message": "Transfer posted successfully"
}
```

**Restriction Response (403 Forbidden):**
```json
{
  "status": "ACCOUNT_RESTRICTED",
  "error": "Account is under restriction (POST_NO_DEBIT) and cannot be debited",
  "pnd_source": "SYSTEM_AUTOMATED",
  "pnd_reason": "Transfer amount 75000.00 NGN breached tenant rule limit"
}
```

---

## Automated PND Rules Engine

Tenants can configure custom transaction rules that trigger automatic PND placement when a transfer amount breaches a threshold. This runs inline during `POST /api/v1/transfers/intra` with no additional consumer action required.

### Define a Rule

```http
POST /api/v1/tenants/pnd-rules
Authorization: Bearer <token>
Content-Type: application/json

{
  "tenant_id": "your-tenant-uuid",
  "rule_name": "DAILY_DEBIT_CAP",
  "threshold_amount": "50000.00",
  "currency": "NGN"
}
```

### List Tenant Rules

```http
GET /api/v1/tenants/{tenant_id}/pnd-rules
Authorization: Bearer <token>
```

Built-in rule types in Open Core:
- `KYC_TIER_LIMIT` — enforces tier single-transaction ceiling from jurisdiction config
- `DAILY_DEBIT_CAP` — custom daily debit limit per account

---

## Audit Trail

All state-changing operations are written to an immutable, SHA-256 chained audit log.

```http
GET /api/v1/audit/logs
Authorization: Bearer <token>
```

**Optional query parameters:** `account_id`, `from`, `to`, `action_type`, `page`, `limit`

The audit log is append-only. Records are never updated or deleted (enforced at the crate level — no `UPDATE`/`DELETE` SQL is permitted on journal entries or postings).

---

## Jurisdiction / Platform Config

```http
GET /api/v1/config/jurisdictions
Authorization: Bearer <token>
```

```http
GET /api/v1/config/jurisdictions/NG
Authorization: Bearer <token>
```

Returns TOML-parsed jurisdiction metadata: country, currency, account number standard, KYC tier definitions.

---

## Tenant Administration

These endpoints are scoped to internal admin staff (not M2M consumers):

| Action | Endpoint |
|---|---|
| Onboard new tenant | `POST /api/v1/admin/tenants` |
| List all tenants | `GET /api/v1/admin/tenants` |
| Get tenant by ID | `GET /api/v1/admin/tenants/:id` |
| Whitelist tenant (→ ACTIVE) | `POST /api/v1/admin/tenants/:id/whitelist` |
| Blacklist tenant (→ SUSPENDED) | `POST /api/v1/admin/tenants/:id/blacklist` |

---

## Health Probes

```http
GET /health/live    # Liveness — process is up
GET /health/ready   # Readiness — DB connections healthy
```

Both return `200 OK` with `{ "status": "ok" }` when healthy.

---

## Error Response Format

All errors follow a consistent envelope:

```json
{
  "status": "ERROR",
  "message": "Human-readable error description",
  "triggered_rule": "DAILY_DEBIT_CAP"   // only present for rule violations
}
```

Common HTTP status codes:

| Code | Meaning |
|---|---|
| 400 | Bad request / validation failure |
| 401 | Missing or invalid Bearer token |
| 403 | Account restricted or tenant blacklisted |
| 404 | Account / resource not found |
| 409 | Conflict (e.g., duplicate lien reference) |
| 422 | Unprocessable entity (e.g., NUBAN check-digit failure) |
| 500 | Internal server error |

---

## Banking Invariants — Mandatory for SDK Authors

When wrapping CoreVault in an SDK, you **must** respect the following invariants. Cross-reference the `banking-invariants` skill for full verification commands.

1. **Zero floating point** — All monetary amounts must be sent as **strings** representing exact decimals (e.g. `"10000.00"`), never as JSON `number` (float). The server uses `rust_decimal::Decimal` internally.
2. **Balanced double-entry** — The server enforces ΣDebits == ΣCredits. You cannot post an unbalanced journal; the API will reject it with 400.
3. **Immutable audit** — Never attempt to edit or delete audit log entries via any admin pathway. The append-only constraint is enforced at the database level.
4. **PND is authoritative** — If the server places a SYSTEM_AUTOMATED PND, your SDK must surface this to the caller. Do not retry automatically — the account needs manual review.

---

## Enterprise Extension Points

For teams with access to private crates, implement these Rust traits and inject them into `AppState` at startup:

### `PaymentRailAdapter` (inter-bank / outward transfers)

```rust
// Defined in cv-types/src/traits.rs
#[async_trait]
pub trait PaymentRailAdapter: Send + Sync {
    async fn name_enquiry(
        &self,
        bank_code: &str,
        account_number: &str,
    ) -> CoreResult<AccountEnquiry>;

    async fn dispatch_outward(&self, transfer: &OutwardTransfer) -> CoreResult<TransferReceipt>;
}
```

### `ComplianceReporter` (AML / regulatory screening)

```rust
// Defined in cv-types/src/traits.rs
#[async_trait]
pub trait ComplianceReporter: Send + Sync {
    async fn evaluate_transaction(
        &self,
        sender_id: &str,
        recipient_id: &str,
        amount: Decimal,
    ) -> CoreResult<ComplianceDisposition>;
}
```

`ComplianceDisposition` values: `Approved` | `FlaggedForReview` | `Rejected`

Enterprise private crates ship concrete implementations of these traits for NIBSS NIP (Nigeria), Anchor Rail, and CBN/NFIU compliance.

---

## Quick-Start Checklist

- [ ] Obtain `client_id` + `client_secret` from CoreVault admin
- [ ] Confirm tenant is whitelisted (`ACTIVE`) via admin portal
- [ ] `POST /api/v1/auth/token` → store `access_token` (refresh before expiry)
- [ ] `POST /api/v1/accounts` → store returned `id` (UUID) and `account_number` (NUBAN)
- [ ] Test intra-bank transfer with a small amount
- [ ] Configure tenant PND rules if custom thresholds are needed
- [ ] Wire health probes into your deployment readiness checks
- [ ] For outward/inter-bank: engage CoreVault for enterprise tier and `PaymentRailAdapter` implementation

---

## Related Skills

- [`banking-invariants`](../banking-invariants/SKILL.md) — Verification of mathematical correctness before committing financial code
- [`platform-rules`](../platform-rules/SKILL.md) — Jurisdiction TOML profile validation and KYC tier parsing
- [`security-audit`](../security-audit/SKILL.md) — Argon2id, JWT, NUBAN spoofing regression checks
- [`verify-cba`](../verify-cba/SKILL.md) — Full test + clippy + fmt pipeline for the workspace
