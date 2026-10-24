# CoreVault Postman Collection & Environment

This directory contains the exportable Postman suite for CoreVault V1.

## Files
- [`corevault_collection.json`](corevault_collection.json): Comprehensive API request collection with 35 endpoints across 9 functional domains, featuring embedded JSON payload formatting, dynamic test scripts, and expandable **Success Examples** and **Failure Examples** on every request.
- [`corevault_environment.json`](corevault_environment.json): Environment variables for local development (`http://127.0.0.1:8080`) including client credentials and token chaining.

---

## 🗂 Collection Structure (9 Functional Folders)

1. **`1. Health & Infrastructure Probes`** — Liveness (`/health/live`) and Readiness (`/health/ready`)
2. **`2. Authentication & Client IAM (cv-auth)`** — Staff login, token verification, and M2M client credentials token issuance (`/api/v1/auth/token`) with approval and blacklist rejections
3. **`3. Account Management & NUBAN (cv-account-manager & cv-nuban-validator)`** — Savings, Current, and NUBAN balance enquiries
4. **`4. Account Controls & Liens (cv-account-manager)`** — Freeze, Activate, Liens, Manual Post-No-Debit (`/pnd`), and PND Lifting (`/pnd/lift`)
5. **`5. Double-Entry Transfers (cv-double-entry-engine)`** — Balanced transfers, negative/zero rejections, and tenant compliance rule breach rejections
6. **`6. Platform & Jurisdiction Configuration (cv-platform-config-parser)`** — Multi-country legal rules (`NG.toml`, `GB.toml`)
7. **`7. Audit Trail (cv-audit)`** — Chronological tamper-evident audit logs
8. **`8. Admin Tenant & Client Governance`** — Onboarding (`/api/v1/admin/tenants`), Whitelisting, Blacklisting, and Platform automated violation simulation
9. **`9. Tenant Custom PND Rules`** — Registration and listing of custom tenant compliance limits (`/api/v1/tenants/pnd-rules`)

---

## 🚀 How to Use in Postman

1. Open Postman.
2. Click **Import** (top left).
3. Drag and drop both `corevault_collection.json` and `corevault_environment.json`.
4. In the top-right environment selector, select **CoreVault Local Dev**.
5. Every request has **Expandable Examples** in the right-hand panel (`Success Example` and `Failure Example`) showing exact JSON schemas and status codes.
6. Run requests or use the **Collection Runner** to run all tests sequentially.

---

## ⚡ Automated CLI Testing with Newman

You can execute the entire test collection headlessly from your terminal using Newman:

```bash
# Install Newman globally (if not already installed)
npm install -g newman

# Execute the CoreVault test suite against your running local server
newman run postman/corevault_collection.json -e postman/corevault_environment.json
```

### Pre-configured Dynamic Scripts Included:
- **JSON Payload Enforcement:** All request bodies use native `mode: "raw"` with JSON language specification and `Content-Type: application/json`.
- **Automatic Idempotency Key Injection:** Automatically generates a new `UUIDv4` header `Idempotency-Key` on each transfer request.
- **Dynamic Serial Generation:** Automatically picks a randomized 6-digit serial on account opening to generate unique CBN-compliant 10-digit NUBANs.
- **Environment Chaining:** Token responses automatically persist Bearer tokens into `authToken` and `m2mToken`; new tenant onboarding automatically chains `newTenantId`, `newClientId`, and `newClientSecret`.
