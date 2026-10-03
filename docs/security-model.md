# Security model — v0.3.0

Warply has additional safeguards, but this is not a claim of complete security or a full independent audit. Source secrecy is not a security control.

## Connection status

The WireGuard service state and the health result are separate fields. A running service alone never produces the green verified state in the UI or tray.

Every 30 seconds while the service runs, Rust checks `https://www.cloudflare.com/cdn-cgi/trace`. It validates the saved profile, confirms that the chosen source address is active on the `warply` adapter, and binds the HTTPS client to that address. The client has no proxy, no redirects, an eight-second request timeout, and a 4 KB response limit. Only an unambiguous `warp=on` or `warp=plus` response verifies WARP. Failed or changed responses remain unverified. No private key, public IP, or trace body is sent to the webview or logged.

Results expire after 30 seconds. Disconnect, reconnect, and network recovery invalidate previous results. A revision check prevents a pending request from verifying a later tunnel session. A failed check does not automatically restart the tunnel: a verification endpoint failure is not proof that the tunnel failed.

The probe prefers the profile's IPv4 address and can use IPv6 for an IPv6-only profile. It proves one HTTPS request through WARP, not every application's routing, all IPv6 routes, DNS leak prevention, or continuous connectivity. The UI displays the age of the last check. See [Cloudflare's trace documentation](https://developers.cloudflare.com/fundamentals/reference/cdn-cgi-endpoint/).

## Implemented protections

- Keys are generated locally; registration sends only the public key. Selected key, profile, and registration-token allocations are zeroized when released. This does not guarantee that every temporary or third-party copy is wiped.
- Cloudflare registration uses HTTPS, certificate validation, no redirects, no proxy, timeouts, and bounded 64 KB response bodies. Registration IDs and tokens are checked before use in a URL or header. There is no telemetry, database, or application backend.
- Imported profiles require one Interface and one Peer section, reject repeated fields and scripts, and validate keys, CIDRs, endpoint, DNS addresses, MTU, and numeric fields. This intentionally rejects multi-peer profiles and DNS search-domain entries.
- Reset requires native confirmation. A new account is registered and a replacement config is built before the old profile is replaced. A registration failure preserves the previous profile.
- Windows resolves LocalAppData through its known-folder API rather than an environment variable. Sensitive storage and export reject linked paths. Profile reads retain a file handle that denies writes/deletion; export retains an exclusive handle, checks the file type, and applies permissions before truncating or writing. Saves use a newly created temporary file and a final rename.
- Private config contents stay in Rust. Export requires a native private-key warning. The webview gets only status and non-secret settings.
- The webview is restricted to the app's local origin (the fixed Vite origin is allowed in debug builds). CSP blocks embedded frames, objects, form submissions, and remote frontend connections. Devtools are disabled.
- Official WireGuard downloads and installed binaries are checked against the WireGuard publisher before execution. The installer fallback retains its replacement protection.
- Updates use Tauri signatures, require a signed version, and ask for native confirmation. Release Actions are pinned to immutable commit SHAs; checkout credentials are not retained. Signing secrets stay outside Git. Windows Authenticode signing of Warply itself remains optional and is not configured.

## Remaining boundaries

The UI and backend still run elevated in one process. The saved `warply.conf` is plaintext with restricted ACLs; DPAPI encryption and a separate privileged helper have not been implemented. Path checks reduce risk but do not establish a fully race-free privileged filesystem design. A local administrator, compromised OS, or malware in the elevated process can access secrets and control networking.

Warply does not implement an additional custom kill switch. WireGuard for Windows has its own behavior for a single peer with `/0` routes; imported or custom configurations may differ. Normal disconnect removes the service and restores ordinary networking. Health checks are diagnostic, not a firewall guarantee.

## Verification and manual tests

Rust tests cover ambiguous trace responses, stale results, service/health separation, config injection and optional fields, source-address selection, failed reset preservation, secure-save replacement, installer checks, and existing setup/recovery behavior. Browser fixtures cover service-running/verified/unverified/disconnected states and menu navigation; they do not prove a live VPN session.

On a Windows VM, check a real working tunnel, block WARP traffic while leaving the service running, restore access, block only the trace endpoint, switch networks, disconnect during a pending check, and inspect IPv4/IPv6/DNS routing separately. Verify that a failed check never shows green and that the user can still disconnect. Test linked file rejection and reset cancellation/registration failure without losing the old profile.
