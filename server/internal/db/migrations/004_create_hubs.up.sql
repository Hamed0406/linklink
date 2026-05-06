-- +migrate Up
CREATE TABLE hubs (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name         TEXT        NOT NULL,
    public_key   TEXT        NOT NULL UNIQUE,
    public_ip    INET        NOT NULL,
    port         INTEGER     NOT NULL DEFAULT 51820,
    port_alt     INTEGER     NOT NULL DEFAULT 443,
    api_key_hash TEXT        NOT NULL,
    network_id   UUID        REFERENCES networks(id),
    status       TEXT        NOT NULL DEFAULT 'active',
    last_seen_at TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
