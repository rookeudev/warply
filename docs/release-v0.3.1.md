# Warply v0.3.1 — centered power button

Fixes the offset ring around the power button introduced in 0.3.0. The button was resized to 132px while its SVG stayed fixed at 120px. The ring now fills its parent and its center matches the button, icon and surrounding halo. It also follows future button-size changes. The decorative SVG does not intercept pointer events.

The production frontend was built and linted. Browser geometry checks passed for disconnected, connected, unverified, connecting and error states at device scale factors 1, 1.25, 1.5 and 2, plus a resized-button check. This is a CSS hotfix; the Rust tunnel logic was not changed and the Rust unit suite was not repeated. Native Windows display-scaling and full installer execution still need manual acceptance testing.

Install **Warply_0.3.1_x64-setup.exe** or check for updates in Settings. Accounts and encrypted profiles remain. Installation requires confirmation; the updater still verifies its Tauri signature and version. Warply itself has no Authenticode signature. No telemetry, backend, database or diagnostic uploads were added.

## Manual check

Connect/disconnect and watch the spinner while connecting. Confirm the ring, power icon, hover surface and outer halo remain concentric, including at Windows scaling of 125%, 150% and 200%. Check both themes and reduced motion. Confirm the button remains clickable and keyboard accessible.

Release assets: installer, matching .sig, portable ZIP, latest.json, SHA256SUMS.txt, README.md, PRIVACY.md and TEST-CHECKLIST.md.

Source: https://github.com/rookeudev/warply

License: Warply Source-Available License; see LICENSE.
