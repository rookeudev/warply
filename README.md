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
- **Verified status.** Separate IPv4/IPv6 WARP checks, selected routes, DNS configuration, and VPN conflict checks; a running service alone never shows green.
- **Optional persistent kill switch.** Keeps ordinary internet traffic blocked during recovery, crashes, and reboot, with an explicit Restore internet action. See the security notes for permitted transport/network exceptions.
- **Local diagnostics.** Copy a partial troubleshooting report even when setup fails, without keys, IP addresses, usernames, or automatic uploads. The explicit `warply.exe --diagnostics` command also works without opening the interface.
- **Simple loading screen.** Logo and status while automatic setup starts; errors open the normal recovery screen.
- **Start menu.** The installer creates a searchable Warply shortcut. Manual launches open the window even when Windows logon is set to start in the tray.
- **Organized settings.** Startup, connection, protection, appearance, account, and updates have their own sections, with grouped navigation and direct category shortcuts. Brief animations respect Windows reduced-motion preferences.
- **Signed updates.** Check manually or opt into daily checks; installation requires confirmation.
- **Private.** No telemetry, analytics, usage history, crash uploads, or Warply server. Your private key stays on your PC. [Privacy details](docs/privacy.md).

## Download

1. Download the installer from the [latest release](../../releases/latest).
2. Run it. Warply offers to install WireGuard if you don't have it.
3. Open Warply and press the button.

Works on Windows 10 and 11. The interface runs as your normal Windows user. A separate privileged helper needs one UAC approval per app session to control the tunnel.

## How it works

1. On first launch, Warply creates a WireGuard key pair on your PC and registers a free WARP account with Cloudflare. Registration sends the public key and fixed protocol fields; the private key is never uploaded.
2. It builds a standard WireGuard configuration and stores it encrypted with Windows DPAPI and restricted file permissions. The service receives a separate encrypted copy; normal connection creates no plaintext config.
3. The button starts or stops that tunnel through the official WireGuard service for Windows.
4. Rust verifies separate IPv4 and IPv6 HTTPS requests through the tunnel addresses every 30 seconds and inspects selected local routes/DNS. A running service alone does not show a verified connection.

## What WARP does and doesn't do

- It **encrypts your traffic** between your PC and Cloudflare. This helps on public Wi-Fi and hides your activity from your internet provider.
- It does **not** let you pick a country. Websites see a Cloudflare address, and your location stays roughly the same.
- It is **not** a replacement for Tor or a no-logs VPN, and it is not meant for bypassing geo-blocks.

## FAQ

**Do I need to install WireGuard first?**
No. Warply offers to install it the first time.

**Why does it ask for administrator rights?**
A separate helper needs them to control the tunnel and optional firewall protection. The interface stays unelevated, and toggles reuse the approved helper.

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
- Connection status separates the service state from a recent WARP HTTPS check. It does not certify every application, all destinations, or external DNS leak prevention.
- Speed and availability of free WARP are controlled by Cloudflare.
- Security details and remaining boundaries are in the [security notes](docs/security-model.md).

## Project policy

Warply is source-available, but it is **not an open-source contribution project**.

The repository is provided for transparency and personal use. Pull requests, forks intended for redistribution, and third-party versions of Warply are not accepted.

Please do not redistribute modified versions, rebrand the application, or publish your own builds as Warply.

Non-sensitive bug reports may be submitted through GitHub Issues. For security issues, follow [SECURITY.md](SECURITY.md); never attach private configurations.

## Support

If you find Warply useful and want to support the project, you can buy me a coffee on Ko-fi.

[**Support Warply on Ko-fi**](https://ko-fi.com/rookdev)

## License

Warply is provided under the **Warply Source-Available License**.

You may view and use the source code for personal, non-commercial purposes. You may not redistribute the source code, publish modified versions, create competing public builds, sell the software, or remove copyright and attribution notices without permission from the author.

See [LICENSE](LICENSE) for the complete terms.

## Windows acceptance tests

See [the 0.2.1 manual test checklist](docs/0.2.1-test-checklist.md), including persistent protection, recovery, updates, and privilege isolation.
