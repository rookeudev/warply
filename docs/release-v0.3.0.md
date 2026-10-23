# Warply v0.3.0 — a clearer desktop

## New interface

- Refreshed blue surfaces inspired by the Warply logo, a clearer power control, and compact connection information. Light, dark and system themes remain available.
- Settings categories use paired tiles. Full routing, DNS and dual-stack proof remain in Connection details; recovery actions remain accessible.
- A compact kill-switch indicator opens Protection & recovery. Verified connections do not also show a redundant protection warning; blocked/offline and unknown protection states still retain recovery messages.
- Improved focus visibility, existing reduced-motion support and a forced-colors fallback. English and Czech are included.

## Useful additions

- **Current session timer.** Rust tracks when this application observes a running tunnel. The counter survives a manual health recheck and clears after disconnect/invalidation. It starts over on app restart; it does not claim to measure how long an already-running Windows service has existed. No session history is persisted.
- **Quick connection recheck.** An unverified connection can be checked from the main screen, without finding the network settings page.
- **Ctrl+,** (Command+, where applicable) opens Settings. Existing Back/Escape navigation remains available.
- **Stale-status protection.** The frontend stops displaying verified green when the reported proof reaches its 30-second validity window or no fresh snapshot has arrived for approximately 30 seconds. A visible one-second clock updates this indication. A stalled response cannot leave the previous green indicator indefinitely. OFF stays available for the last known running service.
- Visible session/freshness updates pause while the page is hidden and refresh when it becomes visible. Existing status polling remains coalesced; no new background network calls were added.

The existing authenticated helper, encrypted profiles, restricted permissions, persistent kill switch, native network-name repair and signed updater remain. No backend, telemetry, analytics, database or diagnostic upload was added.

## Verification

46 regular Rust tests passed; 4 opt-in integration tests were not run for this release. cargo fmt, cargo clippy, ESLint and the production build passed. The new Rust test checks that session time survives a manual recheck and clears on disconnect.

Browser fixtures verified the timer, clean verified state, keyboard navigation, a stalled status request losing green, OFF during that stall, timer clearing, coalesced refreshes, quick recheck, Czech/light mode and reduced motion. The signed release manifest, updater signatures and public installer hash were checked when publishing.

No real connected traffic, full installer execution, crash/reboot or Windows notification delivery test was performed for this version. Historical live tests of 0.2.5/0.2.7 do not replace a current live test. No independent security audit is claimed.

## Install

Use **Warply_0.3.0_x64-setup.exe** or check for updates in Settings. Existing accounts and encrypted profiles remain; a reset is unnecessary. Automatic update checks remain opt-in and installation requires confirmation. Warply itself has no Authenticode signature; updater artifacts have the required Tauri signature.

## Manual checklist

1. Launch with an existing profile; check both themes and Czech/English. Confirm the power button remains usable at Windows display scaling settings.
2. Connect, wait for dual-stack verification and watch Current session advance. Manually recheck; the timer must not reset. Disconnect; it must clear. Restart the app; the observed-session timer begins again.
3. Leave the service running but interrupt WARP access. The main screen must lose green and offer a recheck; restoring access can produce a new proof.
4. In a controlled test environment, stall status replies for over 30 seconds. The previous green proof must expire and OFF must remain available. Resume responses and confirm normal status returns.
5. Open Settings using Ctrl+, then visit each category and return with Back/Escape. Check full connection details, native export confirmation and recovery access.
6. Hide/show the app, then check for a signed update. Review the banner, download/signature stages and cancellation. Verify opt-out launches make no automatic GitHub request.

Release assets: installer, matching .sig, portable ZIP, latest.json, SHA256SUMS.txt, README.md, PRIVACY.md and TEST-CHECKLIST.md. Never publish profiles, exported configurations or signing secrets.

Source: https://github.com/rookeudev/warply

License: Warply Source-Available License; see LICENSE.
