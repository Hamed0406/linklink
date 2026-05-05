package auth

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"sync"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rs/zerolog/log"
)

const (
	deviceCodeTTL   = 15 * time.Minute
	pollInterval    = 5 * time.Second
)

type pendingCode struct {
	userCode  string
	expiresAt time.Time
	userID    string // filled after user activates
	role      string
	done      bool
}

// DeviceFlowStore manages in-memory device authorization codes.
// In production this would be replaced with Redis.
type DeviceFlowStore struct {
	mu    sync.Mutex
	codes map[string]*pendingCode // key = device_code
}

func NewDeviceFlowStore() *DeviceFlowStore {
	s := &DeviceFlowStore{codes: make(map[string]*pendingCode)}
	go s.reapExpired()
	return s
}

func (s *DeviceFlowStore) reapExpired() {
	ticker := time.NewTicker(time.Minute)
	for range ticker.C {
		s.mu.Lock()
		for k, v := range s.codes {
			if time.Now().After(v.expiresAt) {
				delete(s.codes, k)
			}
		}
		s.mu.Unlock()
	}
}

type DeviceFlowHandler struct {
	store  *DeviceFlowStore
	db     *pgxpool.Pool
	secret string
}

func NewDeviceFlowHandler(store *DeviceFlowStore, db *pgxpool.Pool, secret string) *DeviceFlowHandler {
	return &DeviceFlowHandler{store: store, db: db, secret: secret}
}

// POST /api/v1/auth/device
func (h *DeviceFlowHandler) StartDevice(w http.ResponseWriter, r *http.Request) {
	deviceCode, err := randomHex(32)
	if err != nil {
		jsonError(w, "internal error", http.StatusInternalServerError)
		return
	}
	userCode := generateUserCode()
	h.store.mu.Lock()
	h.store.codes[deviceCode] = &pendingCode{
		userCode:  userCode,
		expiresAt: time.Now().Add(deviceCodeTTL),
	}
	h.store.mu.Unlock()

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"device_code":      deviceCode,
		"user_code":        userCode,
		"verification_uri": r.Header.Get("Origin") + "/activate",
		"expires_in":       int(deviceCodeTTL.Seconds()),
		"interval":         int(pollInterval.Seconds()),
	})
}

// POST /api/v1/auth/token — agent polls this
func (h *DeviceFlowHandler) PollToken(w http.ResponseWriter, r *http.Request) {
	var body struct {
		GrantType  string `json:"grant_type"`
		DeviceCode string `json:"device_code"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		jsonError(w, "invalid request body", http.StatusBadRequest)
		return
	}

	h.store.mu.Lock()
	entry, ok := h.store.codes[body.DeviceCode]
	h.store.mu.Unlock()

	if !ok {
		jsonError(w, "expired_token", http.StatusBadRequest)
		return
	}
	if time.Now().After(entry.expiresAt) {
		h.store.mu.Lock()
		delete(h.store.codes, body.DeviceCode)
		h.store.mu.Unlock()
		jsonError(w, "expired_token", http.StatusBadRequest)
		return
	}
	if !entry.done {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusBadRequest)
		json.NewEncoder(w).Encode(map[string]string{"error": "authorization_pending"})
		return
	}

	// Issue tokens
	access, err := GenerateAccessToken(h.secret, entry.userID, entry.role)
	if err != nil {
		jsonError(w, "internal error", http.StatusInternalServerError)
		return
	}
	plain, hash, err := GenerateRefreshToken()
	if err != nil {
		jsonError(w, "internal error", http.StatusInternalServerError)
		return
	}
	exp := RefreshTokenExpiry()
	_, err = h.db.Exec(r.Context(),
		`INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)`,
		entry.userID, hash, exp,
	)
	if err != nil {
		log.Error().Err(err).Msg("store refresh token")
		jsonError(w, "internal error", http.StatusInternalServerError)
		return
	}

	// Remove device code (single-use)
	h.store.mu.Lock()
	delete(h.store.codes, body.DeviceCode)
	h.store.mu.Unlock()

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"access_token":  access,
		"refresh_token": plain,
		"token_type":    "Bearer",
		"expires_in":    int(accessTokenTTL.Seconds()),
	})
}

// POST /api/v1/auth/activate — user submits the code via web
func (h *DeviceFlowHandler) Activate(w http.ResponseWriter, r *http.Request, userID, role string) {
	var body struct {
		UserCode string `json:"user_code"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		jsonError(w, "invalid request body", http.StatusBadRequest)
		return
	}
	userCode := strings.ToUpper(strings.ReplaceAll(body.UserCode, "-", ""))

	h.store.mu.Lock()
	defer h.store.mu.Unlock()
	for _, entry := range h.store.codes {
		normalized := strings.ToUpper(strings.ReplaceAll(entry.userCode, "-", ""))
		if normalized == userCode && !entry.done && time.Now().Before(entry.expiresAt) {
			entry.done = true
			entry.userID = userID
			entry.role = role
			w.Header().Set("Content-Type", "application/json")
			json.NewEncoder(w).Encode(map[string]string{"status": "authorized"})
			return
		}
	}
	jsonError(w, "invalid or expired user code", http.StatusBadRequest)
}

// POST /api/v1/auth/logout — revokes a refresh token
func (h *DeviceFlowHandler) Logout(w http.ResponseWriter, r *http.Request) {
	var body struct {
		RefreshToken string `json:"refresh_token"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		jsonError(w, "invalid request body", http.StatusBadRequest)
		return
	}
	hash := HashToken(body.RefreshToken)
	_, err := h.db.Exec(r.Context(),
		`UPDATE refresh_tokens SET revoked_at = now() WHERE token_hash = $1`, hash,
	)
	if err != nil {
		log.Error().Err(err).Msg("revoke refresh token")
	}
	w.WriteHeader(http.StatusNoContent)
}

// POST /api/v1/auth/refresh — issues a new access token from a refresh token
func (h *DeviceFlowHandler) Refresh(w http.ResponseWriter, r *http.Request) {
	var body struct {
		RefreshToken string `json:"refresh_token"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		jsonError(w, "invalid request body", http.StatusBadRequest)
		return
	}
	hash := HashToken(body.RefreshToken)

	var userID, role string
	var expiresAt time.Time
	var revokedAt *time.Time
	err := h.db.QueryRow(r.Context(),
		`SELECT rt.user_id, u.role, rt.expires_at, rt.revoked_at
		 FROM refresh_tokens rt JOIN users u ON u.id = rt.user_id
		 WHERE rt.token_hash = $1`, hash,
	).Scan(&userID, &role, &expiresAt, &revokedAt)
	if err != nil || revokedAt != nil || time.Now().After(expiresAt) {
		jsonError(w, "invalid or expired refresh token", http.StatusUnauthorized)
		return
	}

	access, err := GenerateAccessToken(h.secret, userID, role)
	if err != nil {
		jsonError(w, "internal error", http.StatusInternalServerError)
		return
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]any{
		"access_token": access,
		"token_type":   "Bearer",
		"expires_in":   int(accessTokenTTL.Seconds()),
	})
}

// ── helpers ──────────────────────────────────────────────────────────────────

func randomHex(n int) (string, error) {
	b := make([]byte, n)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	return hex.EncodeToString(b), nil
}

func generateUserCode() string {
	// Format: XXXX-XXXX (8 alphanumeric chars, uppercase)
	const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789" // no confusable chars
	b := make([]byte, 32)
	rand.Read(b)
	result := make([]byte, 9)
	for i := 0; i < 4; i++ {
		result[i] = chars[int(b[i])%len(chars)]
	}
	result[4] = '-'
	for i := 0; i < 4; i++ {
		result[5+i] = chars[int(b[4+i])%len(chars)]
	}
	return string(result)
}

func jsonError(w http.ResponseWriter, msg string, code int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	fmt.Fprintf(w, `{"error":%q}`, msg)
}
