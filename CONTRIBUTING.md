# Contributing

Issues and pull requests are welcome. For larger changes, please open an issue first.

1. Install the prerequisites in the README and run `npm install`.
2. Make one focused change per pull request.
3. Run `npm run lint`, `npm run build`, `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
4. Describe what changed and how you tested it.

On Windows, run `npm install` and `npm run tauri dev`. Warply first focuses an existing instance, otherwise it restarts itself with a single UAC prompt when needed. Task Scheduler starts the installed app with highest privileges at logon. Rust and the Visual Studio C++ build tools are required for development.

First launch creates local keys, registers a free WARP account, and saves `%LOCALAPPDATA%\Warply\warply.conf` with restricted ACLs. Encrypted storage remains pending in the security-hardening work. If WireGuard is missing, one confirmation allows automatic installation. Later launches reuse the saved config and installation decision. Settings → Account contains reset/export; its Advanced section contains import. Exported files contain a private key.

See [UI and background review](docs/ui-review.md) for the visual and native Windows test checklist. Autostart requires installation in Program Files; development/portable copies cannot register an elevated startup task.

Manual checks:

- In a disposable Windows VM, test a clean first launch without WireGuard: account progress, one installation confirmation, and a Disconnected power button after setup. Test both winget and the official MSI fallback.
- Test a clean first launch with WireGuard installed: automatic registration with no setup dialogs.
- Close and reopen: the config stays unchanged, no registration or installation prompt occurs. Enable automatic connection and reopen to check that setting separately.
- Connect and disconnect: `WireGuardTunnel$warply` starts and is removed, and the status follows it.
- Test offline registration, a rate-limit/API rejection response, and retry. Settings → Advanced must still allow importing a valid config. Invalid configs must show a friendly error.
- Decline installation or simulate a failed download/install: the official download link and Check again remain available, and later launches do not repeat the confirmation.
- Reset while disconnected; export after its private-key warning, including cancelling the export. Check that no private key appears in the UI or logs.
- Launch from an ordinary terminal: one UAC prompt. Declining UAC must not start setup or loop through prompts.

The API URL, version, headers, and registration body were checked against [wgcf cloudflare/api.go](https://github.com/ViRb3/wgcf/blob/ace873cbaa618365beebde5790a7fb3481e5a211/cloudflare/api.go). wgcf uses a custom Android TLS fingerprint; Warply uses reqwest/rustls with TLS 1.2 and HTTP/1.1. Live registration succeeded during implementation, but this unofficial API can change. Generated profiles use `engage.cloudflareclient.com:2408` as required.

Normal tests do not create accounts or install software. The ignored live tests can be run explicitly with `cargo test --manifest-path src-tauri/Cargo.toml live_registration -- --ignored` (creates a free account) and `cargo test --manifest-path src-tauri/Cargo.toml official_download_passes_authenticode -- --ignored` (downloads and verifies only).

Never include a generated `.conf` file, a private key, or account tokens in issues or commits.
