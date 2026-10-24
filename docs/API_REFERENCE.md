# CoreVault REST API Reference

This document provides complete, production-grade documentation for the **CoreVault Core Banking Application (CBA)** REST API.

All endpoints adhere to standard HTTP semantics, return `application/json` responses, and follow strict ISO-8601 formatting for timestamps.

---

## 📑 Table of Contents
1. [Overview & Standards](#overview--standards)
2. [Section 1: Health & Infrastructure Probes](#section-1-health--infrastructure-probes)
3. [Section 2: Authentication & Client IAM](#section-2-authentication--client-iam)
4. [Section 3: Account Management & NUBAN](#section-3-account-management--nuban)
5. [Section 4: Account Controls & Liens](#section-4-account-controls--liens)
6. [Section 5: Double-Entry Transfers](#section-5-double-entry-transfers)
7. [Section 6: Multi-Country Platformization](#section-6-multi-country-platformization)
8. [Section 7: Audit Trail & Compliance](#section-7-audit-trail--compliance)
9. [Section 8: Admin Tenant & Client Governance](#section-8-admin-tenant--client-governance)
10. [Section 9: Tenant Custom Post-No-Debit (PND) Rules](#section-9-tenant-custom-post-no-debit-pnd-rules)
11. [HTTP Error Codes & Responses](#http-error-codes--responses)

---

## 🌐 Overview & Standards

### Base URL
```
http://127.0.0.1:8080
```

### Common Headers
| Header | Description | Required | Example |
| :--- | :--- | :--- | :--- |
| `Content-Type` | Payload format | Yes for POST/PUT | `application/json` |
| `Authorization` | Bearer JWT token | For authenticated routes | `Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6...` |
| `Idempotency-Key`| Unique UUID to guarantee at-most-once execution | For state-mutating requests | `8e3a2b40-97c2-40f0-8c2f-e771c99f1234` |

---

## Section 1: Health & Infrastructure Probes

### 1.1 Liveness Probe
Checks if the CoreVault HTTP listener and process are alive.

- **Method:** `GET`
- **Path:** `/health/live`
- **Authentication:** None (Public)

#### Response
- **Status:** `200 OK`
```json
{
  "service": "corevault-monolith",
  "status": "UP",
  "timestamp": "2026-10-23T08:00:00Z"
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/health/live
```

---

### 1.2 Readiness Probe
Validates connectivity to backend dependencies (PostgreSQL database and Valkey cache).

- **Method:** `GET`
- **Path:** `/health/ready`
- **Authentication:** None (Public)

#### Response
- **Status:** `200 OK`
```json
{
  "checks": {
    "database": "UP",
    "valkey": "UP"
  },
  "service": "corevault-monolith",
  "status": "READY",
  "timestamp": "2026-10-23T08:00:00Z"
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/health/ready
```

---

## Section 2: Authentication & Client IAM

Staff authentication is governed by the `cv-auth` crate using Argon2id password hashing and signed JSON Web Tokens (JWT).

### 2.1 Staff Login
Authenticates an internal banking operator and issues a signed JWT bearer token.

- **Method:** `POST`
- **Path:** `/api/v1/auth/login`
- **Authentication:** None (Public)

#### Request Headers
```http
Content-Type: application/json
```

#### Request Body
```json
{
  "email": "admin@corevault.bank",
  "password": "AdminSecurePass#2026"
}
```

#### Response (Success)
- **Status:** `200 OK`
```json
{
  "expires_in_seconds": 900,
  "status": "SUCCESS",
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "user": {
    "email": "admin@corevault.bank",
    "id": "c1f7a0b2-1e9a-4c91-b66a-2771d9d1bc50",
    "role": "OPERATIONS_OFFICER",
    "tenant_id": "a0000000-0000-0000-0000-000000000001"
  }
}
```

#### Response (Unauthorized)
- **Status:** `401 Unauthorized`
```json
{
  "message": "Invalid email or password",
  "status": "UNAUTHORIZED"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@corevault.bank", "password": "AdminSecurePass#2026"}'
```

---

---

### 2.2 Issue M2M Client Token (client_credentials)
Authenticates an external tenant machine-to-machine service using `client_id` and raw `client_secret`, returning a scoped Bearer JWT token valid for 1 hour.

- **Method:** `POST`
- **Path:** `/api/v1/auth/token`
- **Authentication:** None (Client Credentials)

#### Request Headers
```http
Content-Type: application/json
```

#### Request Body
```json
{
  "client_id": "client_wallet_app_seed001",
  "client_secret": "sec_live_wallet_app_seed_key_2026"
}
```

#### Response (Success - 200 OK)
- **Status:** `200 OK`
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_in_seconds": 3600,
  "scopes": [
    "transfers:write",
    "accounts:read",
    "accounts:write",
    "admin:read"
  ],
  "status": "SUCCESS",
  "tenant_id": "a0000000-0000-0000-0000-000000000001",
  "token_type": "Bearer"
}
```

#### Response (Failure - 401 Unauthorized: Invalid Secret)
- **Status:** `401 Unauthorized`
```json
{
  "message": "Invalid client_id or client_secret",
  "status": "UNAUTHORIZED"
}
```

#### Response (Failure - 403 Forbidden: Pending Approval)
- **Status:** `403 Forbidden`
```json
{
  "error": "Client/Tenant onboarding is pending admin approval and whitelisting",
  "status": "FORBIDDEN"
}
```

#### Response (Failure - 403 Forbidden: Blacklisted Client)
- **Status:** `403 Forbidden`
```json
{
  "error": "Client/Tenant is blacklisted: Sanctions match on international watch list",
  "status": "FORBIDDEN",
  "violation_count": 1
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/auth/token \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "client_wallet_app_seed001",
    "client_secret": "sec_live_wallet_app_seed_key_2026"
  }'
```

---

### 2.3 Get Authenticated User Identity
Validates the current session's JWT token and returns claims.

- **Method:** `GET`
- **Path:** `/api/v1/auth/me`
- **Authentication:** `Bearer <token>`

#### Request Headers
```http
Authorization: Bearer <token>
```

#### Response (Success)
- **Status:** `200 OK`
```json
{
  "claims": {
    "exp": 1792742400,
    "role": "OPERATIONS_OFFICER",
    "tenant_id": "a0000000-0000-0000-0000-000000000001",
    "user_id": "c1f7a0b2-1e9a-4c91-b66a-2771d9d1bc50"
  },
  "status": "SUCCESS"
}
```

#### Response (Missing or Invalid Token)
- **Status:** `401 Unauthorized`
```json
{
  "message": "Missing Authorization header",
  "status": "UNAUTHORIZED"
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/api/v1/auth/me \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

---

## Section 3: Account Management & NUBAN

Account lifecycle management is handled by `cv-account-manager` with check-digit validation provided by `cv-nuban-validator`.

### 3.1 Open Account
Creates a new customer or internal account, automatically assigning a CBN-compliant 10-digit NUBAN.

- **Method:** `POST`
- **Path:** `/api/v1/accounts`
- **Authentication:** Optional in MVP / Bearer Token recommended

#### Request Headers
```http
Content-Type: application/json
```

#### Request Body (Savings Account)
```json
{
  "customer_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "account_type": "Savings",
  "currency": "NGN",
  "bank_code": "044",
  "serial_number": "000123",
  "credit_limit": "0.00",
  "interest_rate": "0.045"
}
```

#### Request Body (Current Account with Overdraft)
```json
{
  "customer_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
  "account_type": "Current",
  "currency": "NGN",
  "bank_code": "044",
  "serial_number": "000124",
  "credit_limit": "500000.00",
  "interest_rate": "0.00"
}
```

#### Response (Success)
- **Status:** `201 Created`
```json
{
  "account": {
    "account_number": "0440001237",
    "account_type": "Savings",
    "credit_limit": "0.00",
    "currency": "NGN",
    "customer_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
    "hold_balance": "0.0000",
    "id": "8f8b1c1e-9271-4775-9c87-bb78c4371900",
    "interest_rate": "0.045",
    "lien_balance": "0.0000",
    "status": "ACTIVE"
  },
  "status": "SUCCESS"
}
```

#### Response (Validation Error - Invalid Bank Code)
- **Status:** `400 Bad Request`
```json
{
  "message": "Invalid bank code: must be exactly 3 digits",
  "status": "ERROR"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/accounts \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id": "f47ac10b-58cc-4372-a567-0e02b2c3d479",
    "account_type": "Savings",
    "currency": "NGN",
    "bank_code": "044",
    "serial_number": "000123",
    "credit_limit": "0.00",
    "interest_rate": "0.045"
  }'
```

---

### 3.2 Get Account by NUBAN
Fetches account status, ledger balance, and available balance.

- **Method:** `GET`
- **Path:** `/api/v1/accounts/:nuban`
- **Authentication:** Optional in MVP

#### Parameters
| Name | Type | Description |
| :--- | :--- | :--- |
| `nuban` | Path string | 10-digit NUBAN account number |

#### Response
- **Status:** `200 OK`
```json
{
  "account_number": "0440001237",
  "available_balance": "0.0000",
  "ledger_balance": "0.0000",
  "status": "ACTIVE"
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/api/v1/accounts/0440001237
```

---

## Section 4: Account Controls & Liens

Controls manage account operating statuses and funds restrictions as mandated by AML/CFT regulations and court orders.

### 4.1 Place Post-No-Debit (PND)
Restricts debits (withdrawals and outgoing transfers) while still permitting incoming credits.

- **Method:** `POST`
- **Path:** `/api/v1/accounts/:nuban/pnd`

#### Request Body
```json
{
  "reason": "Court Order FHC/ABJ/CS/2026/099 - Pending AML Investigation"
}
```

#### Response
- **Status:** `200 OK`
```json
{
  "account_number": "0440001237",
  "can_credit": true,
  "can_debit": false,
  "current_status": "POST_NO_DEBIT",
  "previous_status": "ACTIVE",
  "reason": "Court Order FHC/ABJ/CS/2026/099 - Pending AML Investigation",
  "status": "SUCCESS"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/accounts/0440001237/pnd \
  -H "Content-Type: application/json" \
  -d '{"reason": "Court Order FHC/ABJ/CS/2026/099 - Pending AML Investigation"}'
```

---

### 4.2 Freeze Account
Completely halts all transactions (both incoming credits and outgoing debits).

- **Method:** `POST`
- **Path:** `/api/v1/accounts/:nuban/freeze`

#### Request Body
```json
{
  "reason": "Direct request from EFCC / NFIU intelligence report"
}
```

#### Response
- **Status:** `200 OK`
```json
{
  "account_number": "0440001237",
  "can_credit": false,
  "can_debit": false,
  "current_status": "FROZEN",
  "previous_status": "ACTIVE",
  "reason": "Direct request from EFCC / NFIU intelligence report",
  "status": "SUCCESS"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/accounts/0440001237/freeze \
  -H "Content-Type: application/json" \
  -d '{"reason": "Direct request from EFCC / NFIU intelligence report"}'
```

---

### 4.3 Activate Account
Restores a restricted or frozen account to fully operational `ACTIVE` status.

- **Method:** `POST`
- **Path:** `/api/v1/accounts/:nuban/activate`

#### Request Body
```json
{
  "reason": "KYC documentation re-validated and compliance clearance granted"
}
```

#### Response
- **Status:** `200 OK`
```json
{
  "account_number": "0440001237",
  "can_credit": true,
  "can_debit": true,
  "current_status": "ACTIVE",
  "previous_status": "POST_NO_DEBIT",
  "reason": "KYC documentation re-validated and compliance clearance granted",
  "status": "SUCCESS"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/accounts/0440001237/activate \
  -H "Content-Type: application/json" \
  -d '{"reason": "KYC documentation re-validated and compliance clearance granted"}'
```

---

### 4.4 Place Financial Lien (Hold)
Places a specific monetary hold against an account's available balance.

- **Method:** `POST`
- **Path:** `/api/v1/accounts/:nuban/liens`

#### Request Body
```json
{
  "amount": "25000.00",
  "reason": "Collateral hold for loan facility LOAN-2026-99"
}
```

#### Response (Success)
- **Status:** `200 OK`
```json
{
  "action": "LIEN_PLACED",
  "account_number": "0440001237",
  "placed_lien_amount": "25000.00",
  "reason": "Collateral hold for loan facility LOAN-2026-99",
  "status": "SUCCESS"
}
```

#### Response (Invalid Amount)
- **Status:** `400 Bad Request`
```json
{
  "message": "Lien amount must be strictly positive",
  "status": "ERROR"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/accounts/0440001237/liens \
  -H "Content-Type: application/json" \
  -d '{"amount": "25000.00", "reason": "Collateral hold for loan facility LOAN-2026-99"}'
```

---

### 4.5 Release Financial Lien
Releases a previously placed monetary hold, restoring available balance.

- **Method:** `POST`
- **Path:** `/api/v1/accounts/:nuban/liens/release`

#### Request Body
```json
{
  "amount": "25000.00",
  "reason": "Loan fully repaid; release collateral hold"
}
```

#### Response
- **Status:** `200 OK`
```json
{
  "action": "LIEN_RELEASED",
  "account_number": "0440001237",
  "released_lien_amount": "25000.00",
  "reason": "Loan fully repaid; release collateral hold",
  "status": "SUCCESS"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/accounts/0440001237/liens/release \
  -H "Content-Type: application/json" \
  -d '{"amount": "25000.00", "reason": "Loan fully repaid; release collateral hold"}'
```

---

### 4.6 Lift Post-No-Debit (PND)
Removes an active Post-No-Debit restriction placed on an account (either `MANUAL_CLIENT_PND` or `SYSTEM_AUTOMATED_PND`) and restores operational `ACTIVE` debiting capabilities.

- **Method:** `POST`
- **Path:** `/api/v1/accounts/:nuban/pnd/lift`
- **Authentication:** `Bearer <token>`

#### Request Headers
```http
Content-Type: application/json
Authorization: Bearer <token>
```

#### Request Body (Optional)
```json
{
  "reason": "Customer Tier 2 KYC identity documentation validated and verified"
}
```

#### Response (Success - 200 OK)
- **Status:** `200 OK`
```json
{
  "account_number": "0440001237",
  "can_credit": true,
  "can_debit": true,
  "current_status": "ACTIVE",
  "lift_notes": "Customer Tier 2 KYC identity documentation validated and verified",
  "pnd_reason": null,
  "pnd_source": null,
  "previous_status": "POST_NO_DEBIT",
  "status": "SUCCESS"
}
```

#### Response (Failure - 400 Bad Request: Account Not Restrained)
- **Status:** `400 Bad Request`
```json
{
  "message": "Account 0440001237 is not under Post-No-Debit",
  "status": "ERROR"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/accounts/0440001237/pnd/lift \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -d '{"reason": "Customer Tier 2 KYC identity documentation validated and verified"}'
```

---

## Section 5: Double-Entry Transfers

Transfers are executed via `cv-double-entry-engine` and enforce strict conservation of value across all posting legs.

### 5.1 Post Balanced Intra-Bank Transfer
Transfers funds between two internal accounts.

- **Method:** `POST`
- **Path:** `/api/v1/transfers/intra`

#### Request Headers
```http
Content-Type: application/json
Idempotency-Key: <UUIDv4>
```

#### Request Body
```json
{
  "source_account_id": "8f8b1c1e-9271-4775-9c87-bb78c4371900",
  "destination_account_id": "b1a77413-5a02-45df-9d3e-908091ecf1aa",
  "amount": "15000.00",
  "currency": "NGN",
  "narration": "Invoice #4028 payment for cloud consultancy services"
}
```

#### Response (Success)
- **Status:** `201 Created`
```json
{
  "amount": "15000.00",
  "currency": "NGN",
  "narration": "Invoice #4028 payment for cloud consultancy services",
  "status": "COMMITTED",
  "transaction_id": "7b0d715a-4e6f-4024-8a4b-8736d936dae1"
}
```

#### Response (Validation Failure - Zero or Negative Amount)
- **Status:** `400 Bad Request`
```json
{
  "error": "Posting leg amount must be strictly positive",
  "status": "VALIDATION_FAILED"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/transfers/intra \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{
    "source_account_id": "8f8b1c1e-9271-4775-9c87-bb78c4371900",
    "destination_account_id": "b1a77413-5a02-45df-9d3e-908091ecf1aa",
    "amount": "15000.00",
    "currency": "NGN",
    "narration": "Invoice #4028 payment"
  }'
```

---

### 5.2 Tenant Compliance PND Rule Enforcement
Demonstrates transaction rejection and automated account restriction when an outgoing transfer breaches custom tenant compliance rules (e.g. single transfer limits).

- **Method:** `POST`
- **Path:** `/api/v1/transfers/intra`
- **Authentication:** `Bearer <token>`

#### Request Body (Breaching Configured Tier Cap)
```json
{
  "source_account_id": "8f8b1c1e-9271-4775-9c87-bb78c4371900",
  "destination_account_id": "b1a77413-5a02-45df-9d3e-908091ecf1aa",
  "amount": "85000.00",
  "currency": "NGN",
  "narration": "Large transfer exceeding tenant tier limit"
}
```

#### Response (Rule Breach - Automated System PND Enforced)
- **Status:** `409 Conflict`
```json
{
  "error": "Transaction breached tenant compliance rule 'KYC_TIER_1_SINGLE_TRANSFER_CAP'. Account has been automatically placed under SYSTEM_AUTOMATED_PND",
  "pnd_source": "SYSTEM_AUTOMATED_PND",
  "status": "PND_ENFORCED",
  "triggered_rule": "KYC_TIER_1_SINGLE_TRANSFER_CAP"
}
```

#### Response (Subsequent Debit Attempt Rejection)
- **Status:** `403 Forbidden`
```json
{
  "error": "Account is under restriction (POST_NO_DEBIT) and cannot be debited",
  "pnd_reason": "SYSTEM_PND [KYC_TIER_1_SINGLE_TRANSFER_CAP]: Transfer amount 85000.00 NGN breached tenant rule limit",
  "pnd_source": "SYSTEM_AUTOMATED_PND",
  "status": "ACCOUNT_RESTRICTED"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/transfers/intra \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -d '{
    "source_account_id": "8f8b1c1e-9271-4775-9c87-bb78c4371900",
    "destination_account_id": "b1a77413-5a02-45df-9d3e-908091ecf1aa",
    "amount": "85000.00",
    "currency": "NGN",
    "narration": "Attempted tier limit breach"
  }'
```

---

## Section 6: Multi-Country Platformization

Dynamic jurisdictional rules parsed by `cv-platform-config-parser`.

### 6.1 List Supported Jurisdictions
Returns the directory of supported country configurations.

- **Method:** `GET`
- **Path:** `/api/v1/config/jurisdictions`

#### Response
- **Status:** `200 OK`
```json
{
  "status": "SUCCESS",
  "supported_jurisdictions": [
    {
      "account_standard": "NUBAN",
      "country_code": "NG",
      "currency": "NGN",
      "name": "Nigeria",
      "regulatory_body": "Central Bank of Nigeria (CBN)"
    },
    {
      "account_standard": "SORT_CODE_AND_ACCOUNT",
      "country_code": "GB",
      "currency": "GBP",
      "name": "United Kingdom",
      "regulatory_body": "Financial Conduct Authority (FCA)"
    }
  ]
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/api/v1/config/jurisdictions
```

---

### 6.2 Get Jurisdiction Profile
Loads the parsed TOML configuration for a specified country code (e.g. `NG` or `GB`).

- **Method:** `GET`
- **Path:** `/api/v1/config/jurisdictions/:code`

#### Parameters
| Name | Type | Description |
| :--- | :--- | :--- |
| `code` | Path string | ISO 3166-1 alpha-2 country code (case-insensitive) |

#### Response (Success - `NG`)
- **Status:** `200 OK`
```json
{
  "configuration": {
    "account_number_validation": {
      "algorithm": "cbn_nuban_mod10",
      "bank_code_length": 3,
      "serial_length": 6,
      "total_length": 10
    },
    "currency": {
      "code": "NGN",
      "minor_units": 2,
      "name": "Nigerian Naira",
      "symbol": "₦"
    },
    "jurisdiction": {
      "country_code": "NG",
      "country_name": "Nigeria",
      "legal_framework": "BOFIA 2020",
      "regulatory_body": "Central Bank of Nigeria (CBN)",
      "timezone": "Africa/Lagos"
    },
    "limits": {
      "tier1_single_credit_max": "50000.00",
      "tier2_cumulative_balance_max": "500000.00",
      "tier3_single_transfer_max": "5000000.00"
    },
    "tax_and_levy": {
      "electronic_money_transfer_levy": "50.00",
      "emtl_threshold": "10000.00",
      "stamp_duty_charge": "50.00"
    }
  },
  "status": "SUCCESS"
}
```

#### Response (Not Found)
- **Status:** `404 Not Found`
```json
{
  "message": "Jurisdiction configuration 'FR' not found",
  "status": "NOT_FOUND"
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/api/v1/config/jurisdictions/NG
```

---

## Section 7: Audit Trail & Compliance

Real-time visibility into system audit events recorded by `cv-audit`.

### 7.1 Retrieve Audit Event Logs
Fetches append-only chronological audit records.

- **Method:** `GET`
- **Path:** `/api/v1/audit/logs`

#### Response
- **Status:** `200 OK`
```json
{
  "audit_logs": [
    {
      "action": "ACCOUNT_CONTROL_UPDATED",
      "actor_id": "c1f7a0b2-1e9a-4c91-b66a-2771d9d1bc50",
      "event_id": "1e8d9753-2940-4284-9dbb-82559ec11122",
      "ip_address": "127.0.0.1",
      "payload_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      "resource_id": "0440001237",
      "resource_type": "ACCOUNT",
      "tenant_id": "a0000000-0000-0000-0000-000000000001",
      "timestamp": "2026-10-23T08:00:00Z"
    }
  ],
  "status": "SUCCESS",
  "total_count": 1
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/api/v1/audit/logs
```

---

---

## Section 8: Admin Tenant & Client Governance

Endpoints for Super Administrators to onboard external client organizations, manage client credentials, whitelist/activate partners, and enforce platform-level compliance blacklisting.

### 8.1 Onboard New Client / Tenant
Registers an external client organization. Credentials are generated with Argon2id hashing and initialized to `PENDING` status until reviewed. The raw `client_secret` is displayed only once.

- **Method:** `POST`
- **Path:** `/api/v1/admin/tenants`
- **Authentication:** Super Admin

#### Request Headers
```http
Content-Type: application/json
```

#### Request Body
```json
{
  "tenant_code": "FINTECH_NEO",
  "tenant_name": "Fintech Neo Global Ltd",
  "base_currency": "NGN",
  "allowed_ips": ["127.0.0.1", "::1"],
  "scopes": ["transfers:write", "accounts:read", "accounts:write"]
}
```

#### Response (Success - 201 Created)
- **Status:** `201 Created`
```json
{
  "message": "Tenant onboarded with PENDING status. Admin whitelisting required before API activation.",
  "status": "SUCCESS",
  "tenant": {
    "base_currency": "NGN",
    "client_id": "client_fintech_neo_a1b2c3d4e5f6",
    "client_secret": "sec_live_9f8e7d6c5b4a3a2b1c0d",
    "id": "e9b21f42-45e3-4c91-9e77-123456789abc",
    "scopes": ["transfers:write", "accounts:read", "accounts:write"],
    "status": "PENDING",
    "tenant_code": "FINTECH_NEO",
    "tenant_name": "Fintech Neo Global Ltd"
  }
}
```

#### Response (Failure - 400 Bad Request)
- **Status:** `400 Bad Request`
```json
{
  "message": "tenant_code and tenant_name are required",
  "status": "ERROR"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/admin/tenants \
  -H "Content-Type: application/json" \
  -d '{
    "tenant_code": "FINTECH_NEO",
    "tenant_name": "Fintech Neo Global Ltd",
    "base_currency": "NGN",
    "scopes": ["transfers:write", "accounts:read", "accounts:write"]
  }'
```

---

### 8.2 List All Clients / Tenants
Returns a sanitized list of all registered clients and tenant organizations, omitting raw secrets and credential hashes.

- **Method:** `GET`
- **Path:** `/api/v1/admin/tenants`
- **Authentication:** Super Admin

#### Response (Success - 200 OK)
- **Status:** `200 OK`
```json
{
  "status": "SUCCESS",
  "tenants": [
    {
      "allowed_ips": ["127.0.0.1", "::1"],
      "base_currency": "NGN",
      "blacklist_reason": null,
      "client_id": "client_wallet_app_seed001",
      "created_at": "2026-10-24T00:00:00Z",
      "id": "a0000000-0000-0000-0000-000000000001",
      "scopes": ["transfers:write", "accounts:read", "accounts:write", "admin:read"],
      "status": "ACTIVE",
      "tenant_code": "WALLET_APP",
      "tenant_name": "CoreVault Native Wallet App",
      "updated_at": "2026-10-24T00:00:00Z",
      "violation_count": 0
    }
  ]
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/api/v1/admin/tenants
```

---

### 8.3 Get Client / Tenant by ID
Retrieves tenant metadata by UUID.

- **Method:** `GET`
- **Path:** `/api/v1/admin/tenants/:id`
- **Authentication:** Super Admin

#### Response (Success - 200 OK)
- **Status:** `200 OK`
```json
{
  "status": "SUCCESS",
  "tenant": {
    "allowed_ips": ["127.0.0.1", "::1"],
    "base_currency": "NGN",
    "blacklist_reason": null,
    "client_id": "client_wallet_app_seed001",
    "created_at": "2026-10-24T00:00:00Z",
    "id": "a0000000-0000-0000-0000-000000000001",
    "scopes": ["transfers:write", "accounts:read", "accounts:write", "admin:read"],
    "status": "ACTIVE",
    "tenant_code": "WALLET_APP",
    "tenant_name": "CoreVault Native Wallet App",
    "updated_at": "2026-10-24T00:00:00Z",
    "violation_count": 0
  }
}
```

#### Response (Failure - 404 Not Found)
- **Status:** `404 Not Found`
```json
{
  "message": "Tenant not found",
  "status": "NOT_FOUND"
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/api/v1/admin/tenants/a0000000-0000-0000-0000-000000000001
```

---

### 8.4 Super Admin Whitelist / Activate Client
Approves a pending or previously suspended client organization, promoting their status to `ACTIVE` and enabling M2M API token generation.

- **Method:** `POST`
- **Path:** `/api/v1/admin/tenants/:id/whitelist`
- **Authentication:** Super Admin

#### Response (Success - 200 OK)
- **Status:** `200 OK`
```json
{
  "message": "Tenant successfully whitelisted and activated",
  "status": "SUCCESS",
  "tenant": {
    "allowed_ips": ["127.0.0.1", "::1"],
    "base_currency": "NGN",
    "blacklist_reason": null,
    "client_id": "client_fintech_neo_a1b2c3d4e5f6",
    "id": "e9b21f42-45e3-4c91-9e77-123456789abc",
    "scopes": ["transfers:write", "accounts:read", "accounts:write"],
    "status": "ACTIVE",
    "tenant_code": "FINTECH_NEO",
    "tenant_name": "Fintech Neo Global Ltd",
    "violation_count": 0
  }
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/admin/tenants/e9b21f42-45e3-4c91-9e77-123456789abc/whitelist
```

---

### 8.5 Super Admin Blacklist Client
Immediately blacklists an external client organization, revoking all credentials and returning HTTP 403 Forbidden on all API endpoints.

- **Method:** `POST`
- **Path:** `/api/v1/admin/tenants/:id/blacklist`
- **Authentication:** Super Admin

#### Request Body
```json
{
  "reason": "Suspicious transaction velocity / AML evasion investigation"
}
```

#### Response (Success - 200 OK)
- **Status:** `200 OK`
```json
{
  "message": "Tenant blacklisted",
  "status": "SUCCESS",
  "tenant": {
    "blacklist_reason": "Suspicious transaction velocity / AML evasion investigation",
    "id": "e9b21f42-45e3-4c91-9e77-123456789abc",
    "status": "BLACKLISTED",
    "tenant_code": "FINTECH_NEO",
    "violation_count": 0
  }
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/admin/tenants/e9b21f42-45e3-4c91-9e77-123456789abc/blacklist \
  -H "Content-Type: application/json" \
  -d '{"reason": "Suspicious transaction velocity / AML evasion investigation"}'
```

---

### 8.6 Simulate Platform Policy Violation (Automated Blacklisting)
Simulates detection of platform rules violations (e.g. sanctions hit, transaction anomaly threshold breached, unauthorized network origin, AML pattern). Automatically increments `violation_count` and revokes API credentials.

- **Method:** `POST`
- **Path:** `/api/v1/admin/tenants/:id/simulate-violation`
- **Authentication:** System / Super Admin

#### Request Body
```json
{
  "violation_type": "SANCTIONS_REGISTRY_MATCH",
  "details": "Beneficiary match on international OFAC/UN consolidated sanctions watch list"
}
```

#### Response (Success - 200 OK)
- **Status:** `200 OK`
```json
{
  "message": "Platform automated blacklist triggered due to platform rule violation. Client credentials revoked.",
  "status": "BLACKLISTED",
  "tenant": {
    "blacklist_reason": "Platform automated blacklist: SANCTIONS_REGISTRY_MATCH: Beneficiary match on international OFAC/UN consolidated sanctions watch list",
    "id": "a0000000-0000-0000-0000-000000000001",
    "status": "BLACKLISTED",
    "tenant_code": "WALLET_APP",
    "violation_count": 1
  }
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/admin/tenants/a0000000-0000-0000-0000-000000000001/simulate-violation \
  -H "Content-Type: application/json" \
  -d '{
    "violation_type": "SANCTIONS_REGISTRY_MATCH",
    "details": "Beneficiary match on international OFAC/UN consolidated sanctions watch list"
  }'
```

---

## Section 9: Tenant Custom Post-No-Debit (PND) Rules

Allows individual tenants to configure automated compliance, AML, and risk thresholds (e.g. KYC single caps, daily velocity) that autonomously dish out system Post-No-Debits on breaching customer accounts.

### 9.1 Register Tenant Custom Compliance PND Rule
Registers a custom PND rule for a specific tenant instance.

- **Method:** `POST`
- **Path:** `/api/v1/tenants/pnd-rules`
- **Authentication:** Tenant Admin (`Bearer <token>`)

#### Request Body
```json
{
  "tenant_id": "a0000000-0000-0000-0000-000000000001",
  "rule_name": "KYC_TIER_1_SINGLE_TRANSFER_CAP",
  "rule_type": "KYC_TIER_LIMIT",
  "max_single_amount": "50000.00",
  "max_daily_cumulative": null,
  "is_enabled": true
}
```

#### Response (Success - 201 Created)
- **Status:** `201 Created`
```json
{
  "message": "Tenant custom compliance PND rule registered",
  "rule": {
    "id": "4b92c431-7e88-4f21-992a-112233445566",
    "is_enabled": true,
    "max_daily_cumulative": null,
    "max_single_amount": "50000.00",
    "rule_name": "KYC_TIER_1_SINGLE_TRANSFER_CAP",
    "rule_type": "KYC_TIER_LIMIT",
    "tenant_id": "a0000000-0000-0000-0000-000000000001"
  },
  "status": "SUCCESS"
}
```

#### Response (Failure - 400 Bad Request)
- **Status:** `400 Bad Request`
```json
{
  "message": "max_single_amount must be strictly positive",
  "status": "ERROR"
}
```

#### cURL Example
```bash
curl -X POST http://127.0.0.1:8080/api/v1/tenants/pnd-rules \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -d '{
    "tenant_id": "a0000000-0000-0000-0000-000000000001",
    "rule_name": "KYC_TIER_1_SINGLE_TRANSFER_CAP",
    "rule_type": "KYC_TIER_LIMIT",
    "max_single_amount": "50000.00",
    "is_enabled": true
  }'
```

---

### 9.2 List Active PND Rules for Tenant
Fetches all custom PND compliance rules configured for the given tenant.

- **Method:** `GET`
- **Path:** `/api/v1/tenants/:tenant_id/pnd-rules`
- **Authentication:** Tenant Admin (`Bearer <token>`)

#### Response (Success - 200 OK)
- **Status:** `200 OK`
```json
{
  "rules": [
    {
      "id": "4b92c431-7e88-4f21-992a-112233445566",
      "is_enabled": true,
      "max_daily_cumulative": null,
      "max_single_amount": "50000.00",
      "rule_name": "KYC_TIER_1_SINGLE_TRANSFER_CAP",
      "rule_type": "KYC_TIER_LIMIT",
      "tenant_id": "a0000000-0000-0000-0000-000000000001"
    }
  ],
  "status": "SUCCESS",
  "tenant_id": "a0000000-0000-0000-0000-000000000001"
}
```

#### cURL Example
```bash
curl -X GET http://127.0.0.1:8080/api/v1/tenants/a0000000-0000-0000-0000-000000000001/pnd-rules \
  -H "Authorization: Bearer YOUR_TOKEN_HERE"
```

---

## 🛑 HTTP Error Codes & Responses

CoreVault uses standardized HTTP status codes and structured JSON error bodies:

| Status Code | Meaning | Common Cause |
| :--- | :--- | :--- |
| `400 Bad Request` | Validation failure | Invalid NUBAN check digit, negative transfer amount, missing field |
| `401 Unauthorized` | Authentication error | Missing, invalid, or expired Bearer JWT token |
| `403 Forbidden` | Access denied | Insufficient role or tenant boundary mismatch |
| `404 Not Found` | Resource absent | Unknown account number or jurisdiction configuration file |
| `409 Conflict` | Business invariant | Account is frozen/PND, or duplicate idempotency key detected |
| `422 Unprocessable`| Ledger invariant | Double-entry journal batch sum is unbalanced ($\sum \text{Debits} \ne \sum \text{Credits}$) |
| `500 Internal Error`| System fault | Database query failure, storage timeout |
