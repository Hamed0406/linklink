import { useEffect, useState } from 'react'
import { devices as api, Device } from '../api/client'

const STATUS_COLORS: Record<string, string> = {
  approved: 'bg-green-100 text-green-800',
  pending:  'bg-yellow-100 text-yellow-800',
  revoked:  'bg-red-100 text-red-800',
  disabled: 'bg-gray-100 text-gray-600',
}

export default function Devices() {
  const [list, setList]     = useState<Device[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError]   = useState('')

  async function load() {
    try {
      const data = await api.list()
      setList(data)
    } catch (e: any) {
      setError(e.message)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  async function handleApprove(id: string) {
    if (!confirm('Approve this device?')) return
    await api.approve(id)
    load()
  }

  async function handleRevoke(id: string) {
    if (!confirm('Revoke access for this device? This cannot be undone.')) return
    await api.revoke(id)
    load()
  }

  if (loading) return <p className="text-gray-400">Loading…</p>
  if (error)   return <p className="text-red-600">{error}</p>

  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-bold text-gray-900">Devices</h1>

      {list.length === 0 ? (
        <p className="text-gray-500 text-sm">No devices registered yet.</p>
      ) : (
        <div className="bg-white rounded-xl shadow overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 text-gray-500 uppercase text-xs">
              <tr>
                {['Name', 'OS', 'Tunnel IP', 'Status', 'Last seen', 'Actions'].map(h => (
                  <th key={h} className="px-4 py-3 text-left">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {list.map(d => (
                <tr key={d.id} className="hover:bg-gray-50">
                  <td className="px-4 py-3 font-medium text-gray-900">{d.name}</td>
                  <td className="px-4 py-3 text-gray-500">{d.os ?? '—'}</td>
                  <td className="px-4 py-3 font-mono text-gray-700">{d.tunnel_ip}</td>
                  <td className="px-4 py-3">
                    <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${STATUS_COLORS[d.status] ?? ''}`}>
                      {d.status}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-gray-500">
                    {d.last_seen_at ? relativeTime(d.last_seen_at) : 'never'}
                  </td>
                  <td className="px-4 py-3 space-x-2">
                    {d.status === 'pending' && (
                      <button
                        onClick={() => handleApprove(d.id)}
                        className="px-3 py-1 text-xs bg-green-600 text-white rounded hover:bg-green-700"
                      >
                        Approve
                      </button>
                    )}
                    {d.status === 'approved' && (
                      <button
                        onClick={() => handleRevoke(d.id)}
                        className="px-3 py-1 text-xs bg-red-600 text-white rounded hover:bg-red-700"
                      >
                        Revoke
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}

function relativeTime(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime()
  const s = Math.floor(diff / 1000)
  if (s < 60)   return `${s}s ago`
  if (s < 3600) return `${Math.floor(s / 60)}m ago`
  return `${Math.floor(s / 3600)}h ago`
}
