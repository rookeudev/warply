# UI and connection review — v0.3.0

## Implemented

- The supplied logo, fixed 380 × 560 window, Segoe font, light/dark/system themes, and Windows Mica fallback remain.
- The main power button is green only after a WARP verification request succeeds, blue while checking, and amber for an unverified running service. ON/OFF intent follows the service state, so an unverified service can always be disconnected.
- A compact status card separates WireGuard service state from WARP verification and shows the age of the last check. Connection & DNS includes a manual recheck and a clear explanation of the probe's limits.
- Settings opens an overview with five visible cards: Startup & behavior, Connection & DNS, Appearance, Account & advanced, and Updates & about. Back, Escape, and Alt+Left return through the section hierarchy. English and Czech labels are supported.
- Config import/export and reset are in Account & advanced. Network edits and reset are disabled while the service runs. Reset asks for native confirmation and preserves the old profile on registration failure.
- The tray marks verified WARP separately from a running but unverified service. Reconnect continues to follow service/network events, not trace-server failures.

## Verification

Frontend production build, ESLint, Rust formatting, Clippy, and Rust unit tests pass. A Playwright fixture at 380 × 560 covers all menu sections, keyboard Back/Escape navigation, English/light and Czech/dark, state rendering, an enabled Disconnect on an unverified service, disabled reset during service operation, and no horizontal overflow. The overview fits the viewport without scrolling. Production npm audit reports no known vulnerabilities at the time of this check.

These are fixture screenshots, not a live VPN session:

- [Settings overview](ui-overview.png)
- [Unverified running service](ui-status-unverified.png)
- [Connection details in Czech/dark](ui-network-cs.png)

## Manual acceptance

- Start with a saved config and WireGuard installed, connect, and wait for WARP verification. Verify tray and main-screen agreement.
- Leave the service running and block tunnel traffic. Within the verification interval, the UI must become unverified; Disconnect must remain available.
- Block only the trace endpoint. Show an unverified check without restarting the service. Restore access and use Check connection.
- Disconnect during an in-flight check, then reconnect. The old result must not verify the new session.
- Switch networks and sleep/resume. Verify recovery invalidates the old health result.
- Test IPv6 and DNS routing separately; the status check does not certify either for all applications.
- Browse every settings section in both languages/themes. Test Tab/Space/Enter, Escape, Alt+Left, high contrast, and 100/125/150 percent scaling.
- Cancel reset; then force a registration failure. The old profile must remain usable. Confirm that a successful reset replaces it only once the replacement is ready.
- Try duplicate sections, invalid optional fields, script hooks, and linked files. Imports/exports must fail safely.
- Check normal setup, connect/disconnect, tray, autostart from Program Files, updater cancellation, and a signed update in a disposable Windows VM.

## Startup and Start menu — 0.2.0

The main window shows a minimal loading screen with the local logo, a spinner, and the current setup status while initialization is pending. It has no simulated percentage or fixed delay. A ready or failed setup returns to the normal main view, including retry/Advanced options. Reduced-motion settings stop the spinner animation.

The NSIS post-install hook restores the all-users `Warply.lnk` Start menu entry on both fresh installs and updates, sets its app identity, and notifies the Windows shell. Manual launches always show the app; the tray-at-logon preference only affects `--autostart` launches. Portable extraction alone does not register an installed Start menu application.

Manual acceptance: install the EXE, search Start for Warply, launch it with tray-at-logon enabled, and confirm the window opens. Repeat an installer update after removing its shortcut; check that uninstall removes the shortcut. Test loading-to-ready and loading-to-error, light/dark, Czech/English, and reduced motion.

## 0.2.1 review

Security settings, explicit protection status/Restore internet, local diagnostics, separate IPv4/IPv6 details, update opt-in, and availability banner were reviewed in English/light and Czech/dark fixtures. Diagnostics copying, default-off automatic checks, opt-in detection, and emergency recovery during a pending connection were exercised. Mocked UI tests do not establish live WFP or VPN behavior. See the Windows acceptance checklist.
