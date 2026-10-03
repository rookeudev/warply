type Props = {
  checked: boolean
  disabled?: boolean
  label: string
  onChange?: (checked: boolean) => void
}

export default function Toggle({ checked, disabled, label, onChange }: Props) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      className="toggle"
      onClick={() => onChange?.(!checked)}
    >
      <span className="toggle-thumb" />
    </button>
  )
}
