import type { ReactNode } from 'react'

type Props = { label: string; description?: string; children: ReactNode }

export default function SettingRow({ label, description, children }: Props) {
  return (
    <div className="setting-row">
      <div className="setting-copy">
        <span className="setting-label">{label}</span>
        {description && <p className="setting-description">{description}</p>}
      </div>
      <div className="setting-control">{children}</div>
    </div>
  )
}
