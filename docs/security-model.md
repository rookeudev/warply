# Security model — current audit

The requested security-hardening implementation is pending. This document records the current risks; it does not claim that the visual redesign solves them. The repository may be public. Source secrecy and obfuscation are not security controls.

## Threat boundaries

Cloudflare API responses, imported configs, the webview and frontend state, environment variables, downloaded installers, and filesystem paths must be treated as untrusted. An administrator or compromised operating system can read or change application memory and tunnel settings. Warply cannot protect against that attacker.

The current app runs its webview and Rust backend elevated. This gives a webview compromise a larger impact than a separated, unprivileged UI and privileged helper. Existing frontend commands use a fixed command set and do not accept executable paths, but this is not process isolation.

## Prioritized findings

| Priority | File                                                               | Finding                                                                                                                                                                                                             |
| -------- | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| High     | `src-tauri/src/storage.rs`                                         | Plaintext `warply.conf` remains in LocalAppData. ACLs include the current user. DPAPI and an Administrators/SYSTEM-only tunnel directory have not been implemented.                                                 |
| High     | `src-tauri/src/keys.rs`, `config.rs`, `setup.rs`                   | Private keys and config text use ordinary strings. They are not guaranteed to be zeroized, and secret wrapper types are absent.                                                                                     |
| High     | `src-tauri/src/lib.rs`, `elevation.rs`                             | The whole UI runs elevated. Development builds relaunch the current executable. A production release must enforce installation and execution from a protected Program Files location.                               |
| High     | `src-tauri/src/tunnel.rs`, `installer.rs`, `storage.rs`            | Some paths derive from environment variables. Reparse-point, protected-path, and replacement-race checks need a consistent review for all elevated file and process operations.                                     |
| High     | `.github/workflows/release.yml`                                    | Releases are unsigned and have no secret, dependency, or provenance gates. Action references are not pinned to immutable commit SHAs. This workflow is not ready for the requested signed-only distribution policy. |
| Medium   | `src-tauri/src/config.rs`, `warp_api.rs`                           | Parsing rejects script hooks and validates keys/CIDRs, but several settings remain unchecked, duplicate/multiple-section semantics are weak, and API response sizes and fields need stricter bounds.                |
| Medium   | `src-tauri/tauri.conf.json`, `capabilities/default.json`, `lib.rs` | Explicit navigation restrictions, release webview restrictions, and single-instance enforcement remain pending.                                                                                                     |
| Medium   | `src-tauri/Cargo.toml`, `vite.config.ts`                           | Release mitigations, binary inspection, and artifact secret scans are not configured.                                                                                                                               |
| Low      | `.gitignore`                                                       | `.env`, key, and certificate patterns and CI secret scanning need to be added.                                                                                                                                      |
| Low      | Network constants and release documentation                        | Outbound requests are spread across modules. The permitted endpoints and disconnect behavior should be documented centrally.                                                                                        |

## Existing protections and limits

- Private keys are generated locally. The registration request sends the public key; the webview receives status fields rather than config contents. Import and export use native dialogs. Export displays a private-key warning.
- Sensitive registration requests use HTTPS, certificate validation, timeouts, and no redirects. No telemetry or crash uploader is implemented.
- Script hooks and unknown config directives are rejected. This does not substitute for complete field and section validation.
- The official MSI fallback checks Authenticode status and the exact WireGuard publisher, while locking the file against replacement until installation finishes. Installed WireGuard binaries are signature checked before tunnel commands.
- These controls do not provide encrypted storage or prevent access to secrets by an administrator, malware in the elevated process, a debugger, or a compromised OS.
- No custom kill switch, public-IP check, scheduled autostart task, or updater is currently implemented. Disconnect removes the WireGuard service and restores ordinary networking; traffic after disconnect is not protected by the tunnel.

## Review checkpoints

The hardening brief requires implementation in order and review after security steps 2 and 5. Neither checkpoint has been reached. The subsequent visual brief is being handled as visual step 1; it preserves the tunnel and registration behavior. Background operation remains a separate requested review step.
