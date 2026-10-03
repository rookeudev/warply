import { Power } from 'lucide-react'

export type PowerState =
  'disconnected' | 'connecting' | 'connected' | 'unverified' | 'error'

type Props = {
  state: PowerState
  pressed: boolean
  disabled: boolean
  label: string
  onClick: () => void
}

export default function PowerButton({
  state,
  pressed,
  disabled,
  label,
  onClick,
}: Props) {
  return (
    <button
      type="button"
      className={`power-button power-button--${state}`}
      disabled={disabled}
      aria-label={label}
      aria-pressed={pressed}
      aria-busy={state === 'connecting'}
      onClick={onClick}
    >
      <svg className="power-ring" viewBox="0 0 120 120" aria-hidden="true">
        <circle className="power-ring-track" cx="60" cy="60" r="58" />
        {state === 'connecting' && (
          <circle className="power-ring-arc" cx="60" cy="60" r="58" />
        )}
      </svg>
      <Power size={40} strokeWidth={1.7} aria-hidden="true" />
    </button>
  )
}
