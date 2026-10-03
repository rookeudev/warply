import { useEffect, useRef } from 'react'
import type { TunnelControls } from '../hooks/useTunnel'

type Props = {
  controls: TunnelControls
  setupBusy: boolean
  onClose: () => void
}

export default function Settings({ controls, setupBusy, onClose }: Props) {
  const dialog = useRef<HTMLDialogElement>(null)
  const { snapshot, busy, error, notice } = controls
  const locked = !!busy || setupBusy
  const disconnected = snapshot?.status === 'disconnected'

  useEffect(() => {
    dialog.current?.showModal()
  }, [])

  return (
    <dialog
      ref={dialog}
      onCancel={onClose}
      aria-labelledby="settings-title"
      className="m-auto w-[calc(100%-2rem)] max-w-md rounded-2xl border border-slate-200 bg-white p-6 text-slate-900 shadow-2xl backdrop:bg-slate-950/60 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100"
    >
      <div className="flex items-center justify-between">
        <h2 id="settings-title" className="text-lg font-semibold">
          Settings
        </h2>
        <button
          type="button"
          onClick={onClose}
          aria-label="Close settings"
          className="rounded-lg px-3 py-2 hover:bg-slate-100 dark:hover:bg-slate-800"
        >
          ✕
        </button>
      </div>

      <label className="mt-6 flex items-center gap-3 text-sm">
        <input
          type="checkbox"
          checked={snapshot?.auto_connect ?? false}
          disabled={locked || !snapshot}
          onChange={(event) =>
            void controls.setAutoConnect(event.target.checked)
          }
          className="size-4 accent-cyan-700"
        />
        Connect automatically on launch
      </label>

      <details className="mt-6 border-t border-slate-200 pt-5 dark:border-slate-700">
        <summary className="cursor-pointer text-sm font-semibold">
          Advanced
        </summary>
        <div className="mt-5 space-y-5 text-sm">
          <div>
            <p className="text-slate-500 dark:text-slate-400">
              Use your own config if automatic WARP registration is unavailable.
              Disconnect first.
            </p>
            <button
              type="button"
              disabled={locked || !disconnected}
              onClick={() => void controls.importConfig()}
              className="mt-2 rounded-lg border border-slate-300 px-3 py-2 disabled:opacity-40 dark:border-slate-600"
            >
              {busy === 'import' ? 'Importing…' : 'Import .conf'}
            </button>
          </div>
          <div>
            <p className="text-slate-500 dark:text-slate-400">
              Replace the saved config with a new WARP account and new local
              keys. Disconnect first.
            </p>
            <button
              type="button"
              disabled={locked || !disconnected}
              onClick={() => void controls.resetAccount()}
              className="mt-2 rounded-lg border border-slate-300 px-3 py-2 disabled:opacity-40 dark:border-slate-600"
            >
              {busy === 'reset'
                ? 'Creating account…'
                : 'Reset / create new WARP account'}
            </button>
          </div>
          <div>
            <p className="text-slate-500 dark:text-slate-400">
              Exported files contain your private key. Anyone with the file can
              use your tunnel.
            </p>
            <button
              type="button"
              disabled={locked || !snapshot?.has_config}
              onClick={() => void controls.exportConfig()}
              className="mt-2 rounded-lg border border-slate-300 px-3 py-2 disabled:opacity-40 dark:border-slate-600"
            >
              {busy === 'export' ? 'Exporting…' : 'Export config'}
            </button>
          </div>
        </div>
      </details>
      {error && (
        <p role="alert" className="mt-5 text-sm text-red-600 dark:text-red-400">
          {error}
        </p>
      )}
      {notice && (
        <p
          role="status"
          className="mt-5 text-sm text-emerald-700 dark:text-emerald-400"
        >
          {notice}
        </p>
      )}
    </dialog>
  )
}
