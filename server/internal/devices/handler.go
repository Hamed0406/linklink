package devices

import (
	"encoding/json"
	"errors"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/linklink/server/internal/auth"
	"github.com/rs/zerolog/log"
)

const (
	contentTypeJSON = "application/json"
	headerCT        = "Content-Type"
	msgInternalErr  = "internal error"
	msgNotFound     = "device not found"
)

type Handler struct {
	svc *Service
}

func NewHandler(svc *Service) *Handler {
	return &Handler{svc: svc}
}

func (h *Handler) Routes() func(r chi.Router) {
	return func(r chi.Router) {
		r.Post("/register", h.Register)
		r.Get("/", h.List)
		r.Route("/{id}", func(r chi.Router) {
			r.Get("/", h.Get)
			r.Post("/approve", h.Approve)
			r.Post("/revoke", h.Revoke)
			r.Delete("/", h.Delete)
			r.Post("/heartbeat", h.Heartbeat)
			r.Get("/config", h.Config)
		})
	}
}

func (h *Handler) Register(w http.ResponseWriter, r *http.Request) {
	claims := auth.ClaimsFromContext(r.Context())
	var req RegisterRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		jsonErr(w, "invalid request body", http.StatusBadRequest)
		return
	}
	dev, err := h.svc.Register(r.Context(), claims.UserID, req)
	if err != nil {
		switch {
		case errors.Is(err, ErrInvalidName):
			jsonErr(w, err.Error(), http.StatusBadRequest)
		case errors.Is(err, ErrInvalidPublicKey):
			jsonErr(w, err.Error(), http.StatusBadRequest)
		case errors.Is(err, ErrDuplicatePublicKey):
			jsonErr(w, err.Error(), http.StatusConflict)
		default:
			log.Error().Err(err).Str("user_id", claims.UserID).Msg("register device")
			jsonErr(w, msgInternalErr, http.StatusInternalServerError)
		}
		return
	}
	w.Header().Set(headerCT, contentTypeJSON)
	w.WriteHeader(http.StatusCreated)
	json.NewEncoder(w).Encode(dev)
}

func (h *Handler) List(w http.ResponseWriter, r *http.Request) {
	claims := auth.ClaimsFromContext(r.Context())
	devs, err := h.svc.List(r.Context(), claims.UserID, claims.Role)
	if err != nil {
		jsonErr(w, msgInternalErr, http.StatusInternalServerError)
		return
	}
	if devs == nil {
		devs = []Device{}
	}
	jsonOK(w, devs)
}

func (h *Handler) Get(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	dev, err := h.svc.GetByID(r.Context(), id)
	if err != nil {
		if errors.Is(err, ErrNotFound) {
			jsonErr(w, msgNotFound, http.StatusNotFound)
			return
		}
		jsonErr(w, msgInternalErr, http.StatusInternalServerError)
		return
	}
	claims := auth.ClaimsFromContext(r.Context())
	if claims.Role != "admin" && dev.UserID != claims.UserID {
		jsonErr(w, "forbidden", http.StatusForbidden)
		return
	}
	jsonOK(w, dev)
}

func (h *Handler) Approve(w http.ResponseWriter, r *http.Request) {
	claims := auth.ClaimsFromContext(r.Context())
	if claims.Role != "admin" {
		jsonErr(w, "admin role required", http.StatusForbidden)
		return
	}
	id := chi.URLParam(r, "id")
	if err := h.svc.Approve(r.Context(), claims.UserID, id); err != nil {
		switch {
		case errors.Is(err, ErrNotPending):
			jsonErr(w, err.Error(), http.StatusBadRequest)
		case errors.Is(err, ErrNotFound):
			jsonErr(w, msgNotFound, http.StatusNotFound)
		default:
			jsonErr(w, msgInternalErr, http.StatusInternalServerError)
		}
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (h *Handler) Revoke(w http.ResponseWriter, r *http.Request) {
	claims := auth.ClaimsFromContext(r.Context())
	id := chi.URLParam(r, "id")
	dev, err := h.svc.GetByID(r.Context(), id)
	if err != nil {
		jsonErr(w, msgNotFound, http.StatusNotFound)
		return
	}
	if claims.Role != "admin" && dev.UserID != claims.UserID {
		jsonErr(w, "forbidden", http.StatusForbidden)
		return
	}
	if err := h.svc.Revoke(r.Context(), claims.UserID, id); err != nil {
		switch {
		case errors.Is(err, ErrAlreadyRevoked):
			jsonErr(w, err.Error(), http.StatusBadRequest)
		default:
			jsonErr(w, msgInternalErr, http.StatusInternalServerError)
		}
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (h *Handler) Delete(w http.ResponseWriter, r *http.Request) {
	claims := auth.ClaimsFromContext(r.Context())
	id := chi.URLParam(r, "id")
	dev, err := h.svc.GetByID(r.Context(), id)
	if err != nil {
		jsonErr(w, msgNotFound, http.StatusNotFound)
		return
	}
	if claims.Role != "admin" && dev.UserID != claims.UserID {
		jsonErr(w, "forbidden", http.StatusForbidden)
		return
	}
	_, err = h.svc.db.Exec(r.Context(), `DELETE FROM devices WHERE id=$1`, id)
	if err != nil {
		jsonErr(w, msgInternalErr, http.StatusInternalServerError)
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func (h *Handler) Heartbeat(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	var body struct {
		ExternalEndpoint string `json:"external_endpoint"`
		ConfigVersion    int    `json:"config_version"`
	}
	if err := json.NewDecoder(r.Body).Decode(&body); err != nil {
		jsonErr(w, "invalid request body", http.StatusBadRequest)
		return
	}
	hasUpdate, err := h.svc.Heartbeat(r.Context(), id, body.ExternalEndpoint, body.ConfigVersion)
	if err != nil {
		if errors.Is(err, ErrNotApproved) {
			jsonErr(w, "device not approved", http.StatusForbidden)
			return
		}
		jsonErr(w, msgInternalErr, http.StatusInternalServerError)
		return
	}
	jsonOK(w, map[string]bool{"has_update": hasUpdate})
}

func (h *Handler) Config(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	cfg, err := h.svc.GetConfig(r.Context(), id)
	if err != nil {
		switch {
		case errors.Is(err, ErrNotApproved):
			jsonErr(w, "device not approved", http.StatusForbidden)
		case errors.Is(err, ErrNotFound):
			jsonErr(w, msgNotFound, http.StatusNotFound)
		default:
			jsonErr(w, msgInternalErr, http.StatusInternalServerError)
		}
		return
	}
	jsonOK(w, cfg)
}

func jsonOK(w http.ResponseWriter, v any) {
	w.Header().Set(headerCT, contentTypeJSON)
	json.NewEncoder(w).Encode(v)
}

func jsonErr(w http.ResponseWriter, msg string, code int) {
	w.Header().Set(headerCT, contentTypeJSON)
	w.WriteHeader(code)
	json.NewEncoder(w).Encode(map[string]string{"error": msg})
}
