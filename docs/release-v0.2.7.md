# Warply 0.2.6 bug fixes — v0.2.7

This is the bug-fix update for 0.2.6. Its internal version is **0.2.7**, so installed 0.2.6 clients can receive it through the updater.

## Fixes and improvements

- Windows network flyout displays **Warply** instead of numbered names such as `warply 3`. The privileged helper uses the Network List Manager API and checks the exact verified adapter GUID. It renames only networks whose connections all belong to that adapter. Physical Wi-Fi names, adapter/service aliases and network categories remain intact. Delayed network-profile creation is retried; cosmetic failures do not stop a working tunnel.
- A verified connection with the kill switch off now reports that no repair is needed. Kill-switch state is labeled separately from WARP health. Check age is readable, without Rust `Some(...)` formatting. Genuine blocked/unknown states keep recovery advice.
- Main screen focuses on the power button, one connection status and the link to full details. Technical fields remain in Connection details. Settings use one category overview and the header Back button; repeated shortcut navigation is removed. Long About notes are collapsed.
- Available updates have a visible banner and a badge in Settings. Installation shows confirmation, download percentage when available, signature verification and installer startup stages. Progress clears when canceled or failed.
- With automatic update checks and notifications enabled, newly discovered updates also use local Windows notifications. Checks remain opt-in and at most daily; installation always needs confirmation. Windows notification settings can suppress toasts.

No telemetry, diagnostic uploads, backend, database or profile reset was added. Updates still require the mandatory Tauri signature, signed version and exact official release asset. Tunnel shutdown happens only after download verification. The frontend receives limited progress, never update bytes or private configurations.

## Verification

45 regular Rust tests passed. Two explicit live checks passed on the affected Windows computer: native network rename to **Warply** with category **Public** preserved, and full source-bound IPv4/IPv6 WARP verification after the rename. Registration and official WireGuard-installer download tests were not run. cargo fmt, cargo clippy, ESLint and the production build passed.

Browser fixtures verified the simpler main screen, preserved detailed proof, category navigation, update banner, downloading/signature stages and cancellation. The actual signed updater manifest, release hashes and installer signature were verified when publishing. Full external installer execution, reboot recovery and Windows notification delivery still require acceptance checks.

## Install

Download **Warply_0.2.7_x64-setup.exe** or check for updates in Settings. Close the old app/tray instance for a manual install, then reconnect. Existing encrypted profiles and accounts remain. Reopen the Windows network flyout if it is still showing a cached name. Warply itself has no Authenticode signature; update artifacts have the required Tauri signature.

## Manual checklist

1. Connect, open the Windows network flyout and confirm **Warply**. Disconnect/reconnect and confirm it does not become `Warply 2` or `warply 3`. Check that physical Wi-Fi names and network category are unchanged.
2. Run diagnostics with a verified connection and kill switch off; confirm the report does not recommend uninstall/Restore internet. Enable the kill switch while disconnected, reconnect and check its separate state.
3. Open Connection details and confirm IPv4/IPv6, DNS, last-check age and manual recheck remain accessible. Verify Back/Escape returns to the category overview.
4. Check for an available update and inspect the main banner and Settings badge. Confirm installation, watch download/verification stages, and test cancellation without disconnecting the tunnel.
5. Enable daily checks and notifications, then verify an available release causes a Windows notification. With daily checks disabled, launching the app must not contact GitHub automatically.
6. Test a failed or invalid signed download. It must not run an installer or interrupt the tunnel, and the progress display must clear.

Release assets: installer, matching .sig, portable ZIP, latest.json, SHA256SUMS.txt, README.md, PRIVACY.md and TEST-CHECKLIST.md. Never publish private profiles, exported configs or signing secrets.

Source: https://github.com/rookeudev/warply

License: Warply Source-Available License; see LICENSE.
