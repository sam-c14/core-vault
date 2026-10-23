# Review: PRD-monolith.md — CoreVault MVP / V1 Modular Monolith

## Overall Verdict

The monolith pivot is **the right call for a V1 / MVP**. Collapsing the distributed sprawl (Kafka, RabbitMQ, OpenSearch, YugabyteDB, OpenBao, ClickHouse) down to PostgreSQL + Valkey + a single Rust binary is a massive reduction in operational surface area and exactly what you need to ship, prove product-market fit, and deploy on a free cloud tier.

That said, the current document is **a skeleton, not a PRD**. It reads more like a quick architecture memo. Below I break down what's strong, what's missing, and what needs to change — especially considering your Open Core model and Oracle Cloud free-tier deployment target.

---

## What's Strong ✅

| Aspect | Notes |
|--------|-------|
| **Modular monolith architecture** | Correct decision. Axum + internal workspace crates = microservice boundaries without microservice infrastructure. |
| **PostgreSQL-only for V1** | Using `pg_trgm` / `tsvector` instead of OpenSearch, partitioning for journals, and `LISTEN/NOTIFY` for async tasks — excellent pragmatism. |
| **Valkey over Redis** | Correct. BSD 3-Clause, wire-compatible, zero licensing risk. |
| **Append-only balance derivation** | Smart. Avoids `SELECT FOR UPDATE` hotspot on balance rows. This is how Tigerbeetle, Moov, and modern ledgers work. |
| **Application-layer AES-256-GCM** | Appropriate for V1 — no need for OpenBao's complexity yet. |

---

## Critical Gaps 🚨

### 1. Open Core Licensing Model Is Completely Absent

You mentioned this is an **Open Core product** where:
- The core engine (double-entry ledger, NUBAN, journal processing) is **open source**
- Multi-country compliance adapters, currency engines, and platformization modules are **closed source (proprietary)**

The PRD has **zero** mention of this. This is arguably the most important architectural decision in the document because it dictates:
- Which crates live in a public repo vs. a private repo
- How the closed-source modules are loaded (compiled-in feature flags? Dynamic `.so`/`.dylib` plugins? Separate gRPC sidecar?)
- Licensing headers on every file
- What the open-source community can see, fork, and contribute to

> [!CAUTION]
> Without defining the Open Core boundary in the PRD, you risk accidentally open-sourcing proprietary IP or, worse, designing a monolith where the closed modules can't be cleanly separated later.

### 2. Oracle Cloud Free Tier Deployment Is Not Addressed

The PRD says "Deployed via Docker containers on a managed Kubernetes cluster or dedicated VMs within a Nigerian data center." That's the production story. But you explicitly want V1 to run on **Oracle Cloud Infrastructure (OCI) Always Free Tier**. The PRD needs to account for those constraints:

| OCI Free Tier Resource | Limit |
|------------------------|-------|
| ARM Ampere A1 Compute | 4 OCPUs, 24 GB RAM (total, up to 4 VMs) |
| AMD Micro VMs | 2 VMs, 1 OCPU, 1 GB RAM each |
| Block Storage | 200 GB total |
| Object Storage | 20 GB |
| Load Balancer | 1 flexible LB (10 Mbps) |
| Autonomous Database | 2 instances, 1 OCPU, 20 GB each |
| Outbound Data | 10 TB/month |

The ARM Ampere A1 instances are extremely generous for a free tier. A single 4-OCPU / 24 GB ARM VM can comfortably run the entire CoreVault stack:
- CoreVault Rust binary (~20–50 MB, negligible memory)
- PostgreSQL 16
- Valkey
- Reverse proxy (Caddy or Traefik, not APISIX — see below)

But this changes several design decisions that the PRD currently ignores.

### 3. Apache APISIX Is Overkill for V1

APISIX requires **etcd** (a 3-node distributed consensus system) as its configuration store. On a free-tier single VM, you're now running:
- CoreVault binary
- PostgreSQL
- Valkey
- APISIX
- etcd (which itself wants 3 nodes for HA)

That's absurd for MVP. Replace with:
- **Caddy** (Apache 2.0) — automatic HTTPS, reverse proxy, rate limiting plugin, ~15 MB binary, zero config for basic use
- Or the Rust application itself handles routing directly (Axum already is an HTTP server)

### 4. No Admin Module

PRD-2.md had a full Admin Module (RBAC, maker-checker, branch management, audit trails). PRD-monolith.md has **none of it**. Even for MVP, a core banking system needs:
- At minimum: user authentication, role-based access, and an audit log
- The maker-checker workflow for transactions above a threshold

### 5. No Customer / KYC Module

PRD-2.md had customer types, KYC tiers (CBN Tier 1/2/3), BVN/NIN integration, document storage. PRD-monolith.md mentions "Customer & KYC" as a box in the architecture diagram but has zero specification. Without customers, you don't have accounts. Without KYC tiers, you can't enforce CBN transaction limits.

### 6. Schema Is Incomplete

The DDL only defines `transactions` and `journal_entries`. Missing:
- `tenants` (even for single-tenant V1, define it for forward compatibility)
- `customers`
- `accounts` (referenced by `journal_entries` but not defined!)
- `chart_of_accounts`
- `audit_logs`
- `users` / `roles` / `permissions`

### 7. No API Surface Defined

Not a single endpoint is specified. An MVP PRD needs at minimum:
- Account CRUD
- Transaction submission
- Balance inquiry
- Customer onboarding
- Authentication

### 8. No Observability

No mention of logging, metrics, or health checks. Even on a free tier VM, you need structured JSON logging to stdout and a `/health` endpoint for the load balancer.

### 9. Append-Only Balance Derivation: Incomplete Design

The PRD mentions this as a strategy but doesn't explain:
- How balances are materialized (PostgreSQL materialized view? Background worker? On-read aggregation with caching?)
- What happens during concurrent writes
- How the available balance (ledger balance minus holds/liens) is computed

---

## Open Core Architecture Proposal

Here's how I'd structure the crate workspace to cleanly separate open-source from proprietary:

```
corevault/
├── Cargo.toml                    # Workspace root
│
├── crates/                       # ── OPEN SOURCE (Apache 2.0 / MIT) ──
│   ├── cv-types/                 # Core domain types (Money, AccountId, EntryType)
│   ├── cv-ledger/                # Double-entry engine, journal validation
│   ├── cv-accounts/              # Account CRUD, NUBAN generation (Nigeria-specific in open core)
│   ├── cv-customers/             # Customer profiles, basic KYC data model
│   ├── cv-auth/                  # Authentication, RBAC, session management
│   ├── cv-audit/                 # Append-only audit log
│   └── cv-api/                   # Axum HTTP handlers, gRPC service definitions
│
├── crates-proprietary/           # ── CLOSED SOURCE (Proprietary License) ──
│   ├── cv-platform-engine/       # Country adapter registry, pluggable compliance trait impls
│   ├── cv-compliance-ng/         # Nigeria: CBN rules, NIBSS NIP adapter, BVN verification
│   ├── cv-compliance-gb/         # UK: FCA rules, Faster Payments, IBAN adapter
│   ├── cv-compliance-ke/         # Kenya: CBK rules, M-Pesa adapter
│   ├── cv-currency-engine/       # Multi-currency FX, conversion rules, settlement logic
│   ├── cv-fee-engine/            # Fee computation, tiered/capped/percentage schedules
│   └── cv-reporting/             # Regulatory report generation (CBN returns, IFRS)
│
├── src/
│   └── main.rs                   # Binary entry point — composes all crates
│
├── docker-compose.yml
└── Dockerfile
```

**Integration mechanism:** The open-source `cv-ledger` and `cv-accounts` crates define **traits** (e.g., `ComplianceEngine`, `CurrencyConverter`, `FeeCalculator`). The closed-source crates provide **implementations**. The binary `main.rs` wires them together at compile time via Cargo feature flags:

```toml
# Cargo.toml (workspace root)
[features]
default = ["nigeria"]
nigeria = ["cv-compliance-ng"]
uk = ["cv-compliance-gb"]
enterprise = ["cv-fee-engine", "cv-currency-engine", "cv-reporting"]
```

The open-source build ships with a **stub/no-op compliance adapter** that always returns `Ok(())` — useful for development and testing but not production-grade. The proprietary crates are distributed as:
- Pre-compiled static libraries (`.rlib`) for paying customers
- Or a separate private Cargo registry (e.g., Cloudsmith, Shipyard)

---

## Oracle Cloud Free Tier Deployment Architecture

```
┌─────────────────────────────────────────────────────┐
│  Oracle Cloud Always Free: ARM A1 VM                │
│  4 OCPU (Ampere) / 24 GB RAM / 200 GB Block        │
│                                                     │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────┐  │
│  │ CoreVault   │  │ PostgreSQL   │  │  Valkey   │  │
│  │ Rust Binary │  │     16       │  │   7/8     │  │
│  │ (Axum)      │  │              │  │           │  │
│  │ Port 8080   │  │ Port 5432    │  │ Port 6379 │  │
│  └──────┬──────┘  └──────────────┘  └───────────┘  │
│         │                                           │
│  ┌──────▼──────┐                                    │
│  │   Caddy     │  ← Automatic HTTPS via Let's       │
│  │ Port 80/443 │    Encrypt. Reverse proxy to 8080   │
│  └─────────────┘                                    │
│                                                     │
│  Managed by: systemd or docker-compose              │
└───────────────────┬─────────────────────────────────┘
                    │
          OCI Free Load Balancer (10 Mbps)
                    │
              Public Internet
```

**Why this works:**
- The Rust binary compiles to a ~20–50 MB static binary with ~10–30 MB RSS at idle. Even under load, it won't exceed 200 MB RAM.
- PostgreSQL 16 with `shared_buffers = 4GB` and `effective_cache_size = 12GB` on 24 GB RAM is extremely comfortable.
- Valkey uses ~50 MB baseline.
- Caddy is a single binary (~40 MB) with zero-config automatic HTTPS — massively simpler than APISIX + etcd on a free tier.
- Total RAM usage at idle: ~5 GB out of 24 GB available. Plenty of headroom.

**Cross-compilation note:** OCI free-tier ARM VMs are `aarch64`. The Rust binary must be cross-compiled:
```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

---

## Recommended Next Steps

1. **Expand PRD-monolith.md** to include all the missing sections (Admin, Customer/KYC, API surface, full schema, observability, Open Core boundary)
2. **Define the Open Core licensing split** — which crates are Apache 2.0 and which are proprietary
3. **Replace Apache APISIX with Caddy** for the MVP (reintroduce APISIX when you move to multi-node Kubernetes)
4. **Add the OCI Free Tier deployment section** with resource budget
5. **Flesh out the append-only balance strategy** — materialized view vs. cached aggregation vs. on-read computation

Would you like me to produce an expanded PRD-monolith-v2.md that incorporates all of this?
