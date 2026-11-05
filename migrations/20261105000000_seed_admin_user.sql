-- Seed Initial Default Tenant and Super Admin User
INSERT INTO tenants (id, code, name, country_code, base_currency, is_active, client_id, client_secret_hash, status, scopes)
VALUES (
    'a0000000-0000-0000-0000-000000000001',
    'WALLET_APP',
    'CoreVault Native Wallet App',
    'NG',
    'NGN',
    true,
    'client_wallet_app_seed001',
    '$argon2id$v=19$m=19456,t=2,p=1$W/Afuu7AAhdbh+DfBEej1w$danfgELzPUtLYBmTeqxcPQoXDTIHbNwaaiZBplv4wDU',
    'ACTIVE',
    ARRAY['transfers:read', 'transfers:write', 'accounts:read', 'accounts:write', 'admin:read']
)
ON CONFLICT (id) DO UPDATE SET
    client_id = EXCLUDED.client_id,
    client_secret_hash = EXCLUDED.client_secret_hash,
    status = EXCLUDED.status,
    scopes = EXCLUDED.scopes;

INSERT INTO users (id, tenant_id, email, password_hash, first_name, last_name, role, is_active)
VALUES (
    '00000000-0000-0000-0000-000000000001',
    'a0000000-0000-0000-0000-000000000001',
    'admin@corevault.bank',
    '$argon2id$v=19$m=19456,t=2,p=1$2EKBk2ShZ0l4lU6lRN++jA$1Uew0+X9Vma66STHRqQ98bv5+z2/4WpHMXf4o+ClKBY',
    'System',
    'Administrator',
    'SUPER_ADMIN',
    true
)
ON CONFLICT (email) DO UPDATE SET
    password_hash = EXCLUDED.password_hash,
    role = EXCLUDED.role,
    is_active = EXCLUDED.is_active;

-- Seed Initial Baseline Customer
INSERT INTO customers (id, tenant_id, customer_number, customer_type, kyc_tier, first_name, last_name, phone_number, status)
VALUES ('c0000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', 'CUST_001', 'INDIVIDUAL', 1, 'Amina', 'Bello', '+2348012345678', 'ACTIVE')
ON CONFLICT (id) DO NOTHING;

-- Seed Initial Baseline Accounts for Intra-Bank Transfers
INSERT INTO accounts (id, tenant_id, account_number, account_name, gl_code, account_type, currency_code, ledger_balance, available_balance, status)
VALUES 
('00000000-0000-0000-0000-000000000001', 'a0000000-0000-0000-0000-000000000001', '0581111111', 'Source Account', '2110001', 'SAVINGS', 'NGN', 10000000.0000, 10000000.0000, 'ACTIVE'),
('00000000-0000-0000-0000-000000000002', 'a0000000-0000-0000-0000-000000000001', '0582222222', 'Destination Account', '2110002', 'SAVINGS', 'NGN', 5000000.0000, 5000000.0000, 'ACTIVE')
ON CONFLICT (id) DO UPDATE SET 
    ledger_balance = EXCLUDED.ledger_balance,
    available_balance = EXCLUDED.available_balance,
    status = EXCLUDED.status;
