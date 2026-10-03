import { useState } from 'react'
import Settings from './components/Settings'
import {
  useTunnel,
  type SetupStatus,
  type TunnelStatus,
} from './hooks/useTunnel'

const statusText: Record<TunnelStatus, string> = {
  disconnected: 'Disconnected',
  connecting: 'Connecting',
  connected: 'Connected',
  error: 'Error',
}
const setupText: Record<SetupStatus, string> = {
  starting: 'Getting ready…',
  creating_account: 'Creating WARP account…',
  installing_wireguard: 'Installing WireGuard…',
  ready: 'Ready',
  registration_error: 'Could not create the WARP account',
  wireguard_required: 'WireGuard is needed to connect',
}

export default function App() {
  const controls = useTunnel()
  const { snapshot, busy, error, notice } = controls
  const [settingsOpen, setSettingsOpen] = useState(false)
  const setup = snapshot?.setup ?? 'starting'
  const setupBusy = [
    'starting',
    'creating_account',
    'installing_wireguard',
  ].includes(setup)
  const connected = snapshot?.status === 'connected'
  const status =
    busy === 'connect' ? 'connecting' : (snapshot?.status ?? 'disconnected')
  const canToggle =
    !!snapshot &&
    !busy &&
    snapshot.wireguard_installed &&
    (connected ||
      (setup === 'ready' && snapshot.has_config && status !== 'connecting'))
  const message = error ?? snapshot?.setup_message

  return (
    <main className="flex min-h-screen flex-col bg-slate-50 px-7 py-7 text-slate-900 dark:bg-slate-950 dark:text-slate-100">
      <header className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div
            aria-hidden="true"
            className="grid size-10 place-items-center rounded-xl bg-cyan-700 text-lg font-bold text-white"
          >
            W
          </div>
          <h1 className="text-lg font-bold tracking-tight">Warply</h1>
        </div>
        <button
          type="button"
          onClick={() => setSettingsOpen(true)}
          aria-label="Open settings"
          className="rounded-xl p-3 text-slate-500 hover:bg-slate-200 dark:text-slate-400 dark:hover:bg-slate-800"
        >
          <svg
            aria-hidden="true"
            width="22"
            height="22"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.7"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="m9 3-.6 2.3-2 .9-2.1-.6-2 3.4 1.5 1.7v2.6l-1.5 1.7 2 3.4 2.1-.6 2 .9L9 21h4l.6-2.3 2-.9 2.1.6 2-3.4-1.5-1.7v-2.6l1.5-1.7-2-3.4-2.1.6-2-.9L13 3Z" />
            <circle cx="11" cy="12" r="3" />
          </svg>
        </button>
      </header>

      <section className="flex flex-1 flex-col items-center justify-center py-12 text-center">
        <p
          role="status"
          aria-live="polite"
          className="mb-7 text-sm font-semibold"
        >
          {setupBusy
            ? setupText[setup]
            : busy === 'disconnect'
              ? 'Disconnecting'
              : message
                ? 'Error'
                : statusText[status]}
        </p>
        <button
          type="button"
          onClick={() => void controls.toggle()}
          disabled={!canToggle}
          aria-label={connected ? 'Disconnect tunnel' : 'Connect tunnel'}
          aria-pressed={connected}
          className={`grid size-44 place-items-center rounded-full border-[10px] shadow-xl transition disabled:cursor-not-allowed disabled:opacity-50 ${connected ? 'border-emerald-200 bg-emerald-600 text-white shadow-emerald-500/20 hover:bg-emerald-700 dark:border-emerald-900' : 'border-cyan-100 bg-cyan-700 text-white shadow-cyan-500/20 hover:bg-cyan-800 dark:border-cyan-950'}`}
        >
          <svg
            aria-hidden="true"
            width="70"
            height="70"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.8"
            strokeLinecap="round"
          >
            <path d="M12 2v10" />
            <path d="M6.2 5.8a9 9 0 1 0 11.6 0" />
          </svg>
        </button>
        <p className="mt-7 text-2xl font-semibold tracking-tight">
          {connected ? 'ON' : 'OFF'}
        </p>
        <p
          role="status"
          aria-live="polite"
          className="mt-2 text-sm text-slate-500 dark:text-slate-400"
        >
          {setupBusy
            ? 'Warply is setting things up for you.'
            : setupText[setup]}
        </p>

        {message && (
          <p
            role="alert"
            className="mt-5 max-w-sm text-sm leading-6 text-red-600 dark:text-red-400"
          >
            {message}
          </p>
        )}
        {setup === 'registration_error' && (
          <button
            type="button"
            disabled={!!busy}
            onClick={() => void controls.retrySetup()}
            className="mt-4 rounded-xl bg-cyan-700 px-5 py-2.5 text-sm font-semibold text-white disabled:opacity-50"
          >
            Try again
          </button>
        )}
        {setup === 'wireguard_required' && (
          <div className="mt-4 flex flex-col items-center gap-3">
            <button
              type="button"
              onClick={() => void controls.openDownload()}
              className="text-sm text-cyan-700 underline dark:text-cyan-400"
            >
              Official WireGuard download page
            </button>
            <button
              type="button"
              disabled={!!busy}
              onClick={() => void controls.checkWireGuard()}
              className="rounded-xl bg-cyan-700 px-5 py-2.5 text-sm font-semibold text-white disabled:opacity-50"
            >
              Check again
            </button>
          </div>
        )}
        {notice && (
          <p
            role="status"
            className="mt-5 text-sm text-emerald-700 dark:text-emerald-400"
          >
            {notice}
          </p>
        )}
      </section>

      <footer className="pt-5 text-center text-xs text-slate-500 dark:text-slate-400">
        Unofficial, not affiliated with Cloudflare.
      </footer>
      {settingsOpen && (
        <Settings
          controls={controls}
          setupBusy={setupBusy}
          onClose={() => setSettingsOpen(false)}
        />
      )}
    </main>
  )
}
