# Warply v0.4.0 — less on screen

## Simplified layout

- The main screen contains the power button, one connection state and Connection details. The kill-switch/session dashboard, tagline, outer halo and repeated overview status are removed.
- A verified tunnel reads **Connected** with a short confirmation. A running service without valid WARP proof still reads **Connection not verified**. Error, stale-status, cancellation and blocked-internet recovery actions remain visible when relevant.
- Settings is one list of six clear destinations: Protection, General, Connection, Appearance, Account & config, and Updates & about. Category tiles, group headings and redundant descriptions are removed. Back/Escape and Ctrl+, remain available.
- Kill-switch state and observed-session duration are available in Connection details. Routing, DNS and individual IPv4/IPv6 proof are inside a collapsed **Technical details** disclosure. Nothing was removed from the tunnel configuration controls.
- Update availability still appears on the main screen and in Settings. The banner uses one line. About keeps version, install/check actions and source/license links; longer notes stay collapsed.
- Connection details now receive the frontend's stale-state flag, so a failed/expired status read also clears verified styling there, hides the old session time and labels uncertain fields appropriately.

Dark/light/system themes, English/Czech, reduced motion, the centered scalable power ring and keyboard accessibility remain. No backend, telemetry, database or diagnostic upload was added. Existing account, encrypted storage, kill switch and signed-updater behavior remain.

## Verification

Production TypeScript/Vite build and ESLint passed. Browser fixtures checked the minimal main view, moved session state, collapsed/expanded technical details, stale proof in details, all six destinations, update discovery/badge, OFF and Restore internet, Czech/light mode and reduced motion. The six settings destinations fit the 380×560 test viewport. The Rust tunnel logic was not changed and the Rust unit suite was not repeated. Full installer execution, real connected traffic and native Windows scaling still require manual acceptance testing.

The public installer hash, updater signature and latest manifest were checked when publishing. Warply itself has no Authenticode signature; the Tauri update signature remains mandatory.

## Install and check

Use **Warply_0.4.0_x64-setup.exe** or check for updates in Settings. Your account and encrypted configuration remain; do not reset them for a UI change.

1. Launch, connect and disconnect. Confirm the main screen has no kill-switch/session tiles or decorative halo.
2. Open Connection details. Check basic status, session and kill-switch state, then expand Technical details for route/DNS/IPv4/IPv6 proof.
3. Open every Settings destination and return using Back/Escape. Check export/import, account reset confirmation, diagnostics and Restore internet are reachable.
4. Interrupt checks without stopping the service. The app must not say Connected or retain verified styling in details after proof expires. Confirm OFF remains usable.
5. Check update discovery and installation confirmation/progress/cancellation. Daily checks stay opt-in and Windows notifications still respect preferences.
6. Check both languages/themes, reduced motion, keyboard focus and 125–200% Windows display scaling.

Release assets: installer, matching .sig, portable ZIP, latest.json, SHA256SUMS.txt, README.md, PRIVACY.md and TEST-CHECKLIST.md. Never publish private profiles, exported configurations or signing secrets.

Source: https://github.com/rookeudev/warply

License: Warply Source-Available License; see LICENSE.
