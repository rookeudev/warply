<p align="center">
  <img src="assets/warply-logo.png" width="128" alt="Warply logo">
</p>

<h1 align="center">Warply</h1>

<p align="center">
  Cloudflare WARP for Windows with one button.<br>
  Click on, you're protected. Click off, you're back to normal.
</p>

<p align="center">
  <a href="../../releases/latest"><b>Download</b></a> ·
  <a href="#how-it-works">How it works</a> ·
  <a href="#faq">FAQ</a>
</p>

<p align="center">
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D4">
  <img alt="License" src="https://img.shields.io/badge/license-MIT-green">
  <img alt="Status" src="https://img.shields.io/badge/status-work%20in%20progress-orange">
</p>

> **Unofficial.** Warply is an independent open-source project and is not affiliated with Cloudflare. "Cloudflare" and "WARP" are trademarks of Cloudflare, Inc.

---

## Why Warply

Cloudflare WARP is free and built on WireGuard, but setting it up by hand means command-line tools, config files and the WireGuard client. Warply does all of that for you. Install it, press the button, done.

## What is Warply?

Cloudflare WARP is a free service that encrypts your traffic and routes it through Cloudflare's network. It is built on WireGuard, but the official app is closed-source and offers almost no settings.

Warply makes the same setup available through a simple Windows app: no terminal, no manual config, and no account with us.

## Features

- **One button.** On and off, nothing else to configure.
- **Automatic setup.** Creates a free WARP account on first launch. No sign-up, no files to import.
- **Runs in the background.** Lives in the system tray and can start with Windows, quietly.
- **Reconnects by itself** after sleep or a network change.
- **Private by design.** No account with us, no telemetry, no server. Your key never leaves your PC.

## Download

1. Go to the [latest release](../../releases/latest) and download the Windows `.exe` installer.
2. Run it. Warply asks to install WireGuard if you don't have it.
3. Open Warply and press the button.

Windows 10 and 11 are supported. A fresh manual launch requests administrator rights. Logon startup through Task Scheduler and reopening an existing tray instance do not request another UAC prompt. Autostart requires an installation in Program Files.

## How it works

1. On first launch, Warply creates a WireGuard key pair on your PC and registers a free WARP account with Cloudflare. Only the public key is sent.
2. It builds a standard WireGuard configuration and stores it protected on your machine.
3. The on/off button starts or stops that tunnel using the official WireGuard service for Windows.

## What WARP does and doesn't do

- It **encrypts your traffic** between your PC and Cloudflare, which protects you on public Wi-Fi and hides your activity from your internet provider.
- It does **not** let you pick a country. Websites see a Cloudflare address, and your location stays roughly the same.
- It is **not** a replacement for Tor or a no-logs VPN, and it is not meant for bypassing geo-blocks.

## Privacy and security

- Your private key is created locally and never leaves your device.
- No analytics, no tracking, no ads.
- The app does not look up your public IP or show invented connection statistics.
- Releases are published as an installer only, with checksums.
- The saved profile currently uses restricted Windows file permissions, not encrypted storage. See the [security audit](docs/security-model.md) for remaining hardening work.

## FAQ

**Do I need to install WireGuard first?**
No. Warply offers to install it for you the first time.

**Why does it ask for administrator rights?**
Creating a network tunnel on Windows requires them. Warply elevates at process startup, not on each toggle. Installed logon startup runs through a highest-privilege scheduled task.

**Will it change my country or unlock other regions?**
No. See [What WARP does and doesn't do](#what-warp-does-and-doesnt-do).

**Is it free?**
Yes. Warply and the WARP account it creates are free.

**Something stopped working.**
Cloudflare can change how free accounts are created. Open an [issue](../../issues) and we'll fix it.

## Build from source

You need Node.js 22.12 or newer, Rust, and the Windows [Tauri prerequisites](https://tauri.app/start/prerequisites/), including the Visual Studio C++ build tools.

```powershell
git clone https://github.com/rookeudev/warply.git
cd warply
npm ci
npm run tauri dev
```

Create Windows installers:

```powershell
npm run tauri build
```

The installers are generated in `src-tauri/target/release/bundle/`. Build output, dependencies, generated schemas and private configs are excluded from Git. To restore dependencies after cloning or cleaning the folder, run `npm ci`; Tauri regenerates its schemas during development/build.

## Settings

- Start with Windows, start minimized, automatic connection and close to tray.
- Cloudflare, Google, Quad9 or custom DNS, and an advanced endpoint editor. Disconnect before applying changes.
- Account reset and explicit configuration export with a private-key warning. Import is available under Account → Advanced.
- English/Czech and system/light/dark themes.

See the [UI and background review checklist](docs/ui-review.md) for previews, verification results and native Windows checks still requiring manual testing.

## Known limitations

- Warply relies on Cloudflare's unofficial registration API, which may change or restrict registrations.
- Connection status reflects the WireGuard Windows service, not a verified recent handshake or Internet reachability.
- This is a work-in-progress review build. Encrypted storage, privilege separation and signed release hardening remain pending; see the [security audit](docs/security-model.md).
- Free WARP speed and availability are controlled by Cloudflare.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development and testing instructions. Never include private keys, account tokens or generated `.conf` files in commits or issues.

## Disclaimer

This software is provided as-is, without warranty. You are responsible for how you use it and for following Cloudflare's terms of service and the laws of your country.

## License

[MIT](LICENSE)
