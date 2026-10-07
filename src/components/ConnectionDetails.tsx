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

export function healthIssueLabel(
  snapshot: TunnelSnapshot | null,
  t: Translator,
) {
  const issue = snapshot?.health.issue
  const keys = {
    inspection: 'issueInspection',
    routes: 'issueRoutes',
    dns: 'issueDns',
    conflict: 'issueConflict',
    not_warp: 'issueNotWarp',
    ipv4: 'issueIpv4',
    ipv6: 'issueIpv6',
  } as const
  return issue && issue in keys ? t(keys[issue]) : null
}

export default function ConnectionDetails({
  snapshot,
  t,
  detailed = false,
  busy,
  uncertain = false,
  onRecheck,
}: Props) {
  const issue = snapshot?.health.issue
  const issueKeys = {
    inspection: 'issueInspection',
    routes: 'issueRoutes',
    dns: 'issueDns',
    conflict: 'issueConflict',
    not_warp: 'issueNotWarp',
    ipv4: 'issueIpv4',
    ipv6: 'issueIpv6',
  } as const
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
        {detailed && snapshot?.health.network && (
          <>
            <div>
              <dt>{t('routeV4')}</dt>
              <dd>
                {!uncertain && snapshot.health.network.ipv4_tunnel
                  ? t('tunnelRoute')
                  : t('notVerified')}
              </dd>
            </div>
            <div>
              <dt>{t('routeV6')}</dt>
              <dd>
                {!uncertain && snapshot.health.network.ipv6_tunnel
                  ? t('tunnelRoute')
                  : t('notVerified')}
              </dd>
            </div>
            <div>
              <dt>{t('dnsCheck')}</dt>
              <dd>
                {!uncertain && snapshot.health.network.dns_matches
                  ? t('dnsConfigured')
                  : t('notVerified')}
              </dd>
            </div>
            <div>
              <dt>IPv4 WARP</dt>
              <dd>
                {!uncertain && snapshot.health.ipv4 === 'verified'
                  ? t('warpVerified')
                  : t('notVerified')}
              </dd>
            </div>
            <div>
              <dt>IPv6 WARP</dt>
              <dd>
                {!uncertain && snapshot.health.ipv6 === 'verified'
                  ? t('warpVerified')
                  : t('notVerified')}
              </dd>
            </div>
            <div>
              <dt>{t('otherVpns')}</dt>
              <dd>
                {uncertain || !snapshot.health.network.inspection_available
                  ? t('notChecked')
                  : snapshot.health.network.other_vpn_count}
              </dd>
            </div>
          </>
        )}
      </dl>
      {detailed && (
        <p className="setting-description">{t('healthExplanation')}</p>
      )}
      {running && !uncertain && issue && issue in issueKeys && (
        <p className="setting-description" role="status">
          {t(issueKeys[issue])}
        </p>
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
