# CoreVault REST API Reference

This document provides complete, production-grade documentation for the **CoreVault Core Banking Application (CBA)** REST API.

All endpoints adhere to standard HTTP semantics, return `application/json` responses, and follow strict ISO-8601 formatting for timestamps.

---

## 📑 Table of Contents
1. [Overview & Standards](#overview--standards)
2. [Section 1: Health & Infrastructure Probes](#section-1-health--infrastructure-probes)
3. [Section 2: Staff Authentication & IAM](#section-2-staff-authentication--iam)
4. [Section 3: Account Management & NUBAN](#section-3-account-management--nuban)
5. [Section 4: Account Controls & Liens](#section-4-account-controls--liens)
6. [Section 5: Double-Entry Transfers](#section-5-double-entry-transfers)
7. [Section 6: Multi-Country Platformization](#section-6-multi-country-platformization)
8. [Section 7: Audit Trail & Compliance](#section-7-audit-trail--compliance)
9. [HTTP Error Codes & Responses](#http-error-codes--responses)

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

## Section 2: Staff Authentication & IAM

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

### 2.2 Get Authenticated User Identity
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
