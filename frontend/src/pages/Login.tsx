import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { auth } from '../api/client'

export default function Login() {
  const navigate = useNavigate()
  const [step, setStep] = useState<'idle' | 'polling' | 'error'>('idle')
  const [userCode, setUserCode] = useState('')
  const [verifyUri, setVerifyUri] = useState('')
  const [errorMsg, setErrorMsg] = useState('')

  async function startFlow() {
    setStep('polling')
    setErrorMsg('')
    try {
      const flow = await auth.startDeviceFlow()
      setUserCode(flow.user_code)
      setVerifyUri(flow.verification_uri)

      // Poll for token
      const intervalMs = (flow.interval ?? 5) * 1000
      const deadline = Date.now() + flow.expires_in * 1000

      while (Date.now() < deadline) {
        await sleep(intervalMs)
        try {
          const tokens = await auth.pollToken(flow.device_code)
          auth.saveTokens(tokens.access_token, tokens.refresh_token)
          navigate('/')
          return
        } catch (e: any) {
          if (!e.message?.includes('authorization_pending')) {
            throw e
          }
        }
      }
      throw new Error('Login timed out. Please try again.')
    } catch (e: any) {
      setErrorMsg(e.message ?? 'Login failed')
      setStep('error')
    }
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gray-50">
      <div className="bg-white rounded-xl shadow p-10 w-full max-w-md text-center space-y-6">
        <h1 className="text-3xl font-bold text-gray-900">linklink</h1>
        <p className="text-gray-500 text-sm">Secure mesh tunnel</p>

        {step === 'idle' && (
          <button
            onClick={startFlow}
            className="w-full py-3 bg-indigo-600 text-white rounded-lg font-medium hover:bg-indigo-700 transition"
          >
            Sign in
          </button>
        )}

        {step === 'polling' && userCode && (
          <div className="space-y-4">
            <p className="text-sm text-gray-600">
              Visit the link below and enter the code:
            </p>
            <a
              href={verifyUri}
              target="_blank"
              rel="noreferrer"
              className="text-indigo-600 underline text-sm"
            >
              {verifyUri}
            </a>
            <div className="text-4xl font-mono font-bold tracking-widest text-gray-900 py-4 bg-gray-50 rounded-lg">
              {userCode}
            </div>
            <p className="text-xs text-gray-400 animate-pulse">Waiting for authorization…</p>
          </div>
        )}

        {step === 'polling' && !userCode && (
          <p className="text-sm text-gray-500 animate-pulse">Starting login…</p>
        )}

        {step === 'error' && (
          <div className="space-y-4">
            <p className="text-red-600 text-sm">{errorMsg}</p>
            <button
              onClick={() => setStep('idle')}
              className="w-full py-2 border border-gray-300 rounded-lg text-sm hover:bg-gray-50"
            >
              Try again
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

function sleep(ms: number) {
  return new Promise(r => setTimeout(r, ms))
}
