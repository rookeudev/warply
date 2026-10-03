import { useEffect, useRef, useState } from 'react'
import { ArrowLeft, Settings as SettingsIcon } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import logo from '../assets/warply-logo.png'
import Settings from './components/Settings'
import About from './components/About'
import PowerButton, { type PowerState } from './components/PowerButton'
import StatusBlock from './components/StatusBlock'
import { usePreferences } from './hooks/usePreferences'
import { useTunnel } from './hooks/useTunnel'
import { localizeBackendMessage } from './i18n'

type View = 'main' | 'settings' | 'about'

export default function App() {
  const controls = useTunnel()
  const appearance = usePreferences()
  const { t } = appearance
  const { snapshot, busy, error, notice } = controls
  const [view, setView] = useState<View>('main')
  const [aboutReturn, setAboutReturn] = useState<View>('main')
  const [linkError, setLinkError] = useState<string | null>(null)
  const content = useRef<HTMLElement>(null)
  const settingsButton = useRef<HTMLButtonElement>(null)
  const backButton = useRef<HTMLButtonElement>(null)
  const previousView = useRef<View>('main')
  const scrollPositions = useRef<Record<View, number>>({
    main: 0,
    settings: 0,
    about: 0,
  })

  useEffect(() => {
    if (content.current)
      content.current.scrollTop = scrollPositions.current[view]
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
        setView(view === 'about' ? aboutReturn : 'main')
      }
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [view, aboutReturn])

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
  const connecting = busy === 'connect' || snapshot?.status === 'connecting'
  const message = error ?? snapshot?.setup_message
  const failed =
    !!message || setup === 'registration_error' || snapshot?.status === 'error'
  const powerState: PowerState =
    setupBusy || connecting
      ? 'connecting'
      : failed
        ? 'error'
        : connected
          ? 'connected'
          : 'disconnected'
  const canToggle =
    !!snapshot &&
    !busy &&
    snapshot.wireguard_installed &&
    (connected || (setup === 'ready' && snapshot.has_config && !connecting))
  let title = connected ? t('connected') : t('disconnected')
  let secondary = connected ? t('tunnelRunning') : t('ready')
  if (setupBusy) {
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
              onClick={() => navigate('settings')}
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
                  navigate(view === 'about' ? aboutReturn : 'main')
                }
              >
                <ArrowLeft size={20} strokeWidth={1.6} aria-hidden="true" />
              </button>
              <h1>{view === 'settings' ? t('settings') : t('about')}</h1>
            </div>
            <img src={logo} className="brand-logo" alt="" />
          </>
        )}
      </header>

      <main
        ref={content}
        className={view === 'main' ? 'main-view' : 'detail-view'}
      >
        {view === 'main' ? (
          <div className="connection-content">
            <PowerButton
              state={powerState}
              pressed={connected}
              disabled={!canToggle}
              label={connected ? t('disconnect') : t('connect')}
              onClick={() => void controls.toggle()}
            />
            <StatusBlock
              title={title}
              secondary={secondary}
              error={failed && !setupBusy && !connecting}
            />
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
              setupBusy={setupBusy}
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
