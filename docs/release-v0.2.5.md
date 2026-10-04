# Warply v0.2.5 — fix incorrect connection verification

## Fix

Windows PowerShell 5.1 returns a JSON DNS array as one pipeline object. Warply previously tried to parse that entire object as a single IP address. This caused local inspection to fail whenever the tunnel was active and configured with multiple DNS servers. The main screen then reported “Connection not verified” even when WARP traffic worked.

The DNS array is now enumerated before parsing. Full verification still requires source-bound Cloudflare proof for IPv4 and IPv6, matching tunnel routes and DNS, and no conflicting active VPN adapter. No security checks were weakened, and no profile reset is required.

## Verification

- 41 regular Rust tests passed. The DNS regression test runs the actual Windows PowerShell executable with empty, single-server and mixed IPv4/IPv6 arrays.
- An additional live integration test passed on the affected computer with its existing connected tunnel: IPv4 and IPv6 source-bound Cloudflare requests, routes and DNS all verified.
- cargo fmt, cargo clippy, ESLint and the production build passed.
- The remaining installer-download and account-registration integration tests were not run; no new account was created.

The local investigation printed only limited states and the WARP result, without private keys, profile contents or user IP addresses. No telemetry or diagnostic upload was added.

## Install

Install **Warply_0.2.5_x64-setup.exe**, then reconnect. Existing accounts and encrypted profiles are preserved. Updater-enabled clients can check for updates in Settings; installation requires confirmation. The portable ZIP contains the same version. Warply itself has no Windows Authenticode signature; update artifacts have the required Tauri signature.

## Manual checks

1. Connect with the default DNS settings. Allow up to 30 seconds for the periodic check. Confirm **Connection verified**, then inspect the IPv4, IPv6 and DNS details.
2. Disconnect and confirm the service stops and the proof clears. Reconnect and confirm a fresh proof.
3. While disconnected, select another supported DNS provider, then reconnect and repeat verification.
4. Make the tunnel unreachable and confirm the app does not keep a green verified status. Restore connectivity and recheck.
5. Use local diagnostics to confirm network inspection works while connected. Do not reset the saved account to resolve this UI error.

Release assets: installer, matching .sig, portable ZIP, latest.json, SHA256SUMS.txt, README.md, PRIVACY.md and TEST-CHECKLIST.md. Never publish profiles, exported configurations or signing secrets.

Source: https://github.com/rookeudev/warply

License: Warply Source-Available License; see LICENSE.
