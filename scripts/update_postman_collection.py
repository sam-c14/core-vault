import json
import os

with open("postman/corevault_collection.json", "r") as f:
    col = json.load(f)

# Update Collection Description
col["info"]["description"] = (
    "Comprehensive Postman Collection for CoreVault V1 — 100% Free & Open-Source Core Banking Engine in Rust.\n\n"
    "Organized by 10 domain crates and architecture boundaries:\n"
    "1. Health & Infrastructure Probes\n"
    "2. Authentication & Client IAM (`cv-auth`)\n"
    "3. Account Management & NUBAN (`cv-account-manager` & `cv-nuban-validator`)\n"
    "4. Account Controls & Liens (`cv-account-manager`)\n"
    "5. Double-Entry Transfers (`cv-double-entry-engine`)\n"
    "6. Payment Rails & External Payouts (`cv-payment-gateway`, `cv-rail-anchor`, `cv-rail-paystack`)\n"
    "7. Platform & Jurisdiction Configuration (`cv-platform-config-parser`)\n"
    "8. Audit Trail (`cv-audit`)\n"
    "9. Admin Tenant & Client Governance (`cv-transaction-api`)\n"
    "10. Tenant Custom PND Rules (`cv-account-manager` & `cv-transaction-api`)"
)

# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------

def ensure_header(headers_list, key, value):
    for h in headers_list:
        if h.get("key", "").lower() == key.lower():
            h["value"] = value
            return
    headers_list.append({"key": key, "value": value, "type": "text"})

def ensure_json_body(req, body_dict):
    req["body"] = {
        "mode": "raw",
        "raw": json.dumps(body_dict, indent=2),
        "options": {
            "raw": {
                "language": "json"
            }
        }
    }
    ensure_header(req.setdefault("header", []), "Content-Type", "application/json")

# ---------------------------------------------------------------------------
# 1. Update Folder 2: Authentication & Client IAM
# ---------------------------------------------------------------------------
f2 = col["item"][1]
f2["description"] = (
    "Powered by the `cv-auth` crate. Implements Argon2id password verification, cryptographic Ed25519/HMAC JWT "
    "token issuance, caller session authorization, M2M OAuth2 client credentials grant, and client basic-auth portal logins. "
    "All requests enforce strict credential verification and produce tamper-resistant signed JWT tokens."
)

# Add Client User Login endpoint to Folder 2
client_login_item = {
    "name": "Client User Login (Basic Auth + JSON Credentials)",
    "event": [
        {
            "listen": "test",
            "script": {
                "exec": [
                    'pm.test("Status code is 200 OK", function () {',
                    '    pm.response.to.have.status(200);',
                    '});',
                    'pm.test("Client session token received and saved to environment", function () {',
                    '    var jsonData = pm.response.json();',
                    '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                    '    pm.expect(jsonData.token).to.not.be.empty;',
                    '    pm.expect(jsonData.token_type).to.eql("Bearer");',
                    '    pm.environment.set("clientUserToken", jsonData.token);',
                    '});'
                ],
                "type": "text/javascript"
            }
        }
    ],
    "request": {
        "method": "POST",
        "header": [
            {
                "key": "Authorization",
                "value": "Basic {{clientBasicAuth}}",
                "type": "text"
            },
            {
                "key": "Content-Type",
                "value": "application/json",
                "type": "text"
            }
        ],
        "body": {
            "mode": "raw",
            "raw": json.dumps({
                "email": "admin@walletapp.io",
                "password": "SecurePassword123!"
            }, indent=2),
            "options": {
                "raw": {
                    "language": "json"
                }
            }
        },
        "url": {
            "raw": "{{baseUrl}}/api/v1/auth/client-login",
            "host": ["{{baseUrl}}"],
            "path": ["api", "v1", "auth", "client-login"]
        },
        "description": "Authenticates a tenant client business administrator into the CoreVault client portal. Requires HTTP Basic Auth with client_id:client_secret plus JSON payload containing business user email and password."
    },
    "response": [
        {
            "name": "Success Example - 200 OK",
            "originalRequest": {
                "method": "POST",
                "header": [
                    {"key": "Authorization", "value": "Basic {{clientBasicAuth}}", "type": "text"},
                    {"key": "Content-Type", "value": "application/json", "type": "text"}
                ],
                "body": {
                    "mode": "raw",
                    "raw": json.dumps({"email": "admin@walletapp.io", "password": "SecurePassword123!"}, indent=2),
                    "options": {"raw": {"language": "json"}}
                },
                "url": {
                    "raw": "{{baseUrl}}/api/v1/auth/client-login",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "auth", "client-login"]
                }
            },
            "status": "OK",
            "code": 200,
            "_postman_previewlanguage": "json",
            "header": [{"key": "Content-Type", "value": "application/json"}],
            "cookie": [],
            "body": json.dumps({
                "status": "SUCCESS",
                "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
                "token_type": "Bearer",
                "expires_in": 3600,
                "tenant_id": "a0000000-0000-0000-0000-000000000001",
                "email": "admin@walletapp.io",
                "role": "CLIENT_ADMIN"
            }, indent=2)
        },
        {
            "name": "Failure Example - 401 Unauthorized",
            "originalRequest": {
                "method": "POST",
                "header": [
                    {"key": "Authorization", "value": "Basic aW52YWxpZDppbnZhbGlk", "type": "text"},
                    {"key": "Content-Type", "value": "application/json", "type": "text"}
                ],
                "body": {
                    "mode": "raw",
                    "raw": json.dumps({"email": "admin@walletapp.io", "password": "WrongPassword"}, indent=2),
                    "options": {"raw": {"language": "json"}}
                },
                "url": {
                    "raw": "{{baseUrl}}/api/v1/auth/client-login",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "auth", "client-login"]
                }
            },
            "status": "Unauthorized",
            "code": 401,
            "_postman_previewlanguage": "json",
            "header": [{"key": "Content-Type", "value": "application/json"}],
            "cookie": [],
            "body": json.dumps({
                "status": "UNAUTHORIZED",
                "error": "Invalid client credentials or user password"
            }, indent=2)
        }
    ]
}

# Insert Client Login after Staff Login if not already present
if not any("Client User Login" in item["name"] for item in f2["item"]):
    f2["item"].insert(1, client_login_item)

# ---------------------------------------------------------------------------
# 2. Update Folder 3: Account Management & NUBAN
# ---------------------------------------------------------------------------
f3 = col["item"][2]
f3["description"] = (
    "Powered by `cv-account-manager` and `cv-nuban-validator`. Implements the Central Bank of Nigeria (CBN) Mod-10 "
    "algorithm for generating valid 10-digit NUBAN account numbers, account lifecycle state machines, and balance inquiries. "
    "All account operations strictly enforce tenant scoping via the mandatory `x-tenant-id` header to guarantee multi-tenant data isolation."
)

for item in f3["item"]:
    req = item["request"]
    headers = req.setdefault("header", [])
    ensure_header(headers, "x-tenant-id", "{{tenantId}}")
    
    # Fix serial_number and ensure valid JSON in raw body
    if "Open Savings Account" in item["name"]:
        body = {
            "customer_id": "{{customerId}}",
            "account_name": "Emeka Okafor Savings",
            "gl_code": "2110001",
            "account_type": "SAVINGS",
            "currency": "NGN",
            "bank_code": "{{bankCode}}",
            "serial_number": 100001,
            "overdraft_limit": "0.00"
        }
        ensure_json_body(req, body)
    elif "Open Current Account" in item["name"]:
        body = {
            "customer_id": "{{customerId}}",
            "account_name": "Lagos Tech Ventures Current",
            "gl_code": "2110002",
            "account_type": "CURRENT",
            "currency": "NGN",
            "bank_code": "{{bankCode}}",
            "serial_number": 100002,
            "overdraft_limit": "50000.00"
        }
        ensure_json_body(req, body)
    elif "Invalid Bank Code" in item["name"]:
        body = {
            "customer_id": "{{customerId}}",
            "account_name": "Invalid Code Test",
            "gl_code": "2110001",
            "account_type": "SAVINGS",
            "currency": "NGN",
            "bank_code": "5",
            "serial_number": 123456,
            "overdraft_limit": "0.00"
        }
        ensure_json_body(req, body)

# ---------------------------------------------------------------------------
# 3. Update Folder 4: Account Controls & Liens
# ---------------------------------------------------------------------------
f4 = col["item"][3]
f4["description"] = (
    "Powered by `cv-account-manager`. Enforces regulatory compliance restrictions (`POST_NO_DEBIT`, `FROZEN`) "
    "and financial liens/holds under Central Bank regulations. Scoped by `x-tenant-id` header to prevent cross-tenant restriction tampering."
)

for item in f4["item"]:
    req = item["request"]
    headers = req.setdefault("header", [])
    ensure_header(headers, "x-tenant-id", "{{tenantId}}")

# ---------------------------------------------------------------------------
# 4. Update Folder 5: Double-Entry Transfers
# ---------------------------------------------------------------------------
f5 = col["item"][4]
f5["description"] = (
    "Powered by `cv-double-entry-engine`. Enforces the fundamental invariant of double-entry bookkeeping: "
    "every transaction consists of balanced posting legs where Total Debits strictly equals Total Credits in the same currency. "
    "Guarantees Zero Floating Point representation and validates against account restriction flags and tenant rules."
)

# ---------------------------------------------------------------------------
# 5. Build New Folder 6: Payment Rails & External Payouts
# ---------------------------------------------------------------------------
f_rails = {
    "name": "6. Payment Rails & External Payouts (Paystack & Anchor)",
    "description": (
        "Powered by `cv-payment-gateway`, `cv-rail-paystack`, and `cv-rail-anchor`. Agnostic multi-rail routing infrastructure "
        "supporting pluggable adapters (Anchor BaaS, Paystack Transfers, and Enterprise NIBSS Direct). Implements real-time "
        "name enquiry identity verification, single outward transfer dispatch, transfer recipient generation, external status "
        "verification, settlement account balance inspection, and commercial bank routing directories."
    ),
    "item": [
        {
            "name": "List Registered Payment Rails",
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": [
                            'pm.test("Status code is 200 OK", function () {',
                            '    pm.response.to.have.status(200);',
                            '});',
                            'pm.test("Registered providers contain Anchor and Paystack", function () {',
                            '    var jsonData = pm.response.json();',
                            '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                            '    pm.expect(jsonData.registered_providers).to.include("Anchor");',
                            '    pm.expect(jsonData.registered_providers).to.include("Paystack");',
                            '});'
                        ],
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "GET",
                "header": [
                    {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"}
                ],
                "url": {
                    "raw": "{{baseUrl}}/api/v1/payment-rails",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "payment-rails"]
                },
                "description": "Returns the currently active payment rail provider and the complete list of registered payment adapters available on the gateway."
            },
            "response": [
                {
                    "name": "Success Example - 200 OK",
                    "originalRequest": {
                        "method": "GET",
                        "header": [{"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"}],
                        "url": {"raw": "{{baseUrl}}/api/v1/payment-rails", "host": ["{{baseUrl}}"], "path": ["api", "v1", "payment-rails"]}
                    },
                    "status": "OK",
                    "code": 200,
                    "_postman_previewlanguage": "json",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "cookie": [],
                    "body": json.dumps({
                        "status": "SUCCESS",
                        "active_provider": "Anchor",
                        "registered_providers": ["Anchor", "Paystack"]
                    }, indent=2)
                }
            ]
        },
        {
            "name": "Switch Active Rail to Paystack",
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": [
                            'pm.test("Status code is 200 OK", function () {',
                            '    pm.response.to.have.status(200);',
                            '});',
                            'pm.test("Active rail successfully switched to Paystack", function () {',
                            '    var jsonData = pm.response.json();',
                            '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                            '    pm.expect(jsonData.active_provider).to.eql("Paystack");',
                            '    pm.environment.set("activeRailProvider", "Paystack");',
                            '});'
                        ],
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "POST",
                "header": [
                    {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                    {"key": "Content-Type", "value": "application/json", "type": "text"}
                ],
                "body": {
                    "mode": "raw",
                    "raw": json.dumps({"provider": "Paystack"}, indent=2),
                    "options": {"raw": {"language": "json"}}
                },
                "url": {
                    "raw": "{{baseUrl}}/api/v1/admin/payment-rails/active",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "admin", "payment-rails", "active"]
                },
                "description": "Switches the active payment rail router to Paystack Transfers for all outward transfers and name enquiries."
            },
            "response": [
                {
                    "name": "Success Example - 200 OK",
                    "originalRequest": {
                        "method": "POST",
                        "header": [
                            {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                            {"key": "Content-Type", "value": "application/json", "type": "text"}
                        ],
                        "body": {"mode": "raw", "raw": json.dumps({"provider": "Paystack"}, indent=2), "options": {"raw": {"language": "json"}}},
                        "url": {"raw": "{{baseUrl}}/api/v1/admin/payment-rails/active", "host": ["{{baseUrl}}"], "path": ["api", "v1", "admin", "payment-rails", "active"]}
                    },
                    "status": "OK",
                    "code": 200,
                    "_postman_previewlanguage": "json",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "cookie": [],
                    "body": json.dumps({
                        "status": "SUCCESS",
                        "message": "Payment rail successfully switched to Paystack",
                        "active_provider": "Paystack"
                    }, indent=2)
                }
            ]
        },
        {
            "name": "Switch Active Rail to Anchor",
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": [
                            'pm.test("Status code is 200 OK", function () {',
                            '    pm.response.to.have.status(200);',
                            '});',
                            'pm.test("Active rail switched to Anchor", function () {',
                            '    var jsonData = pm.response.json();',
                            '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                            '    pm.expect(jsonData.active_provider).to.eql("Anchor");',
                            '    pm.environment.set("activeRailProvider", "Anchor");',
                            '});'
                        ],
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "POST",
                "header": [
                    {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                    {"key": "Content-Type", "value": "application/json", "type": "text"}
                ],
                "body": {
                    "mode": "raw",
                    "raw": json.dumps({"provider": "Anchor"}, indent=2),
                    "options": {"raw": {"language": "json"}}
                },
                "url": {
                    "raw": "{{baseUrl}}/api/v1/admin/payment-rails/active",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "admin", "payment-rails", "active"]
                },
                "description": "Switches the active payment rail router back to Anchor BaaS."
            },
            "response": [
                {
                    "name": "Success Example - 200 OK",
                    "originalRequest": {
                        "method": "POST",
                        "header": [
                            {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                            {"key": "Content-Type", "value": "application/json", "type": "text"}
                        ],
                        "body": {"mode": "raw", "raw": json.dumps({"provider": "Anchor"}, indent=2), "options": {"raw": {"language": "json"}}},
                        "url": {"raw": "{{baseUrl}}/api/v1/admin/payment-rails/active", "host": ["{{baseUrl}}"], "path": ["api", "v1", "admin", "payment-rails", "active"]}
                    },
                    "status": "OK",
                    "code": 200,
                    "_postman_previewlanguage": "json",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "cookie": [],
                    "body": json.dumps({
                        "status": "SUCCESS",
                        "message": "Payment rail successfully switched to Anchor",
                        "active_provider": "Anchor"
                    }, indent=2)
                }
            ]
        },
        {
            "name": "Name Enquiry (Identity Verification)",
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": [
                            'pm.test("Status code is 200 OK", function () {',
                            '    pm.response.to.have.status(200);',
                            '});',
                            'pm.test("Account name resolved successfully", function () {',
                            '    var jsonData = pm.response.json();',
                            '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                            '    pm.expect(jsonData.data.account_name).to.not.be.empty;',
                            '});'
                        ],
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "POST",
                "header": [
                    {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                    {"key": "Content-Type", "value": "application/json", "type": "text"}
                ],
                "body": {
                    "mode": "raw",
                    "raw": json.dumps({"bank_code": "058", "account_number": "0580001237"}, indent=2),
                    "options": {"raw": {"language": "json"}}
                },
                "url": {
                    "raw": "{{baseUrl}}/api/v1/transfers/enquiry",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "transfers", "enquiry"]
                },
                "description": "Performs inter-bank beneficiary identity verification (Paystack GET /bank/resolve or Anchor counterparties) to verify beneficiary account name prior to outward dispatch."
            },
            "response": [
                {
                    "name": "Success Example - 200 OK",
                    "originalRequest": {
                        "method": "POST",
                        "header": [
                            {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                            {"key": "Content-Type", "value": "application/json", "type": "text"}
                        ],
                        "body": {"mode": "raw", "raw": json.dumps({"bank_code": "058", "account_number": "0580001237"}, indent=2), "options": {"raw": {"language": "json"}}},
                        "url": {"raw": "{{baseUrl}}/api/v1/transfers/enquiry", "host": ["{{baseUrl}}"], "path": ["api", "v1", "transfers", "enquiry"]}
                    },
                    "status": "OK",
                    "code": 200,
                    "_postman_previewlanguage": "json",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "cookie": [],
                    "body": json.dumps({
                        "status": "SUCCESS",
                        "provider": "Paystack",
                        "data": {
                            "account_number": "0580001237",
                            "account_name": "COREVAULT PAYSTACK BENEFICIARY",
                            "bank_code": "058",
                            "kyc_tier": 3
                        }
                    }, indent=2)
                }
            ]
        },
        {
            "name": "Create Transfer Recipient",
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": [
                            'pm.test("Status code is 201 Created", function () {',
                            '    pm.response.to.have.status(201);',
                            '});',
                            'pm.test("Recipient code received and saved", function () {',
                            '    var jsonData = pm.response.json();',
                            '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                            '    pm.expect(jsonData.data.recipient_code).to.not.be.empty;',
                            '    pm.environment.set("recipientCode", jsonData.data.recipient_code);',
                            '});'
                        ],
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "POST",
                "header": [
                    {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                    {"key": "Content-Type", "value": "application/json", "type": "text"}
                ],
                "body": {
                    "mode": "raw",
                    "raw": json.dumps({
                        "name": "Jane Doe",
                        "account_number": "0581111111",
                        "bank_code": "058",
                        "currency": "NGN",
                        "description": "Vendor payout recipient"
                    }, indent=2),
                    "options": {"raw": {"language": "json"}}
                },
                "url": {
                    "raw": "{{baseUrl}}/api/v1/transfers/recipients",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "transfers", "recipients"]
                },
                "description": "Registers a transfer recipient with the payment provider (Paystack POST /transferrecipient) returning a persistent recipient code (`RCP_...`) for automated payouts."
            },
            "response": [
                {
                    "name": "Success Example - 201 Created",
                    "originalRequest": {
                        "method": "POST",
                        "header": [
                            {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                            {"key": "Content-Type", "value": "application/json", "type": "text"}
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": json.dumps({
                                "name": "Jane Doe",
                                "account_number": "0581111111",
                                "bank_code": "058",
                                "currency": "NGN",
                                "description": "Vendor payout recipient"
                            }, indent=2),
                            "options": {"raw": {"language": "json"}}
                        },
                        "url": {"raw": "{{baseUrl}}/api/v1/transfers/recipients", "host": ["{{baseUrl}}"], "path": ["api", "v1", "transfers", "recipients"]}
                    },
                    "status": "Created",
                    "code": 201,
                    "_postman_previewlanguage": "json",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "cookie": [],
                    "body": json.dumps({
                        "status": "SUCCESS",
                        "active_provider": "Paystack",
                        "data": {
                            "recipient_code": "RCP_paystack_b7e95f68b31c",
                            "name": "Jane Doe",
                            "account_number": "0581111111",
                            "bank_code": "058",
                            "currency": "NGN",
                            "recipient_type": "nuban",
                            "description": "Vendor payout recipient"
                        }
                    }, indent=2)
                }
            ]
        },
        {
            "name": "Single Outward Transfer Dispatch",
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": [
                            'pm.test("Status code is 201 Created", function () {',
                            '    pm.response.to.have.status(201);',
                            '});',
                            'pm.test("Transfer dispatched with valid session ID", function () {',
                            '    var jsonData = pm.response.json();',
                            '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                            '    pm.expect(jsonData.data.session_id).to.not.be.empty;',
                            '    pm.environment.set("transferReference", jsonData.data.reference);',
                            '    pm.environment.set("transferSessionId", jsonData.data.session_id);',
                            '});'
                        ],
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "POST",
                "header": [
                    {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                    {"key": "Content-Type", "value": "application/json", "type": "text"}
                ],
                "body": {
                    "mode": "raw",
                    "raw": json.dumps({
                        "source_account_id": "00000000-0000-0000-0000-000000000001",
                        "destination_account": "0582222222",
                        "destination_bank_code": "058",
                        "amount": "15000.00",
                        "currency": "NGN",
                        "narration": "Vendor payout via Paystack single transfer"
                    }, indent=2),
                    "options": {"raw": {"language": "json"}}
                },
                "url": {
                    "raw": "{{baseUrl}}/api/v1/transfers/outward",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "transfers", "outward"]
                },
                "description": "Initiates an outward NIP/inter-bank single payout via the currently active payment rail (Paystack POST /transfer or Anchor NIP transfer), debiting the source account with full ledger guarantees."
            },
            "response": [
                {
                    "name": "Success Example - 201 Created",
                    "originalRequest": {
                        "method": "POST",
                        "header": [
                            {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"},
                            {"key": "Content-Type", "value": "application/json", "type": "text"}
                        ],
                        "body": {
                            "mode": "raw",
                            "raw": json.dumps({
                                "source_account_id": "00000000-0000-0000-0000-000000000001",
                                "destination_account": "0582222222",
                                "destination_bank_code": "058",
                                "amount": "15000.00",
                                "currency": "NGN",
                                "narration": "Vendor payout via Paystack single transfer"
                            }, indent=2),
                            "options": {"raw": {"language": "json"}}
                        },
                        "url": {"raw": "{{baseUrl}}/api/v1/transfers/outward", "host": ["{{baseUrl}}"], "path": ["api", "v1", "transfers", "outward"]}
                    },
                    "status": "Created",
                    "code": 201,
                    "_postman_previewlanguage": "json",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "cookie": [],
                    "body": json.dumps({
                        "status": "SUCCESS",
                        "provider": "Paystack",
                        "data": {
                            "transaction_id": "d0000000-0000-0000-0000-000000000099",
                            "reference": "TXN_OUT_b4f91c78",
                            "session_id": "TRF_ps_9a7d2e4f",
                            "response_code": "00",
                            "message": "Paystack single transfer dispatched successfully for 1500000 kobo (ref: TXN_OUT_b4f91c78)",
                            "amount": "15000.00",
                            "currency": "NGN",
                            "destination_account": "0582222222",
                            "destination_bank_code": "058"
                        }
                    }, indent=2)
                }
            ]
        },
        {
            "name": "Query Active Rail Settlement Balance",
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": [
                            'pm.test("Status code is 200 OK", function () {',
                            '    pm.response.to.have.status(200);',
                            '});',
                            'pm.test("Balance returned", function () {',
                            '    var jsonData = pm.response.json();',
                            '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                            '    pm.expect(jsonData.balance).to.not.be.empty;',
                            '});'
                        ],
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "GET",
                "header": [
                    {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"}
                ],
                "url": {
                    "raw": "{{baseUrl}}/api/v1/payment-rails/balance?currency=NGN",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "payment-rails", "balance"],
                    "query": [{"key": "currency", "value": "NGN"}]
                },
                "description": "Queries the operational settlement account balance with the active payment rail provider (Paystack GET /balance or Anchor account ledger balance)."
            },
            "response": [
                {
                    "name": "Success Example - 200 OK",
                    "originalRequest": {
                        "method": "GET",
                        "header": [{"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"}],
                        "url": {"raw": "{{baseUrl}}/api/v1/payment-rails/balance?currency=NGN", "host": ["{{baseUrl}}"], "path": ["api", "v1", "payment-rails", "balance"], "query": [{"key": "currency", "value": "NGN"}]}
                    },
                    "status": "OK",
                    "code": 200,
                    "_postman_previewlanguage": "json",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "cookie": [],
                    "body": json.dumps({
                        "status": "SUCCESS",
                        "active_provider": "Paystack",
                        "currency": "NGN",
                        "balance": "25000000"
                    }, indent=2)
                }
            ]
        },
        {
            "name": "Verify Outward Transfer Status",
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": [
                            'pm.test("Status code is 200 OK", function () {',
                            '    pm.response.to.have.status(200);',
                            '});',
                            'pm.test("Transfer verified", function () {',
                            '    var jsonData = pm.response.json();',
                            '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                            '    pm.expect(jsonData.response_code).to.eql("00");',
                            '});'
                        ],
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "GET",
                "header": [
                    {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"}
                ],
                "url": {
                    "raw": "{{baseUrl}}/api/v1/transfers/verify/{{transferReference}}",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "transfers", "verify", "{{transferReference}}"]
                },
                "description": "Queries and verifies the definitive delivery and settlement status of an outward transfer (Paystack GET /transfer/verify/:reference)."
            },
            "response": [
                {
                    "name": "Success Example - 200 OK",
                    "originalRequest": {
                        "method": "GET",
                        "header": [{"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"}],
                        "url": {"raw": "{{baseUrl}}/api/v1/transfers/verify/{{transferReference}}", "host": ["{{baseUrl}}"], "path": ["api", "v1", "transfers", "verify", "{{transferReference}}"]}
                    },
                    "status": "OK",
                    "code": 200,
                    "_postman_previewlanguage": "json",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "cookie": [],
                    "body": json.dumps({
                        "status": "SUCCESS",
                        "active_provider": "Paystack",
                        "reference": "TXN_OUT_b4f91c78",
                        "session_id": "TRF_ver_TXN_OUT_b4f91c78",
                        "response_code": "00",
                        "message": "Paystack transfer reference 'TXN_OUT_b4f91c78' verified successfully with status 'success'"
                    }, indent=2)
                }
            ]
        },
        {
            "name": "List Supported Commercial Banks",
            "event": [
                {
                    "listen": "test",
                    "script": {
                        "exec": [
                            'pm.test("Status code is 200 OK", function () {',
                            '    pm.response.to.have.status(200);',
                            '});',
                            'pm.test("Banks list returned", function () {',
                            '    var jsonData = pm.response.json();',
                            '    pm.expect(jsonData.status).to.eql("SUCCESS");',
                            '    pm.expect(jsonData.banks.length).to.be.above(0);',
                            '});'
                        ],
                        "type": "text/javascript"
                    }
                }
            ],
            "request": {
                "method": "GET",
                "header": [
                    {"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"}
                ],
                "url": {
                    "raw": "{{baseUrl}}/api/v1/transfers/banks",
                    "host": ["{{baseUrl}}"],
                    "path": ["api", "v1", "transfers", "banks"]
                },
                "description": "Returns directory of supported commercial banks and CBN routing codes (Paystack GET /bank) required for name enquiry and outward payout routing."
            },
            "response": [
                {
                    "name": "Success Example - 200 OK",
                    "originalRequest": {
                        "method": "GET",
                        "header": [{"key": "Authorization", "value": "Bearer {{authToken}}", "type": "text"}],
                        "url": {"raw": "{{baseUrl}}/api/v1/transfers/banks", "host": ["{{baseUrl}}"], "path": ["api", "v1", "transfers", "banks"]}
                    },
                    "status": "OK",
                    "code": 200,
                    "_postman_previewlanguage": "json",
                    "header": [{"key": "Content-Type", "value": "application/json"}],
                    "cookie": [],
                    "body": json.dumps({
                        "status": "SUCCESS",
                        "banks": [
                            {"name": "Access Bank", "code": "044"},
                            {"name": "Guaranty Trust Bank", "code": "058"},
                            {"name": "Zenith Bank", "code": "057"},
                            {"name": "First Bank of Nigeria", "code": "011"},
                            {"name": "United Bank For Africa", "code": "033"}
                        ]
                    }, indent=2)
                }
            ]
        }
    ]
}

# Insert Folder 6 into items if not already present
if not any("Payment Rails" in f["name"] for f in col["item"]):
    col["item"].insert(5, f_rails)
else:
    # replace existing
    idx = next(i for i, f in enumerate(col["item"]) if "Payment Rails" in f["name"])
    col["item"][idx] = f_rails

# ---------------------------------------------------------------------------
# 6. Update Folder 7 (formerly 6): Platform & Jurisdiction Configuration
# ---------------------------------------------------------------------------
f7 = col["item"][6]
f7["name"] = "7. Platform & Jurisdiction Configuration (cv-platform-config-parser)"
f7["description"] = (
    "Powered by `cv-platform-config-parser`. Exposes declarative multi-country rules parsed from local TOML "
    "configurations (Nigeria NUBAN/CBN rules, UK Sort Code/FCA rules) enabling multi-jurisdiction platformization and compliance."
)

# ---------------------------------------------------------------------------
# 7. Update Folder 8 (formerly 7): Audit Trail
# ---------------------------------------------------------------------------
f8 = col["item"][7]
f8["name"] = "8. Audit Trail (cv-audit)"
f8["description"] = (
    "Powered by `cv-audit`. Exposes the append-only tamper-evident audit trail capturing administrative modifications, "
    "lifecycle status updates, and critical banking operations for compliance and regulators. Enforces tenant scoping via `x-tenant-id` header."
)
for item in f8["item"]:
    headers = item["request"].setdefault("header", [])
    ensure_header(headers, "x-tenant-id", "{{tenantId}}")

# ---------------------------------------------------------------------------
# 8. Update Folder 9 (formerly 8): Admin Tenant & Client Governance
# ---------------------------------------------------------------------------
f9 = col["item"][8]
f9["name"] = "9. Admin Tenant & Client Governance"
f9["description"] = (
    "Super Admin endpoints for onboarding external client organizations, client credential generation, whitelisting, "
    "blacklisting, and platform automated violation simulation."
)
for item in f9["item"]:
    if "Whitelist" in item["name"]:
        headers = item["request"].setdefault("header", [])
        ensure_header(headers, "Authorization", "Bearer {{authToken}}")
        ensure_header(headers, "Content-Type", "application/json")
        item["request"]["body"] = {
            "mode": "raw",
            "raw": json.dumps({"note": "Approved by super admin"}, indent=2),
            "options": {"raw": {"language": "json"}}
        }

# ---------------------------------------------------------------------------
# 9. Update Folder 10 (formerly 9): Tenant Custom PND Rules
# ---------------------------------------------------------------------------
f10 = col["item"][9]
f10["name"] = "10. Tenant Custom PND Rules"
f10["description"] = (
    "Powered by `cv-account-manager` and `cv-transaction-api`. Endpoints allowing tenants to configure custom compliance, "
    "AML, and risk thresholds (e.g. KYC single caps, daily velocity) that automatically trigger system Post-No-Debits. "
    "Scoped strictly via the `x-tenant-id` header without passing tenant IDs in the URL path."
)

for item in f10["item"]:
    headers = item["request"].setdefault("header", [])
    ensure_header(headers, "x-tenant-id", "{{tenantId}}")
    if "List Active PND Rules" in item["name"]:
        # Update URL from /api/v1/tenants/{{tenantId}}/pnd-rules to /api/v1/tenants/pnd-rules
        item["request"]["url"] = {
            "raw": "{{baseUrl}}/api/v1/tenants/pnd-rules",
            "host": ["{{baseUrl}}"],
            "path": ["api", "v1", "tenants", "pnd-rules"]
        }

# ---------------------------------------------------------------------------
# Validate all POST/PUT/PATCH bodies
# ---------------------------------------------------------------------------
for folder in col["item"]:
    for item in folder.get("item", []):
        req = item["request"]
        method = req["method"]
        if method in ("POST", "PUT", "PATCH"):
            body = req.get("body", {})
            if body.get("mode") != "raw":
                print(f"Warning: {item['name']} ({method}) does not use raw body")
            else:
                raw_text = body.get("raw", "")
                try:
                    json.loads(raw_text)
                except Exception as e:
                    print(f"ERROR: {item['name']} has invalid JSON: {e}")

with open("postman/corevault_collection.json", "w") as f:
    json.dump(col, f, indent=2)

print("postman/corevault_collection.json updated successfully!")
