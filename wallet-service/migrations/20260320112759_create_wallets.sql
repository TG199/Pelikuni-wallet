--  Wallets table

CREATE TABLE wallets {
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id VARCHAR(100) NOT NULL,
    balance DECIMAL(19, 4) NOT NULL DEFAULT 0 CHECK (balance >= 0),
    version BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
}

-- index for user lookups
CREATE INDEX idx_wallets_user_id ON wallets(user_id);

-- Trigger to update updated_at
CREATE OR REPLACE FUNCTION trigger_set_timestamps()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER set_timestamp
BEFORE UPDATE ON wallets
FOR EACH ROW
EXECUTE FUNCTION trigger_set_timestamp();