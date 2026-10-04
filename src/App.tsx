import { useEffect, useRef, useState } from 'react'
import { ArrowLeft, Settings as SettingsIcon } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import logo from '../assets/warply-logo.png'
import Settings, { type SettingsSection } from './components/Settings'
import ConnectionDetails from './components/ConnectionDetails'
import About from './components/About'
import PowerButton, { type PowerState } from './components/PowerButton'
import StatusBlock from './components/StatusBlock'
import LoadingScreen from './components/LoadingScreen'
import { usePreferences } from './hooks/usePreferences'
import { useTunnel } from './hooks/useTunnel'
import { localizeBackendMessage } from './i18n'

type View = 'main' | 'settings' | 'about'
type AvailableUpdate = { version: string }

export default function App() {
  const controls = useTunnel()
  const appearance = usePreferences()
  const { t } = appearance
  const { snapshot, busy, error, notice } = controls
  const [view, setView] = useState<View>('main')
  const [settingsSection, setSettingsSection] =
    useState<SettingsSection>('overview')
  const [aboutReturn, setAboutReturn] = useState<View>('main')
  const [linkError, setLinkError] = useState<string | null>(null)
  const [updateVersion, setUpdateVersion] = useState<string | null>(null)
  const [updateBusy, setUpdateBusy] = useState<
    'checking' | 'installing' | null
  >(null)
  const [updateMessage, setUpdateMessage] = useState<string | null>(null)
  const content = useRef<HTMLElement>(null)
  const settingsButton = useRef<HTMLButtonElement>(null)
  const backButton = useRef<HTMLButtonElement>(null)
  const previousView = useRef<View>('main')
  const scrollPositions = useRef<Record<View, number>>({
    main: 0,
    settings: 0,
    about: 0,
  })

  async function checkUpdates() {
    setUpdateBusy('checking')
    setUpdateMessage(null)
    try {
      const update = await invoke<AvailableUpdate | null>('check_for_update')
      setUpdateVersion(update?.version ?? null)
      if (!update) setUpdateMessage(t('upToDate'))
    } catch {
      setUpdateMessage(t('updateCheckFailed'))
    } finally {
      setUpdateBusy(null)
    }
  }

  async function installUpdate() {
    setUpdateBusy('installing')
    setUpdateMessage(null)
    try {
      const installed = await invoke<boolean>('install_update')
      if (!installed) setUpdateMessage(t('updateCancelled'))
    } catch {
      setUpdateMessage(t('updateInstallFailed'))
    } finally {
      setUpdateBusy(null)
    }
  }

  useEffect(() => {
    if (content.current)
      content.current.scrollTop =
        view === 'settings' ? 0 : scrollPositions.current[view]
    if (view !== 'main') backButton.current?.focus()
    else if (previousView.current !== 'main') settingsButton.current?.focus()
    previousView.current = view
    function handleKey(event: KeyboardEvent) {
      if (
        view !== 'main' &&
        (event.key === 'Escape' || (event.altKey && event.key === 'ArrowLeft'))
      ) {
        event.preventDefault()
        scrollPositions.current[view] = content.current?.scrollTop ?? 0
        if (view === 'settings' && settingsSection !== 'overview')
          setSettingsSection('overview')
        else setView(view === 'about' ? aboutReturn : 'main')
      }
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [view, aboutReturn, settingsSection])

  useEffect(() => {
    if (
      !snapshot?.settings.automatic_update_checks ||
      snapshot.setup !== 'ready'
    )
      return
    let stopped = false
    async function check() {
      try {
        const update = await invoke<AvailableUpdate | null>(
          'automatic_update_check',
        )
        if (!stopped && update) setUpdateVersion(update.version)
      } catch {
        /* Background failures stay quiet; manual checks show an error. */
      }
    }
    const timer = window.setTimeout(() => void check(), 5000)
    const interval = window.setInterval(() => void check(), 3600000)
    return () => {
      stopped = true
      window.clearTimeout(timer)
      window.clearInterval(interval)
    }
  }, [snapshot?.settings.automatic_update_checks, snapshot?.setup])

  function navigate(next: View) {
    scrollPositions.current[view] = content.current?.scrollTop ?? 0
    setView(next)
  }

  function openAbout() {
    setAboutReturn(view)
    navigate('about')
  }

  const setup = snapshot?.setup ?? 'starting'
  const setupBusy = [
    'starting',
    'creating_account',
    'installing_wireguard',
  ].includes(setup)
  const connected = snapshot?.status === 'connected'
  const serviceActive = connected || snapshot?.status === 'connecting'
  const verified =
    connected && snapshot?.health?.status === 'verified' && !controls.stale
  const verifying =
    connected &&
    (!snapshot?.health ||
      ['unknown', 'checking'].includes(snapshot.health.status))
  const connecting = busy === 'connect' || snapshot?.status === 'connecting'
  const message = error ?? snapshot?.setup_message
  const failed =
    !!message || setup === 'registration_error' || snapshot?.status === 'error'
  const powerState: PowerState =
    (setupBusy && !failed) || connecting || verifying
      ? 'connecting'
      : failed
        ? 'error'
        : verified
          ? 'connected'
          : connected
            ? 'unverified'
            : 'disconnected'
  const canToggle =
    !!snapshot &&
    !busy &&
    updateBusy !== 'installing' &&
    snapshot.wireguard_installed &&
    (serviceActive || (setup === 'ready' && snapshot.has_config && !connecting))
  let title = verified
    ? t('warpVerified')
    : verifying
      ? t('checkingConnection')
      : connected
        ? t('connectionUnverified')
        : t('disconnected')
  let secondary = verified
    ? t('verificationPassed')
    : verifying
      ? t('verificationPending')
      : connected
        ? t('verificationFailed')
        : t('ready')
  if (setupBusy && !failed) {
    title = t('starting')
    secondary =
      setup === 'creating_account'
        ? t('creatingAccount')
        : setup === 'installing_wireguard'
          ? t('installingWireGuard')
          : t('starting')
  } else if (connecting) {
    title = t('connecting')
    secondary = ''
  } else if (busy === 'disconnect') {
    title = t('disconnecting')
    secondary = ''
  } else if (failed) {
    title =
      setup === 'registration_error'
        ? t('setupFailed')
        : setup === 'wireguard_required'
          ? t('wireGuardRequired')
          : t('error')
    secondary = message ? localizeBackendMessage(message, t) : ''
  }
  const noticeText = notice?.startsWith('Config exported')
    ? t('exported')
    : notice?.startsWith('Config imported')
      ? t('imported')
      : notice

  if (view === 'main' && setupBusy && !failed) {
    return <LoadingScreen message={secondary} />
  }

  return (
    <div className="app-shell">
      <header className="app-header">
        {view === 'main' ? (
          <>
            <div className="app-brand">
              <img src={logo} className="brand-logo" alt="" />
              <h1>Warply</h1>
            </div>
            <button
              ref={settingsButton}
              type="button"
              className="icon-button"
              aria-label={t('openSettings')}
              onClick={() => {
                setSettingsSection('overview')
                navigate('settings')
              }}
            >
              <SettingsIcon size={20} strokeWidth={1.6} aria-hidden="true" />
            </button>
          </>
        ) : (
          <>
            <div className="view-heading">
              <button
                ref={backButton}
                type="button"
                className="icon-button"
                aria-label={t('back')}
                onClick={() =>
                  view === 'settings' && settingsSection !== 'overview'
                    ? setSettingsSection('overview')
                    : navigate(view === 'about' ? aboutReturn : 'main')
                }
              >
                <ArrowLeft size={20} strokeWidth={1.6} aria-hidden="true" />
              </button>
              <h1>
                {view === 'settings'
                  ? t(
                      settingsSection === 'overview'
                        ? 'settings'
                        : settingsSection === 'general'
                          ? 'generalMenu'
                          : settingsSection === 'network'
                            ? 'networkMenu'
                            : settingsSection === 'security'
                              ? 'securityMenu'
                              : settingsSection === 'advanced'
                                ? 'advancedMenu'
                                : 'appearance',
                    )
                  : t('updatesAbout')}
              </h1>
            </div>
            <img src={logo} className="brand-logo" alt="" />
          </>
        )}
      </header>

      <main
        key={view}
        ref={content}
        className={
          view === 'main' ? 'main-view view-enter' : 'detail-view view-enter'
        }
      >
        {view === 'main' ? (
          <div className="connection-content">
            <PowerButton
              state={powerState}
              pressed={serviceActive}
              disabled={!canToggle}
              label={serviceActive ? t('disconnect') : t('connect')}
              onClick={() => void controls.toggle()}
            />
            <StatusBlock
              key={title}
              title={title}
              secondary={secondary}
              error={failed && !connecting}
            />
            {snapshot?.protection?.active && (
              <div className="protection-notice" role="status">
                <p>{t('protectionActive')}</p>
                <button
                  type="button"
                  className="text-button"
                  disabled={busy === 'restore'}
                  onClick={() => void controls.restoreInternet()}
                >
                  {t('restoreInternet')}
                </button>
              </div>
            )}
            {updateVersion && (
              <button type="button" className="text-button" onClick={openAbout}>
                {t('updateAvailable')} {updateVersion}
              </button>
            )}
            {setup === 'ready' && (
              <ConnectionDetails
                snapshot={snapshot}
                uncertain={controls.stale}
                t={t}
              />
            )}
            {setup === 'ready' && (
              <button
                type="button"
                className="text-button connection-link"
                onClick={() => {
                  setSettingsSection('network')
                  navigate('settings')
                }}
              >
                {t('connectionDetailsLink')}
              </button>
            )}
            {setup === 'registration_error' && (
              <button
                type="button"
                className="native-button"
                disabled={!!busy}
                onClick={() => void controls.retrySetup()}
              >
                {t('retry')}
              </button>
            )}
            {setup === 'wireguard_required' && (
              <div className="recovery-actions">
                <button
                  type="button"
                  className="text-button"
                  onClick={() => void controls.openDownload()}
                >
                  {t('downloadWireGuard')}
                </button>
                <button
                  type="button"
                  className="native-button"
                  disabled={!!busy}
                  onClick={() => void controls.checkWireGuard()}
                >
                  {t('checkAgain')}
                </button>
              </div>
            )}
            {noticeText && (
              <p className="notice" role="status">
                {noticeText}
              </p>
            )}
          </div>
        ) : view === 'settings' ? (
          <>
            <Settings
              controls={controls}
              appearance={appearance}
              setupBusy={setupBusy || updateBusy === 'installing'}
              section={settingsSection}
              onSection={setSettingsSection}
              onAbout={openAbout}
            />
            {message && (
              <p className="view-error" role="alert">
                {localizeBackendMessage(message, t)}
              </p>
            )}
            {noticeText && (
              <p className="notice" role="status">
                {noticeText}
              </p>
            )}
          </>
        ) : (
          <>
            <About
              t={t}
              updateVersion={updateVersion}
              updateBusy={updateBusy}
              updateMessage={updateMessage}
              onCheckUpdates={() => void checkUpdates()}
              onInstallUpdate={() => void installUpdate()}
              onProjectLink={() => {
                void invoke('open_project_page').catch((cause) =>
                  setLinkError(String(cause)),
                )
              }}
            />
            {linkError && (
              <p className="view-error" role="alert">
                {linkError}
              </p>
            )}
          </>
        )}
      </main>

      <footer className="app-footer">
        <span>{t('unofficial')}</span>
        {view !== 'about' && (
          <button type="button" className="text-button" onClick={openAbout}>
            {t('about')}
          </button>
        )}
      </footer>
    </div>
  )
}
