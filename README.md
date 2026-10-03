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

<!-- Add a screenshot here, for example: -->
<!-- <p align="center"><img src="docs/screenshot.png" width="360" alt="Warply screenshot"></p> -->

> **Unofficial.** Warply is an independent open-source project and is not affiliated with Cloudflare. "Cloudflare" and "WARP" are trademarks of Cloudflare, Inc.

---

## Why Warply

Cloudflare WARP is free and built on WireGuard, but setting it up by hand means command-line tools, config files and the WireGuard client. Warply does all of that for you. Install it, press the button, done.

## Features

- **One button.** On and off, nothing else to configure.
- **Automatic setup.** Creates a free WARP account on first launch. No sign-up, no files to import.
- **Runs in the background.** Lives in the system tray and can start with Windows, quietly.
- **Reconnects by itself** after sleep or a network change.
- **Private by design.** No account with us, no telemetry, no server. Your key never leaves your PC.

## Download

1. Go to the [latest release](../../releases/latest) and download the installer (`Warply-Setup.exe`).
2. Run it. Warply asks to install WireGuard if you don't have it.
3. Open Warply and press the button.

Windows 10 and 11 are supported. Administrator rights are needed once, to create the network tunnel.

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
- Showing your public IP in the app makes one request to an IP service. You can turn it off in Settings.
- Releases are published as an installer only, with checksums.
- Found a security problem? See [SECURITY.md](SECURITY.md).

## FAQ

**Do I need to install WireGuard first?**
No. Warply offers to install it for you the first time.

**Why does it ask for administrator rights?**
Creating a network tunnel on Windows requires them. Warply asks once.

**Will it change my country or unlock other regions?**
No. See [What WARP does and doesn't do](#what-warp-does-and-doesnt-do).

**Is it free?**
Yes. Warply and the WARP account it creates are free.

**Something stopped working.**
Cloudflare can change how free accounts are created. Open an [issue](../../issues) and we'll fix it.

## Build from source

You need Node.js 18+, Rust and the [Tauri prerequisites](https://tauri.app/start/prerequisites/).

```bash
git clone https://github.com/YOUR-USERNAME/warply.git
cd warply
npm install
npm run tauri dev      # run in development
npm run tauri build    # build the installer
```

## Contributing

Issues and pull requests are welcome. For bigger changes, please open an issue first.

## License

[MIT](LICENSE). Provided as-is, without warranty. You are responsible for following Cloudflare's terms and the laws of your country.