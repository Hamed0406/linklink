package devices

import (
	"context"
	"encoding/binary"
	"fmt"
	"net"

	"github.com/jackc/pgx/v5/pgxpool"
)

// AllocateIP finds the lowest unused tunnel IP in the given CIDR range.
// The first address (.1) is reserved for the hub. Uses SELECT FOR UPDATE
// on the devices table to prevent concurrent allocation of the same IP.
func AllocateIP(ctx context.Context, db *pgxpool.Pool, networkCIDR string) (string, error) {
	_, ipNet, err := net.ParseCIDR(networkCIDR)
	if err != nil {
		return "", fmt.Errorf("invalid CIDR %q: %w", networkCIDR, err)
	}

	// Collect all IPs in range (skip network addr, broadcast addr, and .1 for hub)
	candidates := cidrHosts(ipNet)
	if len(candidates) == 0 {
		return "", fmt.Errorf("CIDR %q has no usable addresses", networkCIDR)
	}

	tx, err := db.Begin(ctx)
	if err != nil {
		return "", err
	}
	defer tx.Rollback(ctx)

	// Lock existing rows so concurrent allocations serialize
	rows, err := tx.Query(ctx,
		`SELECT tunnel_ip::text FROM devices WHERE status != 'revoked' FOR UPDATE`,
	)
	if err != nil {
		return "", fmt.Errorf("query existing IPs: %w", err)
	}
	used := make(map[string]bool)
	for rows.Next() {
		var ip string
		if err := rows.Scan(&ip); err == nil {
			used[ip] = true
		}
	}
	rows.Close()

	for _, ip := range candidates {
		if !used[ip] {
			if err := tx.Commit(ctx); err != nil {
				return "", err
			}
			return ip, nil
		}
	}
	tx.Commit(ctx)
	return "", fmt.Errorf("CIDR %q is exhausted — no free addresses", networkCIDR)
}

// cidrHosts returns all usable host IPs in a CIDR, skipping network,
// broadcast, and .1 (reserved for hub).
func cidrHosts(n *net.IPNet) []string {
	var hosts []string
	ip := cloneIP(n.IP.To4())
	if ip == nil {
		return nil
	}
	// Start from .2 (skip .0 network addr, .1 hub)
	inc(ip)
	inc(ip)
	for n.Contains(ip) {
		// Skip broadcast (all host bits = 1)
		broadcast := broadcastIP(n)
		if ip.Equal(broadcast) {
			break
		}
		hosts = append(hosts, ip.String())
		inc(ip)
	}
	return hosts
}

func cloneIP(ip net.IP) net.IP {
	c := make(net.IP, len(ip))
	copy(c, ip)
	return c
}

func inc(ip net.IP) {
	for i := len(ip) - 1; i >= 0; i-- {
		ip[i]++
		if ip[i] != 0 {
			break
		}
	}
}

func broadcastIP(n *net.IPNet) net.IP {
	ip := n.IP.To4()
	mask := n.Mask
	b := make(net.IP, 4)
	for i := 0; i < 4; i++ {
		b[i] = ip[i] | ^mask[i]
	}
	return b
}

// ipToUint32 converts an IPv4 address to uint32.
func ipToUint32(ip net.IP) uint32 {
	ip = ip.To4()
	return binary.BigEndian.Uint32(ip)
}
