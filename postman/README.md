# CoreVault Postman Collection & Environment

This directory contains the exportable Postman suite for CoreVault V1.

## Files

- [`corevault_collection.json`](corevault_collection.json): Comprehensive API request collection with **45 endpoints across 10 functional domains**, featuring embedded JSON payload formatting, dynamic test scripts, `x-tenant-id` header injection, and expandable **Success Examples** and **Failure Examples** on every request.
- [`corevault_environment.json`](corevault_environment.json): Environment variables for local development (`http://127.0.0.1:8080`) including client credentials, Basic Auth tokens, and automatic token chaining.

---

## 🗂 Collection Structure (10 Functional Folders)

1. **`1. Health & Infrastructure Probes`** — Liveness (`/health/live`) and Readiness (`/health/ready`)
2. **`2. Authentication & Client IAM (cv-auth)`** — Staff login, **Client User Login** (HTTP Basic Auth + JSON email/password), token introspection, and M2M `client_credentials` grant (`/api/v1/auth/token`) with approval and blacklist rejections
3. **`3. Account Management & NUBAN (cv-account-manager & cv-nuban-validator)`** — Savings/Current account opening with `x-tenant-id` header and NUBAN Mod-10 generation, account lookup by NUBAN
4. **`4. Account Controls & Liens (cv-account-manager)`** — Freeze, Activate, Liens, Manual Post-No-Debit (`/pnd`), and PND Lifting (`/pnd/lift`) — all scoped via `x-tenant-id` header
5. **`5. Double-Entry Transfers (cv-double-entry-engine)`** — Balanced intra-bank transfers, negative/zero rejections, and tenant compliance rule breach rejections
6. **`6. Payment Rails & External Payouts (Paystack & Anchor)`** — Multi-rail routing (Anchor BaaS / Paystack Transfers), name enquiry identity verification, outward payout dispatch, transfer recipient registration, settlement balance check, transfer status verification, and commercial bank directory
7. **`7. Platform & Jurisdiction Configuration (cv-platform-config-parser)`** — Multi-country legal rules (`NG.toml`, `GB.toml`)
8. **`8. Audit Trail (cv-audit)`** — Chronological tamper-evident audit logs scoped via `x-tenant-id` header
9. **`9. Admin Tenant & Client Governance`** — Onboarding (`/api/v1/admin/tenants`), Whitelisting, Blacklisting, and Platform automated violation simulation
10. **`10. Tenant Custom PND Rules`** — Registration and listing of custom tenant compliance limits via `x-tenant-id` header (`/api/v1/tenants/pnd-rules`)

---

## 🔐 Authentication

CoreVault has three authentication flows:

| Flow | Endpoint | Credentials |
|------|----------|-------------|
| **Staff Login** | `POST /api/v1/auth/login` | JSON `{"email": "...", "password": "..."}` |
| **Client User Login (Portal)** | `POST /api/v1/auth/client-login` | HTTP Basic Auth `client_id:client_secret` + JSON email/password |
| **M2M Client Credentials** | `POST /api/v1/auth/token` | JSON `{"grant_type": "client_credentials", "client_id": "...", "client_secret": "..."}` |

For **Client User Login**, compute the `Authorization` header as:
```
Authorization: Basic <base64(client_id:client_secret)>
```

**Seed credentials (local dev):**
- `client_id`: `client_wallet_app_seed001`
- `client_secret`: `sec_live_wallet_app_seed_key_2026`
- Basic Auth header: `Basic Y2xpZW50X3dhbGxldF9hcHBfc2VlZDAwMTpzZWNfbGl2ZV93YWxsZXRfYXBwX3NlZWRfa2V5XzIwMjY=`

---

## 📋 Tenant Scoping Header

All client-facing APIs require the `x-tenant-id` header to enforce strict cross-tenant data isolation:

```
x-tenant-id: a0000000-0000-0000-0000-000000000001
```

Requests without this header or with a mismatched tenant ID are rejected with `403 Forbidden`.

---

## 💳 Payment Rails

CoreVault supports swappable payment rail adapters via `cv-payment-gateway`:

| Provider | Description | Switch Command |
|----------|-------------|----------------|
| **Anchor** | Default BaaS NIP adapter (sandbox) | `{"provider": "Anchor"}` |
| **Paystack** | Paystack Transfers API (sandbox) | `{"provider": "Paystack"}` |
| **NibssDirect** | Direct NIBSS enterprise (cba-enterprise only) | Requires enterprise crate |

Switch the active rail via:
```
POST /api/v1/admin/payment-rails/active
{"provider": "Paystack"}
```

---

## 🚀 How to Use in Postman

1. Open Postman.
2. Click **Import** (top left).
3. Drag and drop both `corevault_collection.json` and `corevault_environment.json`.
4. In the top-right environment selector, select **CoreVault Local Dev**.
5. Every request has **Expandable Examples** in the right-hand panel (`Success Example` and `Failure Example`) showing exact JSON schemas and status codes.
6. Run the **Staff Login** request first to auto-populate `authToken`, then use the **Collection Runner** for sequential execution.

---

## ⚡ Automated CLI Testing with Newman

```bash
# Install Newman globally (if not already installed)
npm install -g newman

# Execute the CoreVault test suite against your running local server
newman run postman/corevault_collection.json -e postman/corevault_environment.json
```

### Pre-configured Dynamic Scripts Included:

- **JSON Payload Enforcement:** All `POST`, `PUT`, `PATCH` request bodies use native `mode: "raw"` with JSON language specification and `Content-Type: application/json`.
- **Automatic Idempotency Key Injection:** Generates a new `UUIDv4` `Idempotency-Key` header on each transfer request.
- **Dynamic Serial Generation:** Picks a randomized 6-digit serial on account opening to generate unique CBN-compliant 10-digit NUBANs.
- **Environment Chaining:** Token responses automatically persist Bearer tokens into `authToken`, `m2mToken`, and `clientUserToken`; new tenant onboarding automatically chains `newTenantId`, `newClientId`, and `newClientSecret`.
- **Recipient Code Chaining:** Transfer recipient creation auto-saves `recipientCode`; transfer dispatch saves `transferReference` and `transferSessionId` for verification.
