-- +migrate Up
CREATE TABLE networks (
    id         UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    name       TEXT        NOT NULL,
    cidr       CIDR        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Default network
INSERT INTO networks (name, cidr) VALUES ('default', '10.44.0.0/24');
