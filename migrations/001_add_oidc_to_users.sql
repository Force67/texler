-- Add OIDC support to users table
-- Migration: 001_add_oidc_to_users.sql

-- Add new columns for OIDC authentication
ALTER TABLE users
ADD COLUMN IF NOT EXISTS auth_method TEXT NOT NULL DEFAULT 'password',
ADD COLUMN IF NOT EXISTS oidc_provider TEXT,
ADD COLUMN IF NOT EXISTS oidc_provider_id TEXT;

-- Make password_hash nullable for OIDC users
ALTER TABLE users
ALTER COLUMN password_hash DROP NOT NULL;

-- Create enum type for auth_method
DO $$ BEGIN
    CREATE TYPE auth_method_enum AS ENUM ('password', 'oidc');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Update the auth_method column to use the enum
ALTER TABLE users
ALTER COLUMN auth_method TYPE auth_method_enum
USING auth_method::auth_method_enum;

-- Add indexes for OIDC fields
CREATE INDEX IF NOT EXISTS idx_users_oidc_provider ON users(oidc_provider) WHERE oidc_provider IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_oidc_provider_id ON users(oidc_provider_id) WHERE oidc_provider_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_auth_method ON users(auth_method);

-- Add constraint to ensure OIDC fields are set when auth_method is 'oidc'
ALTER TABLE users
ADD CONSTRAINT IF NOT EXISTS check_oidc_fields
CHECK (
    (auth_method = 'oidc' AND oidc_provider IS NOT NULL AND oidc_provider_id IS NOT NULL) OR
    (auth_method = 'password')
);

-- Create unique constraint for OIDC provider + provider_id combination
ALTER TABLE users
ADD CONSTRAINT IF NOT EXISTS unique_oidc_provider_user
UNIQUE (oidc_provider, oidc_provider_id)
WHERE oidc_provider IS NOT NULL AND oidc_provider_id IS NOT NULL;