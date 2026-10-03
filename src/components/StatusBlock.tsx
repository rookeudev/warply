type Props = { title: string; secondary: string; error: boolean }

export default function StatusBlock({ title, secondary, error }: Props) {
  return (
    <div
      className="status-block"
      role="status"
      aria-live="polite"
      aria-atomic="true"
    >
      <p className={`status-title${error ? ' status-title--error' : ''}`}>
        {title}
      </p>
      <p className="status-secondary">{secondary}</p>
    </div>
  )
}
