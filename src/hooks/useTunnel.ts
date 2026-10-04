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
  health: {
    status: 'unknown' | 'checking' | 'verified' | 'not_warp' | 'unavailable'
    duration_ms: number | null
    checked_ago_secs: number | null
    ipv4?: string
    ipv6?: string
    network?: {
      ipv4_tunnel: boolean
      ipv6_tunnel: boolean
      dns_matches: boolean
      other_vpn_count: number
      inspection_available: boolean
    }
  }
  protection?: { active: boolean; known?: boolean }
  has_config: boolean
  wireguard_installed: boolean
  setup: SetupStatus
  setup_message: string | null
  auto_connect: boolean
  poll_after_ms?: number
  settings: {
    kill_switch?: boolean
    notifications?: boolean
    automatic_update_checks?: boolean
    start_with_windows: boolean
    start_minimized: boolean
    close_to_tray: boolean
    dns: string
    custom_dns: string
    endpoint: string
    language: string
  }
}

export function useTunnel() {
  const [snapshot, setSnapshot] = useState<TunnelSnapshot | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [pollError, setPollError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const busyRef = useRef(false)
  const restorePending = useRef(false)
  const requestId = useRef(0)
  const appliedId = useRef(0)
  const pollDelay = useRef(3000)

  const applySnapshot = useCallback((next: TunnelSnapshot, id: number) => {
    if (id >= appliedId.current) {
      appliedId.current = id
      pollDelay.current = next.poll_after_ms === 15000 ? 15000 : 3000
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
    let stopped = false
    let timer: number
    async function poll() {
      await refresh()
      if (!stopped)
        timer = window.setTimeout(
          () => void poll(),
          document.hidden ? 15000 : pollDelay.current,
        )
    }
    timer = window.setTimeout(() => void poll(), 3000)
    const visible = () => {
      if (!document.hidden) void refresh()
    }
    document.addEventListener('visibilitychange', visible)
    return () => {
      stopped = true
      window.clearTimeout(timer)
      document.removeEventListener('visibilitychange', visible)
    }
  }, [refresh])

  async function run(
    command: string,
    activity: string,
    args?: Record<string, unknown>,
  ) {
    if (busyRef.current || restorePending.current) return null
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
      if (!restorePending.current) setBusy(null)
      void refresh()
    }
  }

  async function interrupt(command: 'restore_internet' | 'disconnect_tunnel') {
    if (restorePending.current) return
    restorePending.current = true
    const previousActivity = busy
    setBusy(command === 'restore_internet' ? 'restore' : 'disconnect')
    setError(null)
    try {
      const next = await invoke<TunnelSnapshot>(command)
      applySnapshot(next, ++requestId.current)
      setError(null)
    } catch (cause) {
      setError(String(cause))
    } finally {
      restorePending.current = false
      setBusy(busyRef.current ? previousActivity : null)
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
    const connected =
      snapshot.status === 'connected' || snapshot.status === 'connecting'
    await run(
      connected ? 'disconnect_tunnel' : 'connect_tunnel',
      connected ? 'disconnect' : 'connect',
    )
  }

  return {
    snapshot,
    busy,
    error: error ?? pollError,
    stale: pollError !== null,
    notice,
    importConfig,
    exportConfig,
    toggle,
    restoreInternet: () => interrupt('restore_internet'),
    cancelConnection: () => interrupt('disconnect_tunnel'),
    retrySetup: () => run('retry_setup', 'setup'),
    checkWireGuard: () => run('check_wireguard', 'check'),
    recheckConnection: () => run('recheck_connection', 'health'),
    resetAccount: () => run('reset_account', 'reset'),
    setAutoConnect: (enabled: boolean) =>
      run('set_auto_connect', 'settings', { enabled }),
    setGeneral: (
      name:
        | 'start_with_windows'
        | 'start_minimized'
        | 'close_to_tray'
        | 'kill_switch'
        | 'notifications'
        | 'automatic_update_checks',
      enabled: boolean,
    ) => run('set_general_setting', 'settings', { name, enabled }),
    setNetwork: (dns: string, customDns: string, endpoint: string) =>
      run('set_network_settings', 'settings', { dns, customDns, endpoint }),
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
