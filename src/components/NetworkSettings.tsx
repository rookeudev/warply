import { useState } from 'react'
import { ChevronDown } from 'lucide-react'
import type { TunnelControls } from '../hooks/useTunnel'
import type { Translator } from '../i18n'
import SettingRow from './SettingRow'

export default function NetworkSettings({
  controls,
  t,
  disabled,
}: {
  controls: TunnelControls
  t: Translator
  disabled: boolean
}) {
  const saved = controls.snapshot?.settings
  const [dns, setDns] = useState(saved?.dns ?? 'config')
  const [customDns, setCustomDns] = useState(saved?.custom_dns ?? '')
  const [endpoint, setEndpoint] = useState(saved?.endpoint ?? '')
  const changed =
    dns !== (saved?.dns ?? 'config') ||
    customDns !== (saved?.custom_dns ?? '') ||
    endpoint !== (saved?.endpoint ?? '')
  return (
    <>
      <SettingRow label={t('dns')}>
        <select
          className="native-select"
          aria-label={t('dns')}
          value={dns}
          disabled={disabled}
          onChange={(event) => setDns(event.target.value)}
        >
          <option value="config">{t('managedConfig')}</option>
          <option value="cloudflare">{t('cloudflare')}</option>
          <option value="google">{t('google')}</option>
          <option value="quad9">{t('quad9')}</option>
          <option value="custom">{t('custom')}</option>
        </select>
      </SettingRow>
      {dns === 'custom' && (
        <label className="network-field">
          <span>{t('customDns')}</span>
          <input
            className="native-input"
            value={customDns}
            disabled={disabled}
            maxLength={512}
            placeholder={t('dnsHint')}
            onChange={(event) => setCustomDns(event.target.value)}
          />
        </label>
      )}
      <details className="advanced-section">
        <summary>
          {t('advanced')}
          <ChevronDown size={16} aria-hidden="true" />
        </summary>
        <label className="network-field">
          <span>{t('endpoint')}</span>
          <input
            className="native-input"
            value={endpoint}
            maxLength={260}
            disabled={disabled}
            placeholder="engage.cloudflareclient.com:2408"
            onChange={(event) => setEndpoint(event.target.value)}
          />
          <span className="setting-description">{t('endpointHint')}</span>
        </label>
      </details>
      <div className="network-actions">
        <span className="setting-description">{t('networkDescription')}</span>
        <button
          type="button"
          className="native-button"
          disabled={disabled || !changed}
          onClick={() => void controls.setNetwork(dns, customDns, endpoint)}
        >
          {t('save')}
        </button>
      </div>
    </>
  )
}
