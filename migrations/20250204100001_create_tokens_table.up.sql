-- Create token type enum for different token purposes
CREATE TYPE token_type AS ENUM ('password_reset', 'email_verification', 'email_change');

-- Create tokens table for password reset, email verification, etc.
CREATE TABLE tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL,
    token_type token_type NOT NULL,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    used_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Store additional data like new email for email_change tokens
    metadata JSONB
);

-- Index for looking up tokens by user
CREATE INDEX idx_tokens_user_id ON tokens(user_id);

-- Index for finding valid (unused, not expired) tokens
CREATE INDEX idx_tokens_expires_at ON tokens(expires_at) WHERE used_at IS NULL;

-- Index for looking up by token hash (for validation)
CREATE INDEX idx_tokens_token_hash ON tokens(token_hash) WHERE used_at IS NULL;

COMMENT ON TABLE tokens IS 'Stores verification tokens for password reset, email verification, etc.';
COMMENT ON COLUMN tokens.token_hash IS 'SHA-256 hash of the actual token (token is sent to user, hash stored)';
COMMENT ON COLUMN tokens.metadata IS 'Additional data like new_email for email change tokens';
