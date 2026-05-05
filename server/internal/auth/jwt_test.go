package auth_test

import (
	"testing"
	"time"

	"github.com/linklink/server/internal/auth"
)

const testSecret = "test-secret-for-unit-tests-only"

func TestGenerateAndValidateAccessToken(t *testing.T) {
	tok, err := auth.GenerateAccessToken(testSecret, "user-123", "admin")
	if err != nil {
		t.Fatalf("generate: %v", err)
	}
	claims, err := auth.ValidateAccessToken(testSecret, tok)
	if err != nil {
		t.Fatalf("validate: %v", err)
	}
	if claims.UserID != "user-123" {
		t.Errorf("userID = %q, want %q", claims.UserID, "user-123")
	}
	if claims.Role != "admin" {
		t.Errorf("role = %q, want %q", claims.Role, "admin")
	}
}

func TestAccessTokenWrongSecretRejected(t *testing.T) {
	tok, _ := auth.GenerateAccessToken(testSecret, "user-123", "user")
	_, err := auth.ValidateAccessToken("wrong-secret", tok)
	if err == nil {
		t.Fatal("expected error for wrong secret, got nil")
	}
}

func TestAccessTokenInvalidStringRejected(t *testing.T) {
	_, err := auth.ValidateAccessToken(testSecret, "not.a.jwt")
	if err == nil {
		t.Fatal("expected error for garbage token")
	}
}

func TestHashTokenIsStable(t *testing.T) {
	h1 := auth.HashToken("mytoken")
	h2 := auth.HashToken("mytoken")
	if h1 != h2 {
		t.Error("hash not stable across calls")
	}
}

func TestHashTokenDifferentInputs(t *testing.T) {
	h1 := auth.HashToken("token-a")
	h2 := auth.HashToken("token-b")
	if h1 == h2 {
		t.Error("different inputs produced same hash")
	}
}

func TestGenerateRefreshTokenIsRandom(t *testing.T) {
	p1, h1, err := auth.GenerateRefreshToken()
	if err != nil {
		t.Fatal(err)
	}
	p2, h2, _ := auth.GenerateRefreshToken()
	if p1 == p2 {
		t.Error("refresh tokens are not random")
	}
	if h1 == h2 {
		t.Error("refresh token hashes are not random")
	}
}

func TestRefreshTokenHashMatchesPlaintext(t *testing.T) {
	plain, hash, _ := auth.GenerateRefreshToken()
	if auth.HashToken(plain) != hash {
		t.Error("hash of plaintext does not match returned hash")
	}
}

func TestRefreshTokenExpiry(t *testing.T) {
	exp := auth.RefreshTokenExpiry()
	// Should be ~30 days from now
	diff := time.Until(exp)
	if diff < 29*24*time.Hour || diff > 31*24*time.Hour {
		t.Errorf("unexpected expiry duration: %v", diff)
	}
}
