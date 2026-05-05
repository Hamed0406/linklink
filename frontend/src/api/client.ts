const BASE = '/api/v1'

function getToken(): string | null {
  return localStorage.getItem('access_token')
}

function setTokens(access: string, refresh: string) {
  localStorage.setItem('access_token', access)
  localStorage.setItem('refresh_token', refresh)
}

function clearTokens() {
  localStorage.removeItem('access_token')
  localStorage.removeItem('refresh_token')
}

async function request<T>(path: string, options: RequestInit = {}): Promise<T> {
  const token = getToken()
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    ...(options.headers as Record<string, string> ?? {}),
  }
  if (token) headers['Authorization'] = `Bearer ${token}`

  const res = await fetch(`${BASE}${path}`, { ...options, headers })

  if (res.status === 401) {
    clearTokens()
    window.location.href = '/login'
    throw new Error('Unauthorized')
  }
  if (!res.ok) {
    const body = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(body.error ?? res.statusText)
  }
  if (res.status === 204) return undefined as T
  return res.json()
}

// ── Auth ──────────────────────────────────────────────────────────────────────

export interface DeviceFlowStart {
  device_code: string
  user_code: string
  verification_uri: string
  expires_in: number
  interval: number
}

export interface TokenResponse {
  access_token: string
  refresh_token: string
  token_type: string
  expires_in: number
}

export const auth = {
  startDeviceFlow: () =>
    request<DeviceFlowStart>('/auth/device', { method: 'POST', body: '{}' }),

  pollToken: (deviceCode: string): Promise<TokenResponse> =>
    request('/auth/token', {
      method: 'POST',
      body: JSON.stringify({
        grant_type: 'urn:ietf:params:oauth:grant-type:device_code',
        device_code: deviceCode,
      }),
    }),

  saveTokens: setTokens,
  clearTokens,

  me: () => request<{ user_id: string; role: string }>('/auth/me'),

  logout: async () => {
    const refresh = localStorage.getItem('refresh_token')
    if (refresh) {
      await request('/auth/logout', { method: 'POST', body: JSON.stringify({ refresh_token: refresh }) })
        .catch(() => {})
    }
    clearTokens()
  },
}

// ── Devices ───────────────────────────────────────────────────────────────────

export interface Device {
  id: string
  user_id: string
  name: string
  os?: string
  hostname?: string
  public_key: string
  tunnel_ip: string
  external_endpoint?: string
  status: 'pending' | 'approved' | 'revoked' | 'disabled'
  is_relay: boolean
  config_version: number
  last_seen_at?: string
  created_at: string
}

export const devices = {
  list: () => request<Device[]>('/devices'),
  get: (id: string) => request<Device>(`/devices/${id}`),
  register: (body: { name: string; public_key: string; os?: string; hostname?: string }) =>
    request<Device>('/devices/register', { method: 'POST', body: JSON.stringify(body) }),
  approve: (id: string) =>
    request<void>(`/devices/${id}/approve`, { method: 'POST' }),
  revoke: (id: string) =>
    request<void>(`/devices/${id}/revoke`, { method: 'POST' }),
  delete: (id: string) =>
    request<void>(`/devices/${id}`, { method: 'DELETE' }),
}

// ── Audit Logs ────────────────────────────────────────────────────────────────

export interface AuditLog {
  id: string
  actor_user_id?: string
  action: string
  target_type?: string
  target_id?: string
  metadata?: Record<string, unknown>
  created_at: string
}

export const auditLogs = {
  list: (action?: string) =>
    request<AuditLog[]>(`/audit-logs${action ? `?action=${action}` : ''}`),
}
