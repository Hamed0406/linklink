-- +migrate Up
CREATE TABLE acl_rules (
    id                    UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    name                  TEXT         NOT NULL,
    network_id            UUID         REFERENCES networks(id),
    source_device_id      UUID         REFERENCES devices(id),      -- NULL = any device
    destination_device_id UUID         REFERENCES devices(id),      -- NULL = any device
    protocol              acl_protocol NOT NULL DEFAULT 'any',
    port_min              INTEGER,
    port_max              INTEGER,
    action                acl_action   NOT NULL DEFAULT 'allow',
    priority              INTEGER      NOT NULL DEFAULT 100,
    created_at            TIMESTAMPTZ  NOT NULL DEFAULT now()
);
