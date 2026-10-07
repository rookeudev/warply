import { ExternalLink } from 'lucide-react'
import { version } from '../../package.json'
import type { Translator } from '../i18n'
import SettingRow from './SettingRow'
import SettingsList from './SettingsList'

type Props = {
  t: Translator
  onProjectLink: () => void
  updateVersion: string | null
  updateBusy: 'checking' | 'installing' | null
  updateMessage: string | null
  progress: { stage: string; downloaded: number; total: number | null } | null
  onCheckUpdates: () => void
  onInstallUpdate: () => void
}

export default function About({
  t,
  onProjectLink,
  updateVersion,
  updateBusy,
  updateMessage,
  progress,
  onCheckUpdates,
  onInstallUpdate,
}: Props) {
  const stageKeys = {
    checking: 'checkingUpdates',
    confirming: 'updateConfirming',
    downloading: 'updateDownloading',
    verifying: 'updateVerifying',
    installing: 'updateStarting',
  } as const
  const stage = progress?.stage as keyof typeof stageKeys | undefined
  const percent =
    progress?.total && progress.total > 0
      ? Math.min(100, Math.floor((progress.downloaded * 100) / progress.total))
      : null
  return (
    <div className="about-content">
      <p className="about-intro">{t('notesIntro')}</p>
      <SettingsList title="Warply">
        <SettingRow label={t('version')}>
          <span className="setting-value">{version}</span>
        </SettingRow>
        {updateVersion ? (
          <button
            type="button"
            className="action-row"
            disabled={!!updateBusy}
            onClick={onInstallUpdate}
          >
            <span>
              {t('updateAvailable')} {updateVersion}
            </span>
            <span className="inline-control">
              {updateBusy === 'installing'
                ? t('installingUpdate')
                : t('installUpdate')}
            </span>
          </button>
        ) : (
          <button
            type="button"
            className="action-row"
            disabled={!!updateBusy}
            onClick={onCheckUpdates}
          >
            <span>{t('checkForUpdates')}</span>
            <span className="inline-control">
              {updateBusy === 'checking' ? t('checkingUpdates') : ''}
            </span>
          </button>
        )}
        {updateMessage && (
          <p className="setting-description" role="status">
            {updateMessage}
          </p>
        )}
        {updateBusy === 'installing' && (
          <div className="update-progress" role="status" aria-live="polite">
            <p>
              {stage && stage in stageKeys
                ? t(stageKeys[stage])
                : t('installingUpdate')}
              {stage === 'downloading' && percent != null ? ` ${percent}%` : ''}
            </p>
            <progress
              max={100}
              value={
                stage === 'downloading' && percent != null ? percent : undefined
              }
              aria-label={t('installingUpdate')}
            />
          </div>
        )}
        <SettingRow label={t('license')}>
          <span className="setting-value">Warply Source-Available</span>
        </SettingRow>
        <button type="button" className="action-row" onClick={onProjectLink}>
          <span>{t('sourceCode')}</span>
          <span className="inline-control">
            {t('github')}
            <ExternalLink size={16} aria-hidden="true" />
          </span>
        </button>
      </SettingsList>
      <details className="about-notes advanced-disclosure">
        <summary>{t('notes')}</summary>
        <p>{t('noteLocation')}</p>
        <p>{t('privacyNoCollection')}</p>
        <p>{t('privacyStorage')}</p>
        <p>{t('privacyNetwork')}</p>
        <p>{t('notePrivacy')}</p>
        <p>{t('noteGeo')}</p>
      </details>
    </div>
  )
}
