package config

import (
	"fmt"
	"os"
	"strconv"
)

type Config struct {
	DatabaseURL      string
	JWTSecret        string
	Env              string // "development" | "production"
	Port             int
	AuthLocalEnabled bool
	STUNServer       string
	MigrationsPath   string
}

func Load() (*Config, error) {
	cfg := &Config{
		DatabaseURL:      getEnv("DATABASE_URL", ""),
		JWTSecret:        getEnv("JWT_SECRET", "dev-secret-change-me"),
		Env:              getEnv("ENV", "development"),
		Port:             getEnvInt("PORT", 8080),
		AuthLocalEnabled: getEnvBool("AUTH_LOCAL_ENABLED", false),
		STUNServer:       getEnv("STUN_SERVER", "stun.l.google.com:19302"),
		MigrationsPath:   getEnv("MIGRATIONS_PATH", "migrations"),
	}

	if cfg.DatabaseURL == "" {
		return nil, fmt.Errorf("DATABASE_URL is required")
	}

	if cfg.Env == "production" && cfg.JWTSecret == "dev-secret-change-me" {
		return nil, fmt.Errorf(
			"JWT_SECRET must be changed from the default value in production",
		)
	}

	return cfg, nil
}

func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func getEnvInt(key string, fallback int) int {
	if v := os.Getenv(key); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			return n
		}
	}
	return fallback
}

func getEnvBool(key string, fallback bool) bool {
	if v := os.Getenv(key); v != "" {
		b, err := strconv.ParseBool(v)
		if err == nil {
			return b
		}
	}
	return fallback
}
