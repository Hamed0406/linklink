import { useEffect, useState } from 'react'
import { auditLogs, AuditLog } from '../api/client'

export default function AuditLogs() {
  const [logs, setLogs]       = useState<AuditLog[]>([])
  const [loading, setLoading] = useState(true)
  const [filter, setFilter]   = useState('')

  async function load(action?: string) {
    setLoading(true)
    try {
      const data = await auditLogs.list(action || undefined)
      setLogs(data)
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => { load() }, [])

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold text-gray-900">Audit Logs</h1>
        <div className="flex gap-2">
          <input
            value={filter}
            onChange={e => setFilter(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && load(filter)}
            placeholder="Filter by action…"
            className="px-3 py-1.5 border border-gray-300 rounded text-sm w-52"
          />
          <button
            onClick={() => load(filter)}
            className="px-3 py-1.5 bg-gray-800 text-white text-sm rounded hover:bg-gray-700"
          >
            Filter
          </button>
          <button
            onClick={() => { setFilter(''); load() }}
            className="px-3 py-1.5 border border-gray-300 text-sm rounded hover:bg-gray-50"
          >
            Clear
          </button>
        </div>
      </div>

      {loading ? (
        <p className="text-gray-400">Loading…</p>
      ) : logs.length === 0 ? (
        <p className="text-gray-500 text-sm">No audit log entries.</p>
      ) : (
        <div className="bg-white rounded-xl shadow overflow-hidden">
          <table className="w-full text-sm">
            <thead className="bg-gray-50 text-gray-500 uppercase text-xs">
              <tr>
                {['Time', 'Action', 'Actor', 'Target'].map(h => (
                  <th key={h} className="px-4 py-3 text-left">{h}</th>
                ))}
              </tr>
            </thead>
            <tbody className="divide-y divide-gray-100">
              {logs.map(entry => (
                <tr key={entry.id} className="hover:bg-gray-50">
                  <td className="px-4 py-2 text-gray-500 whitespace-nowrap">
                    {new Date(entry.created_at).toLocaleString()}
                  </td>
                  <td className="px-4 py-2">
                    <span className="font-mono text-xs bg-gray-100 px-2 py-0.5 rounded">
                      {entry.action}
                    </span>
                  </td>
                  <td className="px-4 py-2 text-gray-500 text-xs font-mono">
                    {entry.actor_user_id ? entry.actor_user_id.slice(0, 8) + '…' : 'system'}
                  </td>
                  <td className="px-4 py-2 text-gray-500 text-xs font-mono">
                    {entry.target_type && entry.target_id
                      ? `${entry.target_type}:${entry.target_id.slice(0, 8)}…`
                      : '—'}
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
