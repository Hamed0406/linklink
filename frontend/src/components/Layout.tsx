import { Outlet, NavLink, useNavigate } from 'react-router-dom'
import { auth } from '../api/client'

const nav = [
  { to: '/',           label: 'Dashboard' },
  { to: '/devices',    label: 'Devices'   },
  { to: '/audit-logs', label: 'Audit Logs'},
]

export default function Layout() {
  const navigate = useNavigate()

  async function handleLogout() {
    await auth.logout()
    navigate('/login')
  }

  return (
    <div className="min-h-screen flex bg-gray-50">
      {/* Sidebar */}
      <nav className="w-56 bg-gray-900 text-white flex flex-col">
        <div className="px-6 py-5 text-xl font-bold tracking-tight border-b border-gray-700">
          linklink
        </div>
        <ul className="flex-1 py-4 space-y-1">
          {nav.map(({ to, label }) => (
            <li key={to}>
              <NavLink
                to={to}
                end={to === '/'}
                className={({ isActive }) =>
                  `block px-6 py-2 text-sm rounded-r-lg transition-colors ${
                    isActive
                      ? 'bg-indigo-600 text-white'
                      : 'text-gray-300 hover:bg-gray-800'
                  }`
                }
              >
                {label}
              </NavLink>
            </li>
          ))}
        </ul>
        <button
          onClick={handleLogout}
          className="m-4 px-4 py-2 text-sm text-gray-400 hover:text-white border border-gray-700 rounded"
        >
          Logout
        </button>
      </nav>

      {/* Main content */}
      <main className="flex-1 p-8 overflow-auto">
        <Outlet />
      </main>
    </div>
  )
}
