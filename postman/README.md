# CoreVault Postman Collection & Environment

This directory contains the exportable Postman suite for CoreVault V1.

## Files
- [`corevault_collection.json`](corevault_collection.json): API request collection covering Health probes, NUBAN Account Opening, Double-Entry Intra-bank Transfers, and Balance Inquiries.
- [`corevault_environment.json`](corevault_environment.json): Environment variables for local development (`http://127.0.0.1:8080`).

---

## 🚀 How to Use in Postman

1. Open Postman.
2. Click **Import** (top left).
3. Drag and drop both `corevault_collection.json` and `corevault_environment.json`.
4. In the top-right environment selector, select **CoreVault Local Dev**.
5. Run requests or use the **Collection Runner** to run all tests sequentially.

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
- **Automatic Idempotency Key Injection:** Automatically generates a new `UUIDv4` header `Idempotency-Key` on each transfer request.
- **Dynamic Serial Generation:** Automatically picks a randomized 6-digit serial on account opening to generate unique CBN-compliant 10-digit NUBANs.
- **Environment Chaining:** The account creation request automatically persists the newly generated NUBAN and Account UUID into `accountNumber` and `sourceAccountId` for subsequent queries and transfer legs.
