-- +migrate Up
CREATE TYPE device_status AS ENUM ('pending', 'approved', 'revoked', 'disabled');
CREATE TYPE acl_action   AS ENUM ('allow', 'deny');
CREATE TYPE acl_protocol AS ENUM ('any', 'tcp', 'udp', 'icmp');
