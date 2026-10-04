import {
  ChevronRight,
  Download,
  Info,
  Monitor,
  Palette,
  RotateCw,
  Shield,
  SlidersHorizontal,
  Upload,
} from 'lucide-react'
import type { TunnelControls } from '../hooks/useTunnel'
import type {
  AppearanceControls,
  ThemePreference,
} from '../hooks/usePreferences'
import type { Language } from '../i18n'
import SettingRow from './SettingRow'
import SettingsList from './SettingsList'
import Toggle from './Toggle'
import NetworkSettings from './NetworkSettings'
import ConnectionDetails from './ConnectionDetails'
import Diagnostics from './Diagnostics'

export type SettingsSection =
  'overview' | 'general' | 'network' | 'security' | 'appearance' | 'advanced'
type Props = {
  controls: TunnelControls
  appearance: AppearanceControls
  setupBusy: boolean
  section: SettingsSection
  onSection: (section: SettingsSection) => void
  onAbout: () => void
}

export default function Settings({
  controls,
  appearance,
  setupBusy,
  section,
  onSection,
  onAbout,
}: Props) {
  const { snapshot, busy } = controls
  const { t } = appearance
  const locked = !!busy || setupBusy
  const disconnected = snapshot?.status === 'disconnected'
  const menu = [
    { id: 'security', title: t('securityMenu'), description: t('securityDescription'), Icon: Shield },
    {
      id: 'general',
      title: t('generalMenu'),
      description: t('generalDescription'),
      Icon: Monitor,
    },
    {
      id: 'network',
      title: t('networkMenu'),
      description: t('networkMenuDescription'),
      Icon: SlidersHorizontal,
    },
    {
      id: 'appearance',
      title: t('appearance'),
      description: t('appearanceDescription'),
      Icon: Palette,
    },
    {
      id: 'advanced',
      title: t('advancedMenu'),
      description: t('advancedDescription'),
      Icon: Shield,
    },
  ] as const
  return (
    <div className="settings-content">
      {section === 'overview' && (
        <>
          <p className="page-intro">{t('settingsIntro')}</p>
          <nav className="settings-menu" aria-label={t('settings')}>
            {menu.map(({ id, title, description, Icon }) => (
              <button
                key={id}
                type="button"
                className="menu-card"
                onClick={() => onSection(id)}
              >
                <span className="menu-icon">
                  <Icon size={19} strokeWidth={1.6} aria-hidden="true" />
                </span>
                <span className="setting-copy">
                  <span className="setting-label">{title}</span>
                  <span className="setting-description">{description}</span>
                </span>
                <ChevronRight size={16} aria-hidden="true" />
              </button>
            ))}
            <button type="button" className="menu-card" onClick={onAbout}>
              <span className="menu-icon">
                <Info size={19} strokeWidth={1.6} aria-hidden="true" />
              </span>
              <span className="setting-copy">
                <span className="setting-label">{t('updatesAbout')}</span>
                <span className="setting-description">
                  {t('aboutDescription')}
                </span>
              </span>
              <ChevronRight size={16} aria-hidden="true" />
            </button>
          </nav>
        </>
      )}
      {section === 'general' && (
        <>
          <p className="page-intro">{t('generalDescription')}</p>
          <SettingsList title={t('startup')}>
            <SettingRow label={t('startWithWindows')}>
              <Toggle
                checked={snapshot?.settings.start_with_windows ?? false}
                disabled={locked || !snapshot}
                label={t('startWithWindows')}
                onChange={(enabled) =>
                  void controls.setGeneral('start_with_windows', enabled)
                }
              />
            </SettingRow>
            <SettingRow label={t('startMinimized')}>
              <Toggle
                checked={snapshot?.settings.start_minimized ?? false}
                disabled={locked || !snapshot}
                label={t('startMinimized')}
                onChange={(enabled) =>
                  void controls.setGeneral('start_minimized', enabled)
                }
              />
            </SettingRow>
            <SettingRow label={t('autoConnect')}>
              <Toggle
                checked={snapshot?.auto_connect ?? false}
                disabled={locked || !snapshot}
                label={t('autoConnect')}
                onChange={(enabled) => void controls.setAutoConnect(enabled)}
              />
            </SettingRow>
          </SettingsList>
          <SettingsList title={t('windowBehavior')}>
            <SettingRow label={t('closeToTray')}>
              <Toggle
                checked={snapshot?.settings.close_to_tray ?? true}
                disabled={locked || !snapshot}
                label={t('closeToTray')}
                onChange={(enabled) =>
                  void controls.setGeneral('close_to_tray', enabled)
                }
              />
            </SettingRow>
          </SettingsList>
        </>
      )}
      {section === 'network' && (
        <>
          <ConnectionDetails
            snapshot={snapshot}
            t={t}
            detailed
            busy={locked}
            onRecheck={() => void controls.recheckConnection()}
          />
          <SettingsList title={t('network')}>
            <NetworkSettings
              key={`${snapshot?.settings.dns}:${snapshot?.settings.custom_dns}:${snapshot?.settings.endpoint}`}
              controls={controls}
              t={t}
              disabled={locked || !disconnected || !snapshot?.has_config}
            />
          </SettingsList>
        </>
      )}
      {section === 'security' && <>
        <p className="page-intro">{t('securityDescription')}</p>
        <SettingsList title={t('killSwitch')}>
          <SettingRow label={t('killSwitch')}>
            <Toggle checked={snapshot?.settings.kill_switch ?? false} disabled={locked || !disconnected || !!snapshot?.protection?.active} label={t('killSwitch')} onChange={(enabled) => void controls.setGeneral('kill_switch', enabled)} />
          </SettingRow>
          <p className="setting-description">{t('killSwitchDescription')}</p>
          {snapshot?.protection?.active && <p className="notice" role="status">{t('protectionActive')}</p>}
          <button type="button" className="action-row" disabled={busy === 'restore'} onClick={() => void controls.restoreInternet()}>
            <span className="setting-copy"><span className="setting-label">{t('restoreInternet')}</span><span className="setting-description">{t('restoreDescription')}</span></span>
          </button>
        </SettingsList>
        <SettingsList title={t('general')}>
          <SettingRow label={t('notifications')}><Toggle checked={snapshot?.settings.notifications ?? true} disabled={locked} label={t('notifications')} onChange={(enabled) => void controls.setGeneral('notifications', enabled)} /></SettingRow>
          <SettingRow label={t('automaticUpdateChecks')}><Toggle checked={snapshot?.settings.automatic_update_checks ?? false} disabled={locked} label={t('automaticUpdateChecks')} onChange={(enabled) => void controls.setGeneral('automatic_update_checks', enabled)} /></SettingRow>
          <p className="setting-description">{t('automaticUpdatesDescription')}</p>
        </SettingsList>
        <Diagnostics t={t} />
      </>}
      {section === 'appearance'  && (
        <>
          <p className="page-intro">{t('appearanceDescription')}</p>
          <SettingsList title={t('appearance')}>
            <SettingRow label={t('language')}>
              <select
                className="native-select"
                aria-label={t('language')}
                value={appearance.language}
                onChange={(event) =>
                  appearance.setLanguage(event.target.value as Language)
                }
              >
                <option value="en">{t('english')}</option>
                <option value="cs">{t('czech')}</option>
              </select>
            </SettingRow>
            <SettingRow label={t('theme')}>
              <select
                className="native-select"
                aria-label={t('theme')}
                value={appearance.theme}
                onChange={(event) =>
                  appearance.setTheme(event.target.value as ThemePreference)
                }
              >
                <option value="system">{t('system')}</option>
                <option value="light">{t('light')}</option>
                <option value="dark">{t('dark')}</option>
              </select>
            </SettingRow>
          </SettingsList>
        </>
      )}
      {section === 'advanced' && (
        <>
          <p className="page-intro">{t('advancedIntro')}</p>
          <SettingsList title={t('configuration')}>
            <button
              type="button"
              className="action-row"
              disabled={locked || !snapshot?.has_config}
              onClick={() => void controls.exportConfig()}
            >
              <span className="setting-copy">
                <span className="setting-label">
                  {busy === 'export' ? t('exporting') : t('exportConfig')}
                </span>
                <span className="setting-description">
                  {t('exportWarning')}
                </span>
              </span>
              <Download size={16} aria-hidden="true" />
            </button>
            <button
              type="button"
              className="action-row"
              disabled={locked || !disconnected}
              onClick={() => void controls.importConfig()}
            >
              <span className="setting-copy">
                <span className="setting-label">
                  {busy === 'import' ? t('importing') : t('importConfig')}
                </span>
                <span className="setting-description">
                  {t('importDescription')}
                </span>
              </span>
              <Upload size={16} aria-hidden="true" />
            </button>
          </SettingsList>
          <SettingsList title={t('account')}>
            <button
              type="button"
              className="action-row reset-row"
              disabled={locked || !disconnected}
              onClick={() => void controls.resetAccount()}
            >
              <span className="setting-copy">
                <span className="setting-label">
                  {busy === 'reset' ? t('creating') : t('newAccount')}
                </span>
                <span className="setting-description">
                  {t('resetSafeDescription')}
                </span>
              </span>
              <RotateCw size={16} aria-hidden="true" />
            </button>
            {!disconnected && (
              <p className="setting-description">{t('disconnectFirst')}</p>
            )}
          </SettingsList>
        </>
      )}
    </div>
  )
}
