package audit

import (
	"context"
	"encoding/json"
	"net/http"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rs/zerolog/log"
)

type LogEntry struct {
	ID          string         `json:"id"`
	ActorUserID *string        `json:"actor_user_id,omitempty"`
	Action      string         `json:"action"`
	TargetType  *string        `json:"target_type,omitempty"`
	TargetID    *string        `json:"target_id,omitempty"`
	Metadata    map[string]any `json:"metadata,omitempty"`
	CreatedAt   time.Time      `json:"created_at"`
}

type Service struct {
	db *pgxpool.Pool
}

func NewService(db *pgxpool.Pool) *Service {
	return &Service{db: db}
}

// Log records an audit event. Never logs private keys, tokens, or packet content.
// Safe to call with nil actorUserID (system actions) or nil targetID.
func (s *Service) Log(ctx context.Context, actorUserID *string, action, targetType string, targetID *string, metadata map[string]any) {
	metaJSON, _ := json.Marshal(metadata)
	_, err := s.db.Exec(ctx,
		`INSERT INTO audit_logs (actor_user_id, action, target_type, target_id, metadata)
		 VALUES ($1, $2, $3, $4, $5)`,
		actorUserID, action, nullStr(targetType), targetID, metaJSON,
	)
	if err != nil {
		log.Error().Err(err).Str("action", action).Msg("failed to write audit log")
	}
}

// Handler serves GET /api/v1/audit-logs
type Handler struct {
	db *pgxpool.Pool
}

func NewHandler(db *pgxpool.Pool) *Handler {
	return &Handler{db: db}
}

func (h *Handler) List(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	action := q.Get("action")
	limit := 100

	var rows interface{ Next() bool }
	var err error

	query := `SELECT id, actor_user_id, action, target_type, target_id::text, metadata, created_at
	          FROM audit_logs`
	args := []any{}
	if action != "" {
		query += " WHERE action = $1"
		args = append(args, action)
	}
	query += " ORDER BY created_at DESC LIMIT $" + itoa(len(args)+1)
	args = append(args, limit)

	pgRows, queryErr := h.db.Query(r.Context(), query, args...)
	if queryErr != nil {
		http.Error(w, `{"error":"internal error"}`, http.StatusInternalServerError)
		return
	}
	defer pgRows.Close()
	_ = rows
	_ = err

	var entries []LogEntry
	for pgRows.Next() {
		var e LogEntry
		var metaBytes []byte
		var targetType, targetID *string
		if scanErr := pgRows.Scan(
			&e.ID, &e.ActorUserID, &e.Action,
			&targetType, &targetID, &metaBytes, &e.CreatedAt,
		); scanErr == nil {
			e.TargetType = targetType
			e.TargetID = targetID
			if metaBytes != nil {
				json.Unmarshal(metaBytes, &e.Metadata)
			}
			entries = append(entries, e)
		}
	}
	if entries == nil {
		entries = []LogEntry{}
	}
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(entries)
}

func nullStr(s string) *string {
	if s == "" {
		return nil
	}
	return &s
}

func itoa(n int) string {
	return string(rune('0' + n))
}
