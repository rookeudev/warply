import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { Translator } from '../i18n'

export default function Diagnostics({ t }: { t: Translator }) {
  const [report, setReport] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  async function inspect() {
    setBusy(true)
    setMessage('')
    try { setReport(await invoke<string>('diagnostic_report')) }
    catch { setMessage(t('diagnosticsFailed')) }
    finally { setBusy(false) }
  }
  async function copy() {
    try { await navigator.clipboard.writeText(report); setMessage(t('reportCopied')) }
    catch { setMessage(t('copyManually')) }
  }
  return <section className="diagnostics" aria-label={t('diagnostics')}>
    <p className="setting-description">{t('diagnosticsDescription')}</p>
    <button type="button" className="native-button" disabled={busy} onClick={() => void inspect()}>{busy ? t('checkingUpdates') : t('diagnostics')}</button>
    {report && <>
      <textarea className="diagnostic-report" readOnly value={report} aria-label={t('diagnosticReport')} />
      <button type="button" className="native-button" onClick={() => void copy()}>{t('copyReport')}</button>
    </>}
    {message && <p role="status" className="setting-description">{message}</p>}
  </section>
}
