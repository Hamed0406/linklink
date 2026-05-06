-- +migrate Up
CREATE TABLE devices (
    id                UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID          NOT NULL REFERENCES users(id),
    network_id        UUID          REFERENCES networks(id),
    name              TEXT          NOT NULL,
    os                TEXT,
    hostname          TEXT,
    public_key        TEXT          NOT NULL UNIQUE,
    tunnel_ip         INET          NOT NULL UNIQUE,
    external_endpoint TEXT,
    status            device_status NOT NULL DEFAULT 'pending',
    is_relay          BOOLEAN       NOT NULL DEFAULT false,
    config_version    INTEGER       NOT NULL DEFAULT 0,
    last_seen_at      TIMESTAMPTZ,
    created_at        TIMESTAMPTZ   NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ   NOT NULL DEFAULT now()
);

CREATE INDEX idx_devices_user_id    ON devices(user_id);
CREATE INDEX idx_devices_status     ON devices(status);
CREATE INDEX idx_devices_network_id ON devices(network_id);
