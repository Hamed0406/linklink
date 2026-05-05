package devices

import (
	"net"
	"testing"
)

func TestCidrHosts_SlashThirty(t *testing.T) {
	_, ipNet, _ := net.ParseCIDR("10.44.0.0/30")
	hosts := cidrHosts(ipNet)
	// /30: 4 addresses total — .0 network, .1 hub, .2 usable, .3 broadcast
	if len(hosts) != 1 {
		t.Fatalf("expected 1 usable host in /30, got %d: %v", len(hosts), hosts)
	}
	if hosts[0] != "10.44.0.2" {
		t.Errorf("expected 10.44.0.2, got %s", hosts[0])
	}
}

func TestCidrHosts_SlashTwentyFour(t *testing.T) {
	_, ipNet, _ := net.ParseCIDR("10.44.0.0/24")
	hosts := cidrHosts(ipNet)
	// /24: 256 addresses — .0 network, .1 hub, .2–.254 usable, .255 broadcast = 253 hosts
	if len(hosts) != 253 {
		t.Fatalf("expected 253 usable hosts in /24, got %d", len(hosts))
	}
	if hosts[0] != "10.44.0.2" {
		t.Errorf("first host should be 10.44.0.2, got %s", hosts[0])
	}
	if hosts[len(hosts)-1] != "10.44.0.254" {
		t.Errorf("last host should be 10.44.0.254, got %s", hosts[len(hosts)-1])
	}
}

func TestCidrHosts_HubNotIncluded(t *testing.T) {
	_, ipNet, _ := net.ParseCIDR("10.44.0.0/24")
	hosts := cidrHosts(ipNet)
	for _, h := range hosts {
		if h == "10.44.0.1" {
			t.Error("hub IP 10.44.0.1 must not appear in usable hosts")
		}
		if h == "10.44.0.0" {
			t.Error("network addr must not appear in usable hosts")
		}
		if h == "10.44.0.255" {
			t.Error("broadcast addr must not appear in usable hosts")
		}
	}
}
