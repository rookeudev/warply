# Warply v0.2.6 — client hardening and faster checks

## Security

- Update packages must match this repository, the manifest version, the exact release tag and the Windows x64 installer filename. Alternate filenames, mismatched tags/versions and query parameters are rejected. Mandatory signed-version and Tauri signature checks remain enabled.
- Rust serializes update checks and installation checks. Repeated requests cannot start concurrent update checks.
- Cloudflare authorization headers are marked sensitive, and the temporary bearer string and registration ID are zeroized after use. This reduces accidental exposure; it cannot erase every copy inside Windows or third-party libraries.
- Existing user-scoped DPAPI, restricted ACLs, authenticated privileged helper, source-bound dual-stack checks, CSP and native export confirmation remain in place. No telemetry, analytics, backend, database or diagnostic uploads were added.

## Performance

- IPv4 and IPv6 verification requests run concurrently. This avoids adding their request durations together; no fixed speedup is promised.
- A verification pass loads/decrypts the profile once and reuses it for local network inspection.
- The frontend shares one pending status read across timer and visibility refreshes. An older failed poll cannot override a newer successful action's snapshot.

## Useful connection details

An unverified connection now shows its specific failed check: local Windows inspection, routing, DNS, conflicting VPN, Cloudflare WARP result, IPv4 or IPv6. Messages are available in English and Czech. Pending/expired checks do not display an old failure reason. A service running by itself still does not count as a verified tunnel.

## Verification

43 Rust tests passed; 3 opt-in integration tests were not run in this release. cargo fmt, cargo clippy, ESLint and the production build passed. Browser fixtures checked refresh coalescing during a burst of visibility events, failure explanations, partial-proof safety, recovery messages and the existing connection cancellation/recovery flows.

The live tunnel was not connected during this release's checks. The 0.2.5 live verification remains historical evidence, not a new 0.2.6 traffic test. Full connected protection, installer and crash/reboot behavior still requires Windows acceptance testing. This release is incremental hardening, not a guarantee against malware running as the same user or an administrator, and not an independent security audit.

## Install and manual checks

Install **Warply_0.2.6_x64-setup.exe** or check for updates in Settings. Installation requires confirmation. Existing accounts and profiles are retained; no reset is needed. Warply itself has no Authenticode signature; the updater artifacts have the required Tauri signature.

1. Connect and wait for IPv4/IPv6 verification; check the detailed network states.
2. Disconnect during verification, then reconnect. A previous session must never produce a green result for the new session.
3. While disconnected, change DNS and reconnect. Verify DNS mismatch disappears when the configured DNS is applied.
4. Block the trace endpoint and confirm the appropriate IPv4/IPv6 explanation and non-green status. Restore access and recheck.
5. Repeatedly show/hide the window, then open Settings and run local diagnostics.
6. Check for updates and cancel the native installation confirmation. The existing connection must remain intact.

Release assets: installer, matching .sig, portable ZIP, latest.json, SHA256SUMS.txt, README.md, PRIVACY.md and TEST-CHECKLIST.md. Never publish profiles, exported configurations or signing secrets.

Source: https://github.com/rookeudev/warply

License: Warply Source-Available License; see LICENSE.
