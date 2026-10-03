# Warply

**Unofficial one-click Cloudflare WARP config generator.**
Get a free WireGuard configuration and a QR code for your phone or PC, no terminal needed.

![License](https://img.shields.io/badge/license-MIT-blue)
![Status](https://img.shields.io/badge/status-work%20in%20progress-orange)

> **Not affiliated with Cloudflare.** Warply is an independent open-source project. "Cloudflare" and "WARP" are trademarks of Cloudflare, Inc.

<!-- Add a screenshot or GIF here: ![Warply demo](docs/demo.gif) -->

---

## What is Warply?

Cloudflare WARP is a free service that encrypts your traffic and routes it through Cloudflare's network. It is built on WireGuard, but the official app is closed-source and offers almost no settings.

Tools like `wgcf` can generate a standard WireGuard config from a free WARP account, but they only work in a terminal. **Warply does the same thing with a simple interface**, so anyone can use WARP with any WireGuard client.

## Features

- One-click registration of a free WARP account
- WireGuard config generated locally (your private key never leaves your device)
- QR code for the WireGuard mobile apps and a `.conf` file for desktop
- Custom DNS
- Split tunneling (choose what goes through WARP)
- Custom endpoint, useful when the default one is slow or blocked
- Built-in IP / DNS leak check
- No server, no database, no account with us, no tracking

> Some features may still be in development. See the [Roadmap](#roadmap).

## Important: what WARP does and does not do

Please read this before using it.

- WARP **encrypts your traffic** and hides it from your ISP and from people on the same public Wi-Fi.
- WARP **does not let you choose a country**. Websites will see a Cloudflare IP address, and your location will roughly match your real one.
- WARP is **not a replacement for Tor** or a no-logs VPN. Cloudflare can see connection metadata.
- It is **not meant for bypassing geo-blocks**.

## Installation

### Download

Grab the latest build for your system from the [Releases](../../releases) page.

| System  | File          |
| ------- | ------------- |
| Windows | `.exe` / `.msi` |
| macOS   | `.dmg`        |
| Linux   | `.AppImage` / `.deb` |

### Build from source

```bash
git clone https://github.com/YOUR-USERNAME/warply.git
cd warply
npm install
npm run tauri dev      # run in development
npm run tauri build    # create a release build
```

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
