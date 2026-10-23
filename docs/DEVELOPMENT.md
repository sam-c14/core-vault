# CoreVault Developer Guide

Welcome to the **CoreVault** contributor and developer guide. CoreVault is an enterprise-grade, cloud-native core banking application engineered in Rust under an Open Core architecture.

---

## 🛠️ 1. Prerequisites

Ensure your workstation has the following tools installed:

| Tool | Recommended Version | Purpose |
| :--- | :--- | :--- |
| **Rust Toolchain** | 1.81.0+ stable | Core language compiler & Cargo package manager |
| **Docker & Docker Compose** | 24.0+ / Compose v2 | Local containerized infrastructure (PostgreSQL, Valkey) |
| **PostgreSQL Client (`psql`)** | 16+ | Direct database inspection and migration management |
| **Node.js & Newman** | Node 18+, Newman 5.3+ | Automated command-line API collection test runner |
| **cURL & jq** | Latest | Ad-hoc HTTP testing and JSON formatting |

Install the Rust toolchain via `rustup` if you haven't already:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable
```

Install Newman CLI globally:
```bash
npm install -g newman
```

---

## 🏗️ 2. Repository Architecture

CoreVault is organized as a Cargo workspace with dedicated crates enforcing domain-driven design and strict separation of concerns:

```
cba/
├── Cargo.toml                              # Workspace manifest
├── Dockerfile                              # Multi-stage Cargo Chef build
├── docker-compose.yml                      # PostgreSQL 16 & Valkey 7.2 services
├── Caddyfile                               # Reverse proxy & TLS termination
├── config/
│   └── jurisdictions/
│       ├── NG.toml                         # Nigeria (NUBAN, EMTL, KYC Tier limits)
│       └── GB.toml                         # United Kingdom (Sort Code, FCA limits)
├── crates/
│   ├── cv-types/                           # Core money primitives & extension traits
│   ├── cv-double-entry-engine/             # Double-entry balance invariant validation
│   ├── cv-nuban-validator/                 # CBN Mod-10 NUBAN generator & validator
│   ├── cv-account-manager/                 # State machine, holds, liens, and PND
│   ├── cv-platform-config-parser/          # TOML multi-country rule parser
│   ├── cv-audit/                           # Append-only audit trail traits & sinks
│   ├── cv-auth/                            # Staff Argon2id hashing & JWT authentication
│   └── cv-transaction-api/                 # Axum REST controllers & routing
├── docs/                                   # Architectural, API, and platform specifications
│   ├── ARCHITECTURE.md
│   ├── API_REFERENCE.md
│   ├── DEVELOPMENT.md
│   ├── PLATFORMIZATION.md
│   └── SECURITY_AUDIT.md
├── migrations/                             # SQL migrations
│   └── 20261022000000_init_corevault_schema.sql
├── postman/                                # Newman test collection & environment
│   ├── corevault_collection.json
│   ├── corevault_environment.json
│   └── README.md
└── src/
    └── main.rs                             # Monolith entrypoint & Tokio async runtime
```

---

## ⚡ 3. Local Environment Setup

### Step 1: Clone and Inspect
```bash
git clone https://github.com/your-org/corevault.git
cd cba
```

### Step 2: Environment Configuration
CoreVault reads environment variables from the process environment. Create a `.env` file or export them directly:

```bash
# PostgreSQL Connection
export DATABASE_URL="postgres://cv_user:cv_password@localhost:5432/corevault_db"

# Valkey / Redis Connection
export VALKEY_URL="redis://localhost:6379"

# Server Port & Binding
export PORT=8080
export HOST="0.0.0.0"

# JWT Secret Key (min 32 characters)
export JWT_SECRET="super_secret_jwt_key_change_in_production_123456"

# Structured Logging
export RUST_LOG="info,corevault=debug,tower_http=info"
```

### Step 3: Spin Up Infrastructure Containers
Use the provided `docker-compose.yml` to start PostgreSQL 16 and Valkey 7.2:

```bash
docker compose up -d postgres valkey
```

Verify that the containers are healthy:
```bash
docker compose ps
```

### Step 4: Run Database Migrations
Initialize the schema and seed default Chart of Accounts (GL codes):

```bash
psql "postgres://cv_user:cv_password@localhost:5432/corevault_db" \
  -f migrations/20261022000000_init_corevault_schema.sql
```

---

## 🚀 4. Compiling and Running

### Build the Entire Workspace
```bash
cargo build --workspace
```

### Run the Monolith Application
```bash
cargo run --bin corevault
```

The server binds to `0.0.0.0:8080`. You will see structured JSON startup logs:
```json
{"timestamp":"2026-10-23T08:00:00Z","level":"INFO","fields":{"message":"CoreVault V1 Monolith starting up on 0.0.0.0:8080"}}
```

### Quick Sanity Liveness Check
```bash
curl -s http://127.0.0.1:8080/health/live | jq .
```
Expected output:
```json
{
  "status": "UP",
  "service": "corevault-monolith",
  "timestamp": "2026-10-23T08:00:01Z"
}
```

---

## 🧪 5. Testing & Verification

CoreVault maintains a multi-tier testing strategy combining Rust unit/integration tests and Newman end-to-end API test suites.

### 5.1 Rust Workspace Tests
Run all 33+ unit and integration tests across every workspace crate:
```bash
cargo test --workspace
```

Run tests with full output:
```bash
cargo test --workspace -- --nocapture
```

### 5.2 Strict Linting (Clippy)
CoreVault enforces zero compiler warnings:
```bash
cargo clippy --workspace --tests -- -D warnings
```

### 5.3 Code Formatting
```bash
cargo fmt --all -- --check
```

### 5.4 End-to-End API Testing (Newman CLI)
With the server running on `http://127.0.0.1:8080`:
```bash
newman run postman/corevault_collection.json \
  -e postman/corevault_environment.json \
  --bail
```
This executes all 21 API scenarios, validating status codes, JSON response schemas, NUBAN check digits, and authentication tokens.

---

## 📐 6. Core Banking Invariants & Rules

When contributing to CoreVault, you must strictly uphold the following mathematical and architectural invariants:

### 1. Absolute Zero Floating Point Rule
- **Never** use `f32` or `f64` anywhere in financial calculation, fee computation, or ledger storage.
- **Always** use `rust_decimal::Decimal` with typed `Money` and `Currency` structs from `cv-types`.
- Enforce rounding modes explicitly (e.g. `Truncation` or `BankersRounding`).

### 2. Strictly Balanced Double-Entry Rule
Every financial movement consists of at least two posting legs.
$$\sum_{\text{currency}} \text{Debits} == \sum_{\text{currency}} \text{Credits}$$
No transaction is ever committed if this mathematical invariant fails.

### 3. Zero Raw SQL String Concatenation
- All database queries must use compile-time parameterized statements (`$1`, `$2`) to completely eliminate SQL injection vectors.
- Never interpolate values directly into SQL strings.

### 4. Immutable Append-Only Ledger
- Rows in `cv_ledger_journal_entries` and `cv_ledger_postings` are strictly **immutable**.
- Balances are updated via credit or debit postings; existing records must never be modified or deleted.
- Adjustments or corrections are performed exclusively through reversing journal entries.

### 5. Idempotent API Execution
- All state-altering POST/PUT endpoints require an `Idempotency-Key` HTTP header.
- Cached responses are stored in Valkey to prevent duplicate processing of financial movements during network timeouts.

---

## 🔒 7. Security Best Practices

1. **Password Hashing:** Always use Argon2id with recommended memory and iteration cost parameters (managed via `cv-auth`).
2. **Access Control:** All sensitive endpoints (e.g. account controls, holds, manual journal entries) require valid Bearer JWT tokens with authorized roles (`OPERATIONS_OFFICER`, `COMPLIANCE_OFFICER`, `ADMIN`).
3. **Audit Trails:** Every state alteration must emit an event to the `AuditSink` capturing the actor ID, tenant ID, action, IP address, and payload hash.

---

## ❓ 8. Troubleshooting

### Database Connection Refused
- Check if PostgreSQL is running: `docker compose ps`
- Verify database connectivity: `pg_isready -h localhost -p 5432 -U cv_user`
- Check container logs: `docker compose logs postgres`

### Valkey Connection Failed
- Check Valkey container: `docker compose logs valkey`
- Ping Valkey: `redis-cli -h localhost -p 6379 ping` (should reply `PONG`)

### Port 8080 Already in Use
- Find the conflicting process: `lsof -i :8080`
- Kill the process or specify a different port: `export PORT=8081`
