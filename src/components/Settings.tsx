import {
  ChevronDown,
  ChevronRight,
  Download,
  RotateCw,
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

type Props = {
  controls: TunnelControls
  appearance: AppearanceControls
  setupBusy: boolean
  onAbout: () => void
}

export default function Settings({
  controls,
  appearance,
  setupBusy,
  onAbout,
}: Props) {
  const { snapshot, busy } = controls
  const { t } = appearance
  const locked = !!busy || setupBusy
  const disconnected = snapshot?.status === 'disconnected'

  return (
    <div className="settings-content">
      <SettingsList title={t('general')}>
        <SettingRow
          label={t('startWithWindows')}
          description={t('notAvailable')}
        >
          <Toggle checked={false} disabled label={t('startWithWindows')} />
        </SettingRow>
        <SettingRow label={t('startMinimized')} description={t('notAvailable')}>
          <Toggle checked={false} disabled label={t('startMinimized')} />
        </SettingRow>
        <SettingRow label={t('autoConnect')}>
          <Toggle
            checked={snapshot?.auto_connect ?? false}
            disabled={locked || !snapshot}
            label={t('autoConnect')}
            onChange={(enabled) => void controls.setAutoConnect(enabled)}
          />
        </SettingRow>
        <SettingRow label={t('closeToTray')} description={t('notAvailable')}>
          <Toggle checked={false} disabled label={t('closeToTray')} />
        </SettingRow>
      </SettingsList>

      <SettingsList title={t('network')}>
        <SettingRow label={t('dns')} description={t('notAvailable')}>
          <select
            className="native-select"
            aria-label={t('dns')}
            value="config"
            disabled
          >
            <option value="config">{t('managedConfig')}</option>
            <option value="cloudflare">{t('cloudflare')}</option>
            <option value="google">{t('google')}</option>
            <option value="quad9">{t('quad9')}</option>
            <option value="custom">{t('custom')}</option>
          </select>
        </SettingRow>
        <details className="advanced-section">
          <summary>
            {t('advanced')}
            <ChevronDown size={16} aria-hidden="true" />
          </summary>
          <SettingRow label={t('endpoint')} description={t('notAvailable')}>
            <span className="setting-value">{t('managedConfig')}</span>
          </SettingRow>
        </details>
      </SettingsList>

      <SettingsList title={t('account')}>
        <button
          type="button"
          className="action-row"
          disabled={locked || !disconnected}
          onClick={() => void controls.resetAccount()}
        >
          <span className="setting-copy">
            <span className="setting-label">
              {busy === 'reset' ? t('creating') : t('newAccount')}
            </span>
            <span className="setting-description">{t('disconnectFirst')}</span>
          </span>
          <RotateCw size={16} aria-hidden="true" />
        </button>
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
            <span className="setting-description">{t('exportWarning')}</span>
          </span>
          <Download size={16} aria-hidden="true" />
        </button>
        <details className="advanced-section">
          <summary>
            {t('advanced')}
            <ChevronDown size={16} aria-hidden="true" />
          </summary>
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
        </details>
      </SettingsList>

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

      <SettingsList title={t('about')}>
        <button type="button" className="action-row" onClick={onAbout}>
          <span>{t('notes')}</span>
          <ChevronRight size={16} aria-hidden="true" />
        </button>
      </SettingsList>
    </div>
  )
}
