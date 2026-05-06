package auth

import (
	"encoding/json"
	"net/http"

	"github.com/google/uuid"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/rs/zerolog/log"
)

// DevTokenHandler handles POST /api/v1/auth/dev-token.
// Only available when AUTH_LOCAL_ENABLED=true (development only).
// Creates an admin user if one doesn't exist and returns a ready-to-use access token.
func DevTokenHandler(db *pgxpool.Pool, secret string) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()

		// Upsert a fixed dev admin user
		const devUserID = "00000000-0000-0000-0000-000000000001"
		const devEmail = "dev@linklink.local"
		_, err := db.Exec(ctx,
			`INSERT INTO users (id, email, role)
			 VALUES ($1, $2, 'admin')
			 ON CONFLICT (id) DO NOTHING`,
			devUserID, devEmail,
		)
		if err != nil {
			log.Error().Err(err).Msg("dev-token: upsert user")
			http.Error(w, "internal error", http.StatusInternalServerError)
			return
		}

		token, err := GenerateAccessToken(secret, devUserID, "admin")
		if err != nil {
			log.Error().Err(err).Msg("dev-token: generate token")
			http.Error(w, "internal error", http.StatusInternalServerError)
			return
		}

		// Also generate and store a refresh token
		plain, hash, err := GenerateRefreshToken()
		if err != nil {
			http.Error(w, "internal error", http.StatusInternalServerError)
			return
		}
		_, err = db.Exec(ctx,
			`INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at)
			 VALUES ($1, $2, $3, $4)`,
			uuid.New().String(), devUserID, hash, RefreshTokenExpiry(),
		)
		if err != nil {
			log.Error().Err(err).Msg("dev-token: store refresh token")
			http.Error(w, "internal error", http.StatusInternalServerError)
			return
		}

		w.Header().Set(headerCT, contentTypeJSON)
		json.NewEncoder(w).Encode(map[string]any{
			"access_token":  token,
			"refresh_token": plain,
			"user_id":       devUserID,
			"role":          "admin",
			"note":          "dev-only endpoint — not available in production",
		})
	}
}
