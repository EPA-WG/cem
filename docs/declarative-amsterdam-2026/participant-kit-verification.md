# Local participant kit verification — 2026-11-01

The selected local-transfer route passed acceptance after extraction outside the
workspace. Participants need the supplied archive, an already installed Node
runtime and a browser; the launcher installs no application dependencies.
The [participant handout](participant-setup.md) gives the steps. This record does
not establish a public Studio URL, registry release or fresh operating-system /
Node installation. Other platforms remain unverified.

| Evidence | Observed value |
| --- | --- |
| Preparation baseline | `d565fb98ad49625ef1b7a992090d2928db32256a` |
| Frozen Login input/runtime baseline | `c5e5b89274175997387a4d0e160a7e6f6262147f`, Login 2.8.1 |
| Node | `v24.16.0` |
| Host | `linux` / `x64` |
| Playwright Chromium | `148.0.7778.96` |
| Archive | `dist/packages/cem-studio-login-kit.tgz`, 16,100,802 bytes |
| File count | 386 |
| Archive SHA-256 | `52f92ebc04ec4c4087d61675f7082951d7bfbaca0d43c809f5e2816b34cfa946` |
| Hosting | Original production index, security headers and service-worker scope |
| Application dependency installation | None in the extracted kit |
| Selected offline configurations | 20 Login + 3 Supplier |

The preparation baseline identifies the checkout used to assemble this dated kit;
its inventory hashes pin the actual launcher, static assets and reference files.
Rebuilt archives get their own checksum in the generated verification report.
The archive is a local preparation artifact, not a signed public release.

## Checks completed

- Five launcher tests passed: production hosting without a build report, original
  development-preview URL versioning, same-size modified bytes, missing files and
  unexpected files. WASM MIME, CSP headers and path confinement were checked.
- The actual extracted `node start.mjs` served Studio through a loopback port.
  The deployed verifier used that server, not the workspace deployment server.
- A fresh browser context completed the Login source edits, original diagnostic
  selection and repair, queries/formatting, graph/provenance, light-DOM preview,
  independent copies, offline saved results and unsaved Unicode draft reload.
- The Supplier companion completed its offline multi-output failure/repair and
  independent-copy checks. This verifies its current route; selecting and freezing
  the companion's exact presentation checkpoints remains separate work.
- The complete kit inventory matched before and after browser use. Existing user
  browser sessions, saved projects and drafts were never opened or cleared.
- Package lint and the packaged handout/link checks passed.

Credential privacy remains unchanged: entered passwords/OTP block recovery until
explicitly cleared, automatic whole-Studio recovery stays disabled, and aggregate
checkpoint limits stay at 4 MiB with a five-second handoff deadline. The kit copies
the already accepted runtime bytes; it adds no application UI JavaScript or
credential persistence. Frozen references contain checked-in project data only.

## Reproduce

```sh
yarn nx run @epa-wg/cem-studio:package:participant-kit
yarn nx run @epa-wg/cem-studio:verify:participant-kit
```

Nx prepares and verifies the deployment and frozen inputs first. The second
target extracts a disposable consumer directory, starts its actual Node launcher,
runs offline browser acceptance, verifies the unchanged kit and removes only its
own temporary directory. It records exact artifact/host evidence in
`dist/reports/cem-studio/participant-kit.json`. Initial toolchain/browser installation
is a preparation prerequisite; it is not covered by the offline exercise claim.
