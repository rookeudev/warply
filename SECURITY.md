# Reporting a security issue

Do not upload WireGuard configs, DPAPI profile/service files, private keys, updater signing material, account tokens, or unredacted diagnostic output to public issues, discussions, or release assets.

For a vulnerability, use GitHub's **Report a vulnerability** option in this repository's Security tab if it is available. If it is unavailable, ask the maintainer for a private reporting channel using a minimal public message without exploit details or secrets. No private email address or reporting service is configured by this repository.

Useful non-secret report details include the Warply version, Windows version, a description of the affected behavior, and synthetic reproduction steps. Do not test against other users' accounts or systems. A screenshot of the status UI should still be reviewed before sharing.

See [privacy](docs/privacy.md) and [security model](docs/security-model.md) for implemented protections, necessary network requests, and remaining boundaries. Release security improvements are not a substitute for an independent audit. Updates require the project's Tauri signature; a downloaded initial installer still needs a trusted source.
