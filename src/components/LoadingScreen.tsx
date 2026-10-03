import logo from '../../assets/warply-logo.png'

type Props = { message: string }

export default function LoadingScreen({ message }: Props) {
  return (
    <div className="app-shell loading-shell">
      <main className="loading-content" aria-busy="true">
        <img src={logo} className="loading-logo" alt="" />
        <h1>Warply</h1>
        <div className="loading-status" role="status" aria-live="polite">
          <span className="loading-spinner" aria-hidden="true" />
          <p>{message}</p>
        </div>
      </main>
    </div>
  )
}
