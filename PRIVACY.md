# Privacy — Warply 0.2.1

## What Warply collects

Warply contains no analytics or telemetry SDK, advertising identifier, usage history, browsing-history collection, crash upload, application backend, or database. It does not send private keys, configuration contents, files, usernames, or computer names to its developer. There is no persistent Warply diagnostic log and no automatic GitHub request unless the user enables automatic update checks.

Local storage contains the encrypted WireGuard profile, non-secret startup/network/protection/update/language settings, and the frontend's language/theme preferences. The current connection result and check age are held in memory. Registration credentials returned by Cloudflare are not persisted. Selected secret allocations and the trace-response buffer are wiped on release; this cannot cover every third-party or allocator copy.

## Necessary external communication

| Recipient | When | Data/traffic |
| --- | --- | --- |
| Cloudflare registration API (`api.cloudflareclient.com`) | First setup or explicit account reset | Public WireGuard key, terms-acceptance timestamp, and fixed protocol compatibility fields. Push token, installation ID, and serial-number fields are empty. Cloudflare sees the requesting IP. The private key is never uploaded. |
| Cloudflare WARP (`engage.cloudflareclient.com:2408`, or an explicitly selected endpoint) | Tunnel is ON | WireGuard traffic. Cloudflare is the VPN provider and processes the traffic it carries. |
| Cloudflare trace (`www.cloudflare.com/cdn-cgi/trace`) | About every 30 seconds while the service runs, or a manual check | Separate IPv4 and IPv6 HTTPS requests bound to their tunnel addresses. The response is parsed locally for WARP status; IP/trace contents are neither stored nor sent to the webview or developer. |
| Configured DNS resolver | When Windows needs DNS | DNS queries under the selected configuration. This includes resolving network endpoints. |
| GitHub and its release-asset hosts | User requests update check/install, or enables daily checks | App update protocol and download requests. The provider sees network metadata and its updater user agent. No usage history or personal payload is added. |
| Official WireGuard download server, or Microsoft winget infrastructure | User approves WireGuard installation | Package lookup/download/installation requests. |

Opening the project or download page explicitly launches the user's browser, whose privacy settings apply. Windows, WebView2, WireGuard, Cloudflare, GitHub, DNS, and winget have their own behavior and policies. Warply cannot promise that those products collect no data. A VPN also cannot make all third-party websites anonymous.

## Private configuration

The main profile is protected with **user-scoped DPAPI**, bound to the Windows identity and usually the computer. Moving it to another account or PC is not a supported backup/restore method. Use explicit Export for a portable backup, and protect that plaintext backup yourself.

The separate WireGuard service copy uses **machine-scoped DPAPI** so LocalSystem can read it. It is stored under `%ProgramData%/WarplyService`. It is protected by a SYSTEM/Administrators-only file ACL and removed after a successful tunnel shutdown. Anyone who obtains that machine-scoped blob on the same machine may decrypt it; never upload it or treat encryption as a substitute for its ACL.

Old app-managed plaintext profiles are removed only after verified encryption succeeds. This does not remove earlier exports, imported source files, backups, or forensic remnants. Never attach those files to GitHub issues. No app can protect its secrets from an already compromised OS, a privileged attacker, or malware using the same account while it is unlocked.

## Manual privacy/security checklist

- Upgrade a copy of a previous profile in a Windows VM: account stays the same; `warply.conf` disappears after migration; `warply.profile` contains no readable private key.
- Connect: service uses `warply.conf.dpapi`; no plaintext temp config appears. Disconnect/quit: encrypted service copy is removed. Repeat after reboot and network recovery.
- Confirm the service-copy DACL contains SYSTEM and Administrators only, and the user profile has only the current user and SYSTEM. Verify access denial from another ordinary account.
- Corrupt a protected profile: setup stops, it is not overwritten, and registration is not attempted. Keep a backup before this test.
- Monitor network requests: disconnected second launch performs no registration, trace, or GitHub update request. Explicit update checks contact only GitHub release infrastructure; enabled tunnels perform their documented probes.
- Export requires confirmation and produces plaintext only at the explicitly selected destination. Hard-linked or reparse destinations are refused.

See [Microsoft DPAPI documentation](https://learn.microsoft.com/en-us/windows/win32/api/dpapi/nf-dpapi-cryptprotectdata) and [security boundaries](security-model.md).

## Local diagnostics and protection

The diagnostic report is generated locally on demand. It includes version, service/protection/health states, check age, routing/DNS check booleans, and a detected VPN count. It excludes keys, configurations, IP addresses, usernames, computer names, network names, and raw command output. Copying it requires an explicit button; Warply never uploads it. Clipboard contents are subject to Windows and other installed software.

The privileged helper exchanges bounded local pipe messages and keeps no diagnostic log. An admin-only endpoint cache stores a hostname and numeric transport address, never keys or account tokens. Kill-switch rules and opt-in update timestamps are local state. Automatic update checks are disabled by default, limited to one per day when enabled, and never automatically install a release. Notifications use local Windows notifications.
