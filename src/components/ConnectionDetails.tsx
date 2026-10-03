import { RefreshCw, ShieldCheck, ShieldQuestion } from 'lucide-react'
import type { TunnelSnapshot } from '../hooks/useTunnel'
import type { Translator } from '../i18n'

export function healthLabel(snapshot: TunnelSnapshot | null, t: Translator) {
  if (snapshot?.status !== 'connected') return t('notChecked')
  switch (snapshot.health?.status) {
    case 'verified':
      return t('warpVerified')
    case 'not_warp':
      return t('warpNotDetected')
    case 'unavailable':
      return t('checkUnavailable')
    default:
      return t('verifyingConnection')
  }
}

type Props = {
  snapshot: TunnelSnapshot | null
  t: Translator
  detailed?: boolean
  busy?: boolean
  uncertain?: boolean
  onRecheck?: () => void
}

export default function ConnectionDetails({
  snapshot,
  t,
  detailed = false,
  busy,
  uncertain = false,
  onRecheck,
}: Props) {
  const running = snapshot?.status === 'connected'
  const verified =
    !uncertain && running && snapshot?.health?.status === 'verified'
  const Icon = verified ? ShieldCheck : ShieldQuestion
  const serviceLabel = running
    ? t('serviceRunning')
    : snapshot?.status === 'connecting'
      ? t('connecting')
      : snapshot?.status === 'error'
        ? t('notAvailable')
        : snapshot
          ? t('serviceStopped')
          : t('notAvailable')
  return (
    <section
      className={`connection-card${verified ? ' connection-card--verified' : ''}`}
      aria-label={t('connectionDetails')}
    >
      <div className="connection-card-heading">
        <Icon size={16} aria-hidden="true" />
        <span>{t('connectionDetails')}</span>
      </div>
      <dl className="connection-facts">
        <div>
          <dt>{t('wireGuardService')}</dt>
          <dd>{uncertain ? t('notAvailable') : serviceLabel}</dd>
        </div>
        <div>
          <dt>{t('warpConnection')}</dt>
          <dd className={verified ? 'verified-value' : ''}>
            {uncertain ? t('notChecked') : healthLabel(snapshot, t)}
          </dd>
        </div>
        {!uncertain &&
          snapshot?.health?.checked_ago_secs != null &&
          running && (
            <div>
              <dt>{t('lastCheck')}</dt>
              <dd>
                {snapshot.health.checked_ago_secs} {t('secondsAgo')}
                {detailed && snapshot.health.duration_ms != null
                  ? ` · ${snapshot.health.duration_ms} ms`
                  : ''}
              </dd>
            </div>
          )}
      </dl>
      {detailed && (
        <p className="setting-description">{t('healthExplanation')}</p>
      )}
      {detailed && onRecheck && (
        <button
          type="button"
          className="native-button check-button"
          disabled={!running || busy || snapshot?.health?.status === 'checking'}
          onClick={onRecheck}
        >
          <RefreshCw size={14} aria-hidden="true" />
          {t('checkConnection')}
        </button>
      )}
    </section>
  )
}
