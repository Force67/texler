-- Create LaTeX engine enum type if it doesn't exist
DO $$ BEGIN
    CREATE TYPE latexengine AS ENUM ('pdflatex', 'xelatex', 'lualatex');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create blacklisted_tokens table for token revocation
CREATE TABLE IF NOT EXISTS blacklisted_tokens (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    jti VARCHAR(255) NOT NULL,              -- JWT ID
    token_type VARCHAR(50) NOT NULL,        -- "access" or "refresh"
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    blacklisted_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    reason VARCHAR(100) NOT NULL DEFAULT 'logout',  -- "logout", "revoke", "admin_action"
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_blacklisted_tokens_jti ON blacklisted_tokens(jti);
CREATE INDEX IF NOT EXISTS idx_blacklisted_tokens_user_id ON blacklisted_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_blacklisted_tokens_expires_at ON blacklisted_tokens(expires_at);

-- Unique constraint to prevent duplicate entries for same token
CREATE UNIQUE INDEX IF NOT EXISTS idx_blacklisted_tokens_jti_unique ON blacklisted_tokens(jti);

-- Trigger to update updated_at
CREATE TRIGGER update_blacklisted_tokens_updated_at BEFORE UPDATE ON blacklisted_tokens
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();