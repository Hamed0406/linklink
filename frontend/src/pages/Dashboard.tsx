import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { devices, Device } from '../api/client'

export default function Dashboard() {
  const [devList, setDevList] = useState<Device[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    devices.list().then(setDevList).finally(() => setLoading(false))
  }, [])

  const online  = devList.filter(d => isOnline(d)).length
  const pending = devList.filter(d => d.status === 'pending').length
  const total   = devList.filter(d => d.status !== 'revoked').length

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold text-gray-900">Dashboard</h1>

      {loading ? (
        <p className="text-gray-400">Loading…</p>
      ) : (
        <>
          <div className="grid grid-cols-3 gap-4">
            <StatCard label="Total devices" value={total} />
            <StatCard label="Online now"    value={online}  color="text-green-600" />
            <StatCard label="Pending approval" value={pending} color="text-yellow-600" />
          </div>

          {pending > 0 && (
            <div className="bg-yellow-50 border border-yellow-200 rounded-lg p-4 text-sm text-yellow-800">
              {pending} device{pending > 1 ? 's' : ''} waiting for approval.{' '}
              <Link to="/devices" className="underline font-medium">Review now</Link>
            </div>
          )}
        </>
      )}
    </div>
  )
}

function StatCard({ label, value, color = 'text-gray-900' }: { label: string; value: number; color?: string }) {
  return (
    <div className="bg-white rounded-xl shadow p-6 space-y-1">
      <p className="text-sm text-gray-500">{label}</p>
      <p className={`text-3xl font-bold ${color}`}>{value}</p>
    </div>
  )
}

function isOnline(d: Device): boolean {
  if (d.status !== 'approved' || !d.last_seen_at) return false
  return Date.now() - new Date(d.last_seen_at).getTime() < 3 * 60 * 1000
}
