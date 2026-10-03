import { useCallback, useEffect, useRef, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'

export type TunnelStatus = 'disconnected' | 'connecting' | 'connected' | 'error'
export type SetupStatus =
  | 'starting'
  | 'creating_account'
  | 'installing_wireguard'
  | 'ready'
  | 'registration_error'
  | 'wireguard_required'

export type TunnelSnapshot = {
  status: TunnelStatus
  has_config: boolean
  wireguard_installed: boolean
  setup: SetupStatus
  setup_message: string | null
  auto_connect: boolean
}

export function useTunnel() {
  const [snapshot, setSnapshot] = useState<TunnelSnapshot | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [pollError, setPollError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const busyRef = useRef(false)
  const requestId = useRef(0)
  const appliedId = useRef(0)

  const applySnapshot = useCallback((next: TunnelSnapshot, id: number) => {
    if (id >= appliedId.current) {
      appliedId.current = id
      setSnapshot(next)
    }
  }, [])

  const refresh = useCallback(async () => {
    const id = ++requestId.current
    try {
      const next = await invoke<TunnelSnapshot>('tunnel_snapshot')
      applySnapshot(next, id)
      setPollError(null)
    } catch (cause) {
      setPollError(String(cause))
    }
  }, [applySnapshot])

  useEffect(() => {
    void refresh()
    const timer = window.setInterval(() => void refresh(), 1000)
    return () => window.clearInterval(timer)
  }, [refresh])

  async function run(
    command: string,
    activity: string,
    args?: Record<string, unknown>,
  ) {
    if (busyRef.current) return null
    busyRef.current = true
    setBusy(activity)
    setError(null)
    setNotice(null)
    try {
      const result = await invoke<TunnelSnapshot | boolean | null>(
        command,
        args,
      )
      if (result && typeof result === 'object') {
        applySnapshot(result, ++requestId.current)
      }
      return result
    } catch (cause) {
      setError(String(cause))
      return null
    } finally {
      busyRef.current = false
      setBusy(null)
      void refresh()
    }
  }

  async function importConfig() {
    const result = await run('import_config', 'import')
    if (result) setNotice('Config imported securely.')
  }

  async function exportConfig() {
    const result = await run('export_config', 'export')
    if (result === true) setNotice('Config exported. Keep the file private.')
  }

  async function toggle() {
    if (!snapshot) return
    const connected = snapshot.status === 'connected'
    await run(
      connected ? 'disconnect_tunnel' : 'connect_tunnel',
      connected ? 'disconnect' : 'connect',
    )
  }

  return {
    snapshot,
    busy,
    error: error ?? pollError,
    notice,
    importConfig,
    exportConfig,
    toggle,
    retrySetup: () => run('retry_setup', 'setup'),
    checkWireGuard: () => run('check_wireguard', 'check'),
    resetAccount: () => run('reset_account', 'reset'),
    setAutoConnect: (enabled: boolean) =>
      run('set_auto_connect', 'settings', { enabled }),
    openDownload: async () => {
      try {
        await invoke('open_wireguard_download')
      } catch (cause) {
        setError(String(cause))
      }
    },
  }
}

export type TunnelControls = ReturnType<typeof useTunnel>
