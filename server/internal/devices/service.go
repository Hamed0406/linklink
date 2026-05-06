package devices

import (
	"context"
	"encoding/base64"
	"errors"
	"fmt"
	"time"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/linklink/server/internal/audit"
)

type Device struct {
	ID               string     `json:"id"`
	UserID           string     `json:"user_id"`
	NetworkID        *string    `json:"network_id,omitempty"`
	Name             string     `json:"name"`
	OS               *string    `json:"os,omitempty"`
	Hostname         *string    `json:"hostname,omitempty"`
	PublicKey        string     `json:"public_key"`
	TunnelIP         string     `json:"tunnel_ip"`
	ExternalEndpoint *string    `json:"external_endpoint,omitempty"`
	Status           string     `json:"status"`
	IsRelay          bool       `json:"is_relay"`
	ConfigVersion    int        `json:"config_version"`
	LastSeenAt       *time.Time `json:"last_seen_at,omitempty"`
	CreatedAt        time.Time  `json:"created_at"`
}

type RegisterRequest struct {
	Name             string  `json:"name"`
	PublicKey        string  `json:"public_key"`
	OS               *string `json:"os"`
	Hostname         *string `json:"hostname"`
	ExternalEndpoint *string `json:"external_endpoint"`
}

type Service struct {
	db    *pgxpool.Pool
	audit *audit.Service
}

func NewService(db *pgxpool.Pool, auditSvc *audit.Service) *Service {
	return &Service{db: db, audit: auditSvc}
}

// Register creates a new device in pending state.
func (s *Service) Register(ctx context.Context, userID string, req RegisterRequest) (*Device, error) {
	if err := validateDeviceName(req.Name); err != nil {
		return nil, err
	}
	if err := validatePublicKey(req.PublicKey); err != nil {
		return nil, err
	}

	// Get default network
	var networkID, networkCIDR string
	err := s.db.QueryRow(ctx,
		`SELECT id, cidr::text FROM networks ORDER BY created_at LIMIT 1`,
	).Scan(&networkID, &networkCIDR)
	if err != nil {
		return nil, fmt.Errorf("get network: %w", err)
	}

	tunnelIP, err := AllocateIP(ctx, s.db, networkCIDR)
	if err != nil {
		return nil, fmt.Errorf("allocate IP: %w", err)
	}

	id := uuid.New().String()
	_, err = s.db.Exec(ctx,
		`INSERT INTO devices
		 (id, user_id, network_id, name, os, hostname, public_key, tunnel_ip, external_endpoint, status)
		 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'pending')`,
		id, userID, networkID, req.Name, req.OS, req.Hostname,
		req.PublicKey, tunnelIP, req.ExternalEndpoint,
	)
	if err != nil {
		if isDuplicateKeyError(err) {
			return nil, ErrDuplicatePublicKey
		}
		return nil, fmt.Errorf("insert device: %w", err)
	}

	s.audit.Log(ctx, &userID, "device_registered", "device", &id, map[string]any{
		"name": req.Name,
	})

	return s.GetByID(ctx, id)
}

// Approve transitions a device from pending → approved.
func (s *Service) Approve(ctx context.Context, actorUserID, deviceID string) error {
	tag, err := s.db.Exec(ctx,
		`UPDATE devices SET status='approved', config_version=config_version+1, updated_at=now()
		 WHERE id=$1 AND status='pending'`,
		deviceID,
	)
	if err != nil {
		return fmt.Errorf("approve device: %w", err)
	}
	if tag.RowsAffected() == 0 {
		return ErrNotPending
	}
	s.audit.Log(ctx, &actorUserID, "device_approved", "device", &deviceID, nil)
	return nil
}

// Revoke transitions a device to revoked and bumps hub config_version.
func (s *Service) Revoke(ctx context.Context, actorUserID, deviceID string) error {
	tag, err := s.db.Exec(ctx,
		`UPDATE devices SET status='revoked', updated_at=now() WHERE id=$1 AND status!='revoked'`,
		deviceID,
	)
	if err != nil {
		return fmt.Errorf("revoke device: %w", err)
	}
	if tag.RowsAffected() == 0 {
		return ErrAlreadyRevoked
	}
	// Bump config_version on all active hubs so they re-sync
	_, _ = s.db.Exec(ctx,
		`UPDATE hubs SET last_seen_at=last_seen_at WHERE status='active'`,
	)
	s.audit.Log(ctx, &actorUserID, "device_revoked", "device", &deviceID, nil)
	return nil
}

// Heartbeat updates last_seen_at and external_endpoint.
// Returns whether the config version has changed since the device last checked.
func (s *Service) Heartbeat(ctx context.Context, deviceID, endpoint string, knownVersion int) (hasUpdate bool, err error) {
	var currentVersion int
	err = s.db.QueryRow(ctx,
		`UPDATE devices SET last_seen_at=now(), external_endpoint=$1, updated_at=now()
		 WHERE id=$2 AND status='approved'
		 RETURNING config_version`,
		endpoint, deviceID,
	).Scan(&currentVersion)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return false, ErrNotApproved
		}
		return false, err
	}
	return currentVersion != knownVersion, nil
}

// GetConfig returns the WireGuard peer config for an approved device.
// The private key is NEVER included — the agent inserts it locally.
func (s *Service) GetConfig(ctx context.Context, deviceID string) (*DeviceConfig, error) {
	dev, err := s.GetByID(ctx, deviceID)
	if err != nil {
		return nil, err
	}
	if dev.Status != "approved" {
		return nil, ErrNotApproved
	}

	// Fetch approved peers in the same network (excluding self)
	rows, err := s.db.Query(ctx,
		`SELECT id, name, public_key, tunnel_ip::text, external_endpoint
		 FROM devices
		 WHERE network_id=$1 AND status='approved' AND id!=$2`,
		dev.NetworkID, deviceID,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var peers []PeerEntry
	for rows.Next() {
		var p PeerEntry
		if err := rows.Scan(&p.ID, &p.Name, &p.PublicKey, &p.TunnelIP, &p.ExternalEndpoint); err == nil {
			p.AllowedIPs = []string{p.TunnelIP + "/32"}
			peers = append(peers, p)
		}
	}

	// Fetch relay endpoints
	relayRows, err := s.db.Query(ctx,
		`SELECT public_key, public_ip::text, port, port_alt FROM hubs WHERE status='active' LIMIT 5`,
	)
	if err != nil {
		return nil, err
	}
	defer relayRows.Close()

	var relays []RelayEntry
	for relayRows.Next() {
		var re RelayEntry
		var ip string
		var port, portAlt int
		if err := relayRows.Scan(&re.PublicKey, &ip, &port, &portAlt); err == nil {
			re.EndpointPrimary = fmt.Sprintf("%s:%d", ip, port)
			re.EndpointFallback = fmt.Sprintf("%s:%d", ip, portAlt)
			relays = append(relays, re)
		}
	}

	s.audit.Log(ctx, nil, "device_config_fetched", "device", &deviceID, nil)

	return &DeviceConfig{
		TunnelIP:      dev.TunnelIP,
		ConfigVersion: dev.ConfigVersion,
		Peers:         peers,
		Relays:        relays,
	}, nil
}

// List returns devices visible to the given user (admins see all).
func (s *Service) List(ctx context.Context, userID, role string) ([]Device, error) {
	var rows pgx.Rows
	var err error
	if role == "admin" {
		rows, err = s.db.Query(ctx,
			`SELECT id, user_id, network_id, name, os, hostname, public_key,
			        tunnel_ip::text, external_endpoint, status, is_relay,
			        config_version, last_seen_at, created_at
			 FROM devices ORDER BY created_at DESC`,
		)
	} else {
		rows, err = s.db.Query(ctx,
			`SELECT id, user_id, network_id, name, os, hostname, public_key,
			        tunnel_ip::text, external_endpoint, status, is_relay,
			        config_version, last_seen_at, created_at
			 FROM devices WHERE user_id=$1 ORDER BY created_at DESC`,
			userID,
		)
	}
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanDevices(rows)
}

// GetByID fetches a single device.
func (s *Service) GetByID(ctx context.Context, deviceID string) (*Device, error) {
	var d Device
	err := s.db.QueryRow(ctx,
		`SELECT id, user_id, network_id, name, os, hostname, public_key,
		        tunnel_ip::text, external_endpoint, status, is_relay,
		        config_version, last_seen_at, created_at
		 FROM devices WHERE id=$1`,
		deviceID,
	).Scan(
		&d.ID, &d.UserID, &d.NetworkID, &d.Name, &d.OS, &d.Hostname,
		&d.PublicKey, &d.TunnelIP, &d.ExternalEndpoint,
		&d.Status, &d.IsRelay, &d.ConfigVersion, &d.LastSeenAt, &d.CreatedAt,
	)
	if err != nil {
		if errors.Is(err, pgx.ErrNoRows) {
			return nil, ErrNotFound
		}
		return nil, err
	}
	return &d, nil
}

// ── types ─────────────────────────────────────────────────────────────────────

type PeerEntry struct {
	ID               string   `json:"id"`
	Name             string   `json:"name"`
	PublicKey        string   `json:"public_key"`
	TunnelIP         string   `json:"tunnel_ip"`
	ExternalEndpoint *string  `json:"external_endpoint,omitempty"`
	AllowedIPs       []string `json:"allowed_ips"`
}

type RelayEntry struct {
	PublicKey        string `json:"public_key"`
	EndpointPrimary  string `json:"endpoint_primary"`
	EndpointFallback string `json:"endpoint_fallback"`
}

type DeviceConfig struct {
	TunnelIP      string       `json:"tunnel_ip"`
	ConfigVersion int          `json:"config_version"`
	Peers         []PeerEntry  `json:"peers"`
	Relays        []RelayEntry `json:"relays"`
}

// ── errors ────────────────────────────────────────────────────────────────────

var (
	ErrNotFound          = errors.New("device not found")
	ErrDuplicatePublicKey = errors.New("a device with this public key is already registered")
	ErrNotPending        = errors.New("device is not in pending state")
	ErrAlreadyRevoked    = errors.New("device is already revoked")
	ErrNotApproved       = errors.New("device is not approved")
	ErrInvalidName       = errors.New("device name must be 1–64 characters")
	ErrInvalidPublicKey  = errors.New("public key must be a 44-character base64 string")
)

// ── validation ────────────────────────────────────────────────────────────────

func validateDeviceName(name string) error {
	if len(name) == 0 || len(name) > 64 {
		return ErrInvalidName
	}
	return nil
}

func validatePublicKey(key string) error {
	b, err := base64.StdEncoding.DecodeString(key)
	if err != nil || len(b) != 32 {
		return ErrInvalidPublicKey
	}
	return nil
}

func isDuplicateKeyError(err error) bool {
	return err != nil && (fmt.Sprintf("%v", err) != "" &&
		(contains(err.Error(), "unique") || contains(err.Error(), "duplicate")))
}

func contains(s, sub string) bool {
	return len(s) >= len(sub) && (s == sub || len(s) > 0 && containsStr(s, sub))
}

func containsStr(s, sub string) bool {
	for i := 0; i <= len(s)-len(sub); i++ {
		if s[i:i+len(sub)] == sub {
			return true
		}
	}
	return false
}

// ── scan helpers ──────────────────────────────────────────────────────────────

func scanDevices(rows pgx.Rows) ([]Device, error) {
	var devs []Device
	for rows.Next() {
		var d Device
		err := rows.Scan(
			&d.ID, &d.UserID, &d.NetworkID, &d.Name, &d.OS, &d.Hostname,
			&d.PublicKey, &d.TunnelIP, &d.ExternalEndpoint,
			&d.Status, &d.IsRelay, &d.ConfigVersion, &d.LastSeenAt, &d.CreatedAt,
		)
		if err != nil {
			return nil, err
		}
		devs = append(devs, d)
	}
	return devs, rows.Err()
}

