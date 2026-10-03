import { ExternalLink } from 'lucide-react'
import { version } from '../../package.json'
import type { Translator } from '../i18n'
import SettingRow from './SettingRow'
import SettingsList from './SettingsList'

type Props = { t: Translator; onProjectLink: () => void }

export default function About({ t, onProjectLink }: Props) {
  return (
    <div className="about-content">
      <p className="about-intro">{t('notesIntro')}</p>
      <SettingsList title="Warply">
        <SettingRow label={t('version')}>
          <span className="setting-value">{version}</span>
        </SettingRow>
        <SettingRow label={t('license')}>
          <span className="setting-value">MIT</span>
        </SettingRow>
        <button type="button" className="action-row" onClick={onProjectLink}>
          <span>{t('sourceCode')}</span>
          <span className="inline-control">
            {t('github')}
            <ExternalLink size={16} aria-hidden="true" />
          </span>
        </button>
      </SettingsList>
      <section className="about-notes" aria-label={t('notes')}>
        <h2>{t('notes')}</h2>
        <p>{t('noteLocation')}</p>
        <p>{t('notePrivacy')}</p>
        <p>{t('noteGeo')}</p>
      </section>
    </div>
  )
}
