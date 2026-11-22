-- Accounts, their sessions, and attempts counted for rate limiting.

CREATE TABLE users (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- as typed when signing up; unique regardless of case
    username TEXT NOT NULL,
    -- argon2id, in PHC string format
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_username_key ON users (lower(username));

CREATE TABLE sessions (
    -- SHA-256 of the token in the cookie; the token itself is never stored
    token_hash BYTEA PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX sessions_user_id ON sessions (user_id);
CREATE INDEX sessions_expires_at ON sessions (expires_at);

-- e.g. failed logins for a username or from an address
CREATE TABLE attempts (
    key TEXT NOT NULL,
    at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX attempts_key_at ON attempts (key, at);
CREATE INDEX attempts_at ON attempts (at);
