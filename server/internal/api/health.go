package api

import (
	"context"
	"encoding/json"
	"net/http"
	"time"

	"github.com/jackc/pgx/v5/pgxpool"
)

const (
	contentTypeJSON = "application/json"
	headerCT        = "Content-Type"
)

// LivenessHandler handles GET /healthz — always returns 200 if the process is up.
func LivenessHandler(w http.ResponseWriter, r *http.Request) {
	w.Header().Set(headerCT, contentTypeJSON)
	json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
}

// ReadinessHandler handles GET /readyz — returns 200 only when the DB is reachable.
func ReadinessHandler(db *pgxpool.Pool) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx, cancel := context.WithTimeout(r.Context(), 2*time.Second)
		defer cancel()
		if err := db.Ping(ctx); err != nil {
			w.Header().Set(headerCT, contentTypeJSON)
			w.WriteHeader(http.StatusServiceUnavailable)
			json.NewEncoder(w).Encode(map[string]string{"status": "db_unavailable"})
			return
		}
		w.Header().Set(headerCT, contentTypeJSON)
		json.NewEncoder(w).Encode(map[string]string{"status": "ok"})
	}
}
