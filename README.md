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
```

```bash
git clone https://github.com/YOUR-USERNAME/warply.git
cd warply
npm install
npm run tauri dev      # run in development
cd warply
npm install
npm run tauri dev      # run in development
npm run tauri build    # create a release build

Requirements: Node.js 18+, Rust, and the [Tauri prerequisites](https://tauri.app/start/prerequisites/) for your OS.

## Usage

1. Open Warply and click **Generate config**.
2. Optionally adjust DNS, split tunneling, or endpoint.
3. Scan the **QR code** with the WireGuard app on your phone, or **export the `.conf` file** and import it into WireGuard on your computer.
4. Connect in WireGuard and use the **leak check** to verify it works.

You need the official [WireGuard client](https://www.wireguard.com/install/) installed to connect.

## How it works

1. Warply generates a WireGuard key pair locally.
2. It registers the public key with Cloudflare's WARP registration API (the same one used by the official client and `wgcf`).
3. Cloudflare returns your address and peer details.
4. Warply assembles them, together with your private key, into a standard WireGuard config.

Your private key is created and stored only on your device. Warply has no backend.

## Roadmap

- [ ] Account registration and config export
- [ ] QR code generation
- [ ] DNS and endpoint options
- [ ] Split tunneling editor
- [ ] IP / DNS / WebRTC / IPv6 leak check
- [ ] Automatic endpoint speed test
- [ ] Translations (Czech, English, ...)

## Known limitations

- Warply relies on an unofficial use of Cloudflare's registration API. Cloudflare may change or restrict it at any time, which could break the app until it is updated.
- Free WARP speed and availability are controlled by Cloudflare.

## Contributing

Issues and pull requests are welcome. If you plan a bigger change, please open an issue first so we can discuss it.

1. Fork the repository
2. Create a branch: `git checkout -b feature/my-feature`
3. Commit your changes and open a pull request

## Disclaimer

This software is provided as-is, without warranty. You are responsible for how you use it and for following Cloudflare's terms of service and the laws of your country.

## License

[MIT](LICENSE)
