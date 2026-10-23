-- CoreVault V1 Production DDL Schema (PostgreSQL 16)
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- 1. Tenants Table
CREATE TABLE IF NOT EXISTS tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(16) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    country_code CHAR(2) NOT NULL DEFAULT 'NG',
    base_currency CHAR(3) NOT NULL DEFAULT 'NGN',
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Staff Users & IAM
CREATE TABLE IF NOT EXISTS users (
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
CREATE TABLE IF NOT EXISTS customers (
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

CREATE INDEX IF NOT EXISTS idx_customers_bvn_hash ON customers(bvn_hash);
CREATE INDEX IF NOT EXISTS idx_customers_phone ON customers(phone_number);
CREATE INDEX IF NOT EXISTS idx_customers_search ON customers USING gin ((first_name || ' ' || last_name) gin_trgm_ops);

-- 4. Chart of Accounts (COA)
CREATE TABLE IF NOT EXISTS chart_of_accounts (
    gl_code VARCHAR(16) PRIMARY KEY,
    name VARCHAR(128) NOT NULL,
    category VARCHAR(16) NOT NULL CHECK (category IN ('ASSET', 'LIABILITY', 'EQUITY', 'INCOME', 'EXPENSE')),
    is_active BOOLEAN NOT NULL DEFAULT true
);

-- Seed Basic GL Accounts
INSERT INTO chart_of_accounts (gl_code, name, category) VALUES
    ('1110001', 'Cash in Vault', 'ASSET'),
    ('1120001', 'Settlement Account with Central Bank', 'ASSET'),
    ('2110001', 'Customer Savings Deposits', 'LIABILITY'),
    ('2110002', 'Customer Current Deposits', 'LIABILITY'),
    ('2110003', 'Escrow Suspense Account', 'LIABILITY'),
    ('4110001', 'Interest Income on Loans', 'INCOME'),
    ('4210001', 'Transaction Fee Income', 'INCOME')
ON CONFLICT (gl_code) DO NOTHING;

-- 5. Bank Accounts
CREATE TABLE IF NOT EXISTS accounts (
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

CREATE INDEX IF NOT EXISTS idx_accounts_customer ON accounts(customer_id);

-- 6. Transactions Header Table (Partitioned by created_at monthly)
CREATE TABLE IF NOT EXISTS transactions (
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

-- Partition for 2026 Q4
CREATE TABLE IF NOT EXISTS transactions_2026_10 PARTITION OF transactions
    FOR VALUES FROM ('2026-10-01 00:00:00+00') TO ('2026-11-01 00:00:00+00');
CREATE TABLE IF NOT EXISTS transactions_2026_11 PARTITION OF transactions
    FOR VALUES FROM ('2026-11-01 00:00:00+00') TO ('2026-12-01 00:00:00+00');

-- 7. Immutable Journal Entries Table (Partitioned by created_at monthly)
CREATE TABLE IF NOT EXISTS journal_entries (
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

CREATE TABLE IF NOT EXISTS journal_entries_2026_10 PARTITION OF journal_entries
    FOR VALUES FROM ('2026-10-01 00:00:00+00') TO ('2026-11-01 00:00:00+00');
CREATE TABLE IF NOT EXISTS journal_entries_2026_11 PARTITION OF journal_entries
    FOR VALUES FROM ('2026-11-01 00:00:00+00') TO ('2026-12-01 00:00:00+00');

CREATE INDEX IF NOT EXISTS idx_journal_account_date ON journal_entries(account_id, created_at DESC);

-- 8. Audit Logs Table
CREATE TABLE IF NOT EXISTS audit_logs (
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

CREATE INDEX IF NOT EXISTS idx_audit_entity ON audit_logs(entity_name, entity_id);
