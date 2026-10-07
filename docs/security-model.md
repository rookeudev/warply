# Security model — v0.2.6

Warply has additional safeguards, but this is not a claim of complete security or a full independent audit. Source secrecy is not a security control.

## 0.2.6 client hardening

Update URLs must match the manifest version, exact repository release tag and x64 installer filename in addition to the existing signed-version and signature validation. Concurrent update checks are rejected in Rust. Registration authorization headers are marked sensitive and temporary bearer/ID allocations are zeroized; third-party copies may still exist. Status polling shares pending requests and ignores outdated poll failures. Dual-stack probes run concurrently after local ownership checks and reuse one decrypted profile. Failed verification has a limited, non-secret reason. See [0.2.6 release notes and acceptance checks](release-v0.2.6.md).

## Connection status

The WireGuard service state and the health result are separate fields. A running service alone never produces the green verified state in the UI or tray.

Every 30 seconds while the service runs, Rust checks `https://www.cloudflare.com/cdn-cgi/trace` separately over IPv4 and IPv6. Each HTTPS client is bound to the corresponding profile address on the `warply` adapter, resolves only destination addresses of the same family, disables proxies and redirects, and limits requests to eight seconds and responses to 4 KB. Both responses must unambiguously report `warp=on` or `warp=plus`. Local inspection must also confirm the selected IPv4/IPv6 routes, configured adapter DNS addresses, and absence of detected competing VPN adapters. Unknown inspection results cannot verify the connection.

Results expire after 30 seconds. Disconnect, reconnect, and network recovery invalidate previous results. A revision check prevents a pending request from verifying a later session. Failure of the trace endpoint alone does not restart an otherwise running service. No private key, public IP, or trace body is sent to the webview or logged.

These checks prove two recent HTTPS requests and selected local routing/DNS configuration. They do not certify every application, all destinations, or external DNS leak prevention. VPN conflict detection is a conservative adapter-description heuristic and cannot identify every VPN. See [Cloudflare's trace documentation](https://developers.cloudflare.com/fundamentals/reference/cdn-cgi-endpoint/).

## Privilege boundary and persistent protection

The GUI runs with the normal desktop user's token. A headless elevated helper performs fixed tunnel, installation, and Windows Filtering Platform operations. One UAC approval is needed per app/helper session; toggles reuse that helper. The local named pipe rejects remote clients, restricts its ACL, limits frames, rejects unknown operations and extra fields, and verifies the exact live helper PID returned by Windows elevation and the executable image. The helper verifies its unelevated parent and server PID. Requests cannot select arbitrary commands, paths, or executables. A broken helper connection stops automatic privileged work instead of repeatedly opening UAC prompts.

The optional kill switch installs persistent WFP rules for IPv4 and IPv6, including early boot outbound blocking. Updates replace only Warply's fixed filter identities in an atomic transaction. It permits the verified tunnel interface, loopback, LocalSystem's verified WireGuard executable for UDP transport, DHCP port pairs, and necessary IPv6 neighbor/router discovery. These explicit exceptions mean this is not a promise that every packet is blocked. Rules survive a GUI/helper crash and reboot. Reconciliation refreshes the tunnel interface identity on the next approved session. Protection remains active during automatic recovery and unsuccessful connection attempts. Explicit OFF, Restore internet, normal quit, or successful uninstall removes Warply's rules; other firewall rules are preserved. The emergency `warply.exe --restore-internet` action also works without WireGuard.exe, by stopping/deleting only Warply's fixed service through Windows service control.

A protected endpoint cache stores only the hostname and numeric transport address in ProgramData. Protected reconnection uses this cache instead of allowing a broad DNS bootstrap exception. An unavailable/stale endpoint can require Restore internet and a fresh connection.

## Implemented protections

- Keys are generated locally; registration sends the public key and required protocol fields, never the private key. Selected key, profile, and registration-token allocations are zeroized when released. This does not guarantee that every temporary or third-party copy is wiped.
- Cloudflare registration uses HTTPS, certificate validation, no redirects, no proxy, timeouts, and bounded 64 KB response bodies. Registration IDs and tokens are checked before use in a URL or header. There is no telemetry, database, or application backend.
- Imported profiles require one Interface and one Peer section, reject repeated fields and scripts, and validate keys, CIDRs, endpoint, DNS addresses, MTU, and numeric fields. This intentionally rejects multi-peer profiles and DNS search-domain entries.
- Reset requires native confirmation. A new account is registered and a replacement config is built before the old profile is replaced. A registration failure preserves the previous profile.
- The main `warply.profile` is encrypted with user-scoped Windows DPAPI and an application context. There is no application encryption key in the repository. Failed decryption or integrity checks stop setup without replacing the saved profile or registering another account.
- WireGuard runs as LocalSystem, so it receives a separate machine-scoped DPAPI `%ProgramData%/WarplyService/warply.conf.dpapi`, with the `warply` description expected by its loader. Its protected ACL grants only SYSTEM and elevated Administrators. This copy exists while the tunnel service needs it and is removed after successful disconnect, quit, or updater shutdown. Startup removes a stale copy when the service is stopped. Machine scope is weaker than user scope if someone obtains the blob; its ACL is an essential part of protection. See [WireGuard's configuration format](https://git.zx2c4.com/wireguard-windows/plain/docs/enterprise.md).
- Existing plaintext profiles are validated, encrypted, read back, and compared before the original is removed. A running legacy service is stopped first and reconnected with the encrypted copy. Migration uses no plaintext temporary file. Deletion cannot erase existing backups or guarantee erasure of old SSD sectors.
- Windows resolves LocalAppData through its known-folder API rather than an environment variable. Sensitive storage and export reject symlinks, reparse paths, hard-linked files, network/device paths, alternate data streams, and ambiguous names. Profile reads retain a file handle that denies writes/deletion; export retains an exclusive handle, checks the file type, and applies an exact protected DACL on the held handle before truncating or writing. ACL replacement uses one native Windows API call rather than a sequence of command-line resets. Saves use a newly created temporary file and a final rename through the held handle, without reopening the protected temporary file.
- Private config contents stay in Rust. Export requires a native private-key warning. The webview gets only status and non-secret settings.
- The webview is restricted to the app's local origin (the fixed Vite origin is allowed in debug builds). CSP blocks embedded frames, objects, form submissions, and remote frontend connections. Devtools are disabled. The frontend has no Tauri core or updater-plugin capability permissions; registered Rust commands remain available to the local main window. No arbitrary path, shell command, or URL is accepted from the frontend.
- Official WireGuard downloads and installed binaries are checked against the WireGuard publisher before execution. The installer fallback retains its replacement protection.
- Updates use Tauri signatures, require a signed version, and ask for native confirmation. Release Actions are pinned to immutable commit SHAs; checkout credentials are not retained. Update checks are manual by default; an explicit opt-in allows at most one automatic check per 24 hours. Installation still requires confirmation. Proxies are disabled, and HTTPS redirects are limited to GitHub release hosts. Installer URLs must identify this repository's release assets. Signing secrets stay outside Git. Windows Authenticode signing of Warply itself remains optional and is not configured.

## Remaining boundaries

DPAPI and ACLs protect storage, but malware running as the same Windows account can use its DPAPI identity. A local administrator or compromised OS can take ownership, inspect memory, change WFP rules, or replace privileged software. Explicit exports and imported source files remain plaintext outside app storage. Selected secret allocations are zeroized; this cannot guarantee removal of every allocator or third-party copy.

Health checks are diagnostic. Persistent filtering needs real Windows traffic/crash/reboot testing; an aborted API-validation transaction proves that Windows accepts the rules, not their full runtime effect. There has been no complete independent security audit.

The installer preserves the previous executable during replacement and restores it on reported installer failure. It does not back up keys/configs and cannot guarantee rollback after power loss; a leftover `warply.previous.exe` supports manual recovery. Signed downloads are verified before tunnel shutdown. If installer launch fails, the app attempts to resume its previous connection; after successful installer launch, the app exits and cannot observe every external installer failure.

## Verification and manual tests

Rust tests additionally cover DPAPI round trips and tampering, verified migration, preservation on corrupt encrypted storage, hard-link rejection, and update URL restrictions. Existing tests cover ambiguous trace responses, stale results, service/health separation, config injection and optional fields, source-address selection, failed reset preservation, secure-save replacement, installer checks, and existing setup/recovery behavior. Browser fixtures cover service-running/verified/unverified/disconnected states and menu navigation; they do not prove a live VPN session.

On a Windows VM, check a real working tunnel, block WARP traffic while leaving the service running, restore access, block only the trace endpoint, switch networks, disconnect during a pending check, and inspect IPv4/IPv6/DNS routing separately. Verify that a failed check never shows green and that the user can still disconnect. Test linked file rejection and reset cancellation/registration failure without losing the old profile.

The 0.2.1 checks also cover dual-stack verification requirements, strict helper request/frame parsing, and UI protection/recovery controls. See [the full Windows acceptance checklist](0.2.1-test-checklist.md).

## 0.2.2 storage correction

Held handles used to update protected directory/file DACLs now request READ_CONTROL as well as WRITE_DAC, avoiding Windows access-denied failures. Machine service folders keep SYSTEM/Administrators scope throughout secure saves instead of being temporarily passed through user-folder protection. A normal-user regression test protects both a directory and file and repeats the operation. Native helper preflight also checks the actual protected service folder. No account reset or profile deletion is required.

## 0.2.4 cleanup and diagnostic corrections

Uninstall uses a dedicated already-elevated headless cleanup action, with distinct exit codes for service, WFP, storage, and elevation failures. Cleanup attempts Warply's rule removal even if service shutdown fails, but retains the service copy when the service could still need it. Removing an absent service copy never creates/reprotects a directory. Unknown protection after a helper failure is reported as unknown; it is not presented as confirmed inactive or active.

Interactive repair upgrades from installed 0.2.1 restart the same new installer with /UPDATE, avoiding the old uninstaller's READ_CONTROL defect. This mechanism was tested in an isolated NSIS callback fixture against the installed 0.2.1 registration, without replacing the installed application. Native cleanup was tested twice with no installed tunnel service and the encrypted user profile hash remained unchanged. These checks do not replace a full connected/install/uninstall/reboot VM acceptance run.

Local diagnostics now produce partial reports for missing/unreadable profiles, unavailable network inspection, and unknown service/protection state. No private configuration or raw error output is included. Explicit `warply.exe --diagnostics` prints the same limited local report without opening the GUI; it does not elevate or perform registration. A fresh diagnostic run clears the old report before starting, so a failed run cannot leave a stale report available for copying.
