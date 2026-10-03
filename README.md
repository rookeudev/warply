<p align="center">
  <img src="assets/warply-logo.png" width="128" alt="Warply logo">
</p>

<h1 align="center">Warply</h1>

<p align="center">
  Cloudflare WARP for Windows with one button.<br>
  Connect with one button. Check whether WARP access is working.
</p>

<p align="center">
  <a href="../../releases/latest"><b>Download</b></a> ·
  <a href="#how-it-works">How it works</a> ·
  <a href="#faq">FAQ</a>
</p>

<p align="center">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4">
  <img alt="License" src="https://img.shields.io/badge/license-Source--Available-red">
  <img alt="Status" src="https://img.shields.io/badge/status-working-brightgreen">
</p>

> **Unofficial.** Warply is an independent project and is not affiliated with Cloudflare. "Cloudflare" and "WARP" are trademarks of Cloudflare, Inc.

---

## What it does

Cloudflare WARP is a free service that encrypts your internet traffic. Setting it up by hand means command-line tools, config files and the WireGuard client. Warply does all of that for you: install it, press the button, done.

## Features

- **One button.** On and off, nothing else to set up.
- **Automatic setup.** Creates a free WARP account on first launch. No sign-up, no files to import.
- **Runs in the background.** Sits in the system tray and can start with Windows.
- **Reconnects by itself** after sleep or a network change.
- **Verified status.** A running WireGuard service and a successful WARP check are shown separately.
- **Organized settings.** Startup, connection, appearance, account, and updates have their own sections.
- **Signed updates.** Check for new releases and install them from the app.
- **Private.** No account with us, no tracking, no server. Your key stays on your PC.

## Download

1. Download the installer from the [latest release](../../releases/latest).
2. Run it. Warply offers to install WireGuard if you don't have it.
3. Open Warply and press the button.

Works on Windows 10 and 11. Administrator rights are needed to create the network tunnel.

## How it works

1. On first launch, Warply creates a WireGuard key pair on your PC and registers a free WARP account with Cloudflare. Only the public key is sent.
2. It builds a standard WireGuard configuration and stores it on your machine with restricted file permissions.
3. The button starts or stops that tunnel through the official WireGuard service for Windows.
4. Rust verifies an HTTPS request through the tunnel address every 30 seconds. A running service alone does not show a verified connection.

## What WARP does and doesn't do

- It **encrypts your traffic** between your PC and Cloudflare. This helps on public Wi-Fi and hides your activity from your internet provider.
- It does **not** let you pick a country. Websites see a Cloudflare address, and your location stays roughly the same.
- It is **not** a replacement for Tor or a no-logs VPN, and it is not meant for bypassing geo-blocks.

## FAQ

**Do I need to install WireGuard first?**
No. Warply offers to install it the first time.

**Why does it ask for administrator rights?**
Creating a network tunnel on Windows requires them.

**Will it change my country?**
No. See [What WARP does and doesn't do](#what-warp-does-and-doesnt-do).

**Is it free?**
Yes. Warply and the WARP account it creates are free.

**Something stopped working.**
Cloudflare can change how free accounts are created. Please open an [issue](../../issues).

## Build from source

You need Node.js 22.12 or newer, Rust, and the Windows [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```powershell
git clone https://github.com/rookeudev/warply.git
cd warply
npm ci
npm run tauri dev       # run in development
npm run tauri -- build --no-bundle # build the app executable
```

Release installers require updater signing. See [RELEASING.md](RELEASING.md). Installer bundles are created in `src-tauri/target/release/bundle/`.

## Good to know

- Warply uses Cloudflare's unofficial registration API, which Cloudflare may change at any time.
- Connection status separates the service state from a recent WARP HTTPS check. It does not certify every application, IPv6 route, or DNS leak prevention.
- Speed and availability of free WARP are controlled by Cloudflare.
- Security details and remaining hardening work are in the [security notes](docs/security-model.md).

## Project policy

Warply is source-available, but it is **not an open-source contribution project**.

The repository is provided for transparency and personal use. Pull requests, forks intended for redistribution, and third-party versions of Warply are not accepted.

Please do not redistribute modified versions, rebrand the application, or publish your own builds as Warply.

Bug reports and security reports may still be submitted through GitHub Issues.

## Support

If you find Warply useful and want to support the project, you can buy me a coffee on Ko-fi.

[**Support Warply on Ko-fi**](https://ko-fi.com/rookdev)

## License

Warply is provided under the **Warply Source-Available License**.

You may view and use the source code for personal, non-commercial purposes. You may not redistribute the source code, publish modified versions, create competing public builds, sell the software, or remove copyright and attribution notices without permission from the author.

See [LICENSE](LICENSE) for the complete terms.
