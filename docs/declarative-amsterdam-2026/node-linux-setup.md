# Amsterdam Node/Linux setup — checked 2026-11-03

The selected participant route is the supplied local Studio kit and its browser
exercise. The [handout](participant-setup.md) describes cloning, editing,
validation, offline reopening and credentials. Keep
[discussion #24](https://github.com/EPA-WG/cem/discussions/24) as the stable
resource bookmark. No public kit download or registry installation is advertised.

## Prepare Node on Linux x64

The exact tested Node archive is
[`node-v24.16.0-linux-x64.tar.xz`](https://nodejs.org/dist/v24.16.0/node-v24.16.0-linux-x64.tar.xz).
It was downloaded, checked against the
[official SHA-256 list](https://nodejs.org/dist/v24.16.0/SHASUMS256.txt), extracted
into a new temporary directory and executed without changing the installed Node.
This pins the tested version; it does not claim it is the latest Node release.

Use a new preparation directory on a Linux x64 machine. Download the archive
and checksum list above there, then run:

```sh
sha256sum --check --ignore-missing SHASUMS256.txt
tar -xJf node-v24.16.0-linux-x64.tar.xz
./node-v24.16.0-linux-x64/bin/node --version
```

Require `node-v24.16.0-linux-x64.tar.xz: OK` and `v24.16.0`. The archive SHA-256 is
`d804845d34eddc21dc1092b519d643ef40b1f58ec5dec5c22b1f4bd8fabde6c9`
(31,428,548 bytes). Checking hashes establishes byte identity, not a separate
signature/provenance verification.

Unpack the presenter-supplied `cem-studio-login-kit.tgz` into that preparation
directory. From inside its `cem-studio-login-kit` folder run:

```sh
../node-v24.16.0-linux-x64/bin/node start.mjs
```

Open the printed loopback origin and follow the kit's README. Keep the origin,
port and browser profile stable so saved projects and drafts remain available.
No npm install, Rust, Yarn, compiler or CEM CLI is needed for this route.
Prepare Node, the browser and the first offline cache before the timed session.
A cold machine without those prerequisites is not an offline installation route.

The tested host is Ubuntu 26.04 LTS x64 under WSL2, glibc 2.43. This is an
existing host with a fresh Node extraction, not a clean OS installation. Consult
[Node's versioned platform requirements](https://github.com/nodejs/node/blob/v24.16.0/BUILDING.md#platform-list)
for another host; upstream support alone does not establish this exercise's
acceptance there. Native Windows, macOS, Linux ARM64 and other browser/device
combinations remain untested. Use the presenter-led path and saved checkpoint
outputs when a participant's machine is outside the verified route.

The complete extracted-kit acceptance passed with this freshly downloaded Node:
20 Login and three Supplier configurations, live interaction, failure/repair,
independent copies and offline draft reload. The unchanged 417-file archive and
Chromium 148.0.7778.96 are pinned in the
[dated verification record](participant-kit-verification.md#fresh-node-download-and-full-kit-repeat--2026-11-03).

## CEM CLI availability and exercise compatibility

The public [CEM-ML 0.1.0-rc.2 prerelease](https://github.com/EPA-WG/cem/releases/tag/cem-ml-v0.1.0-rc.2)
was published on 2026-08-18 from `f85b7703b2a7f8be2746824763ac4fdd8c911d6b`.
Its Linux AMD64 GNU archive and paired universal CLI/WASM npm tarballs download
successfully and match their GitHub asset digests. The
[machine-readable readback](installation-readback.json) pins the URLs, sizes,
SHA-256 hashes, commands and observed outputs.

| Route checked | Actual result | Participant decision |
| --- | --- | --- |
| Public npm registry: `@epa-wg/cem-ml-cli` and `@epa-wg/cem-ml` | Both metadata endpoints returned HTTP 404 on 2026-11-03. | Do not advertise `npm install` from the public registry. |
| GitHub Node/WASM tarballs | Both 0.1.0-rc.2 packages installed together offline into a temporary consumer with a fresh npm cache and scripts disabled. | Download/install success is established; complete Login exercise parity is not. |
| GitHub Linux AMD64 GNU tarball | Extracted outside the workspace; `cem-ml --version` returned `0.1.0-rc.2`. | Binary startup is established; complete Login exercise parity is not. |
| Frozen Login 2.8.1, first `read-edit` checkpoint on both CLIs | Exit 0, but output hash differs from the frozen AST projection. Both emit a formatted source document instead. | Stop acceptance at this mismatch; use the verified Studio kit. |

The native diagnostic bytes match the frozen first checkpoint. Node diagnostics
also use absolute file URIs, so their exact hash differs; no URI normalization
was used to claim a match. Later CLI checkpoints were not run after the first
output mismatch. This establishes incompatibility with the frozen output
contract, not that every prerelease operation fails.

These checks did not install system packages, publish a release, access user
browser sessions, clear storage or alter credentials. Entered passwords/OTP
still block recovery until explicitly cleared. Automatic whole-Studio recovery
remains disabled; the 4 MiB aggregate budget and five-second deadline remain.
