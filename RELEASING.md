# Warply releases and updates

Warply checks the latest GitHub release on launch. The **About** screen can check again and install a newer version. Update packages are verified with Tauri's signing key before installation. This signature is separate from Windows Authenticode signing.

## Release a new version

1. Increase the version in `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json` to the same SemVer value.
2. Run `npm ci`, `npm run build`, `npm run lint`, `cargo fmt --all -- --check`, and `cargo clippy --all-targets --locked -- -D warnings`.
3. Commit and push the source, then push a matching `vX.Y.Z` tag. GitHub Actions builds the Windows installers, signs updater packages, and publishes `latest.json` to the release.
4. Check that the release contains `latest.json`, a Windows installer, and its `.sig` file. A release must be published as **latest** for the app to see it.

The repository's Actions secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` were configured for the first updater release. Keep a separate, secure backup of the private key and password in `~/.tauri/warply-updater/` on the release machine. Losing the key prevents existing installations from trusting future updates; replacing it requires a manual installer update. Never commit or upload the private key or password as release assets.

Installations of `v0.1.0` have no updater and need one manual install of `v0.2.0` or later. Updates after that can run from the app.
