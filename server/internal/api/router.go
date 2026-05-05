package api

import (
	"encoding/json"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/linklink/server/internal/audit"
	"github.com/linklink/server/internal/auth"
	"github.com/linklink/server/internal/devices"
)

func NewRouter(db *pgxpool.Pool, jwtSecret string) http.Handler {
	r := chi.NewRouter()

	// Global middleware
	r.Use(middleware.Recoverer)
	r.Use(middleware.RequestID)
	r.Use(Logger)
	r.Use(SecurityHeaders)

	// Services
	auditSvc := audit.NewService(db)
	deviceSvc := devices.NewService(db, auditSvc)
	deviceHandler := devices.NewHandler(deviceSvc)
	auditHandler := audit.NewHandler(db)

	deviceFlowStore := auth.NewDeviceFlowStore()
	deviceFlowHandler := auth.NewDeviceFlowHandler(deviceFlowStore, db, jwtSecret)

	authMiddleware := auth.Middleware(jwtSecret)

	// Health (no auth)
	r.Get("/healthz", LivenessHandler)
	r.Get("/readyz", ReadinessHandler(db))

	// API v1
	r.Route("/api/v1", func(r chi.Router) {

		// Auth endpoints
		r.Route("/auth", func(r chi.Router) {
			r.Post("/device", deviceFlowHandler.StartDevice)
			r.Post("/token", deviceFlowHandler.PollToken)
			r.Post("/refresh", deviceFlowHandler.Refresh)
			r.Post("/logout", deviceFlowHandler.Logout)
			r.Get("/me", func(w http.ResponseWriter, req *http.Request) {
				authMiddleware(http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
					claims := auth.ClaimsFromContext(req.Context())
					w.Header().Set("Content-Type", "application/json")
					json.NewEncoder(w).Encode(map[string]string{
						"user_id": claims.UserID,
						"role":    claims.Role,
					})
				})).ServeHTTP(w, req)
			})
			// Web activation page (used by browser after login)
			r.Post("/activate", func(w http.ResponseWriter, req *http.Request) {
				authMiddleware(http.HandlerFunc(func(w http.ResponseWriter, req *http.Request) {
					claims := auth.ClaimsFromContext(req.Context())
					deviceFlowHandler.Activate(w, req, claims.UserID, claims.Role)
				})).ServeHTTP(w, req)
			})
		})

		// Authenticated routes
		r.Group(func(r chi.Router) {
			r.Use(authMiddleware)

			// Devices
			r.Route("/devices", deviceHandler.Routes())

			// Audit logs
			r.Get("/audit-logs", auditHandler.List)
		})
	})

	return r
}
