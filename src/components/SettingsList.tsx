import type { ReactNode } from 'react'

type Props = { title: string; children: ReactNode }

export default function SettingsList({ title, children }: Props) {
  return (
    <section className="settings-group" aria-label={title}>
      <h2>{title}</h2>
      <div className="settings-list">{children}</div>
    </section>
  )
}
