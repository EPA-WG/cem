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

## Supplier refresh and advertised entry URLs — 2026-11-01

The refreshed kit also includes the frozen Supplier 1.1.1 project: all six
configurations plus XML repair and zero/invalid/restored stock (ten native
checkpoints). All three generated formats and original diagnostics replayed
exactly. Seven Supplier tests, 12 existing Login tests, Login's 21 unchanged
native results and package lint passed after extracting shared project helpers.

The original handout used `collection=amsterdam`, although the catalog's actual
ID is `amsterdam-2026`. The initial browser workflow used legacy workbench choices
and did not validate that printed collection parameter. The handout and Login
walkthrough now use the real ID. Acceptance now cold-opens both advertised
README routes while offline; separate root-path checks also passed for the exact
`/?collection=amsterdam-2026&sample=login&file=readme` and corresponding Supplier
URL. The generated `participant-entry-routes.json` records those root-path checks.

The extracted kit again completed all 23 selected offline configurations,
independent copies, diagnostic selection, live preview and unsaved-draft reload.
Those browser scenarios include three of Supplier's six configurations; the
native freeze covers all six. Existing user sessions were never opened or cleared.

| Refreshed artifact evidence | Observed value |
| --- | --- |
| Preparation baseline | `ee3731757e2864334ef2bb18f67c92ea4ed37a3b` |
| Frozen Supplier inputs | `ee3731757e2864334ef2bb18f67c92ea4ed37a3b`, version 1.1.1 |
| File count | 417 |
| Archive SHA-256 | `2b8cec8f3cebe74c3a705313f9fc8b815056e614a1bd707a336c9b550ccfdf53` |
| Host | v24.16.0, linux/x64, Chromium 148.0.7778.96 |

The archive path remains `dist/packages/cem-studio-login-kit.tgz`. The original
386-file record above is historical; the refreshed artifact has 417 files.
The local Node launcher, credential policy and public-availability limits remain
the same. This record does not expand the tested operating-system matrix.


## Release-claim guards — 2026-11-03

The [retained machine-readable receipt](participant-kit-readback.json) preserves
exactly the refreshed 417-file report described above. It records a historical
acceptance run, not a new browser run or the identity of a later rebuilt archive.
The participant-kit verifier now validates its report before writing it; the
participant-kit test target also checks the retained receipt against the recorded
archive, source revision and tested Node/Linux/Chromium versions.

Negative fixtures reject HTTP-only reachability, CLI version smoke, mismatched
archive/source/platform/browser identities, incomplete configuration and offline
coverage, lost drafts/copy isolation, incorrect entry routes and weakened
credential policy. Successful local exercise evidence still leaves public
availability, fresh installation and other platforms unverified. A compatible
Node engine range does not establish that each version was installed or tested.
These guards validate recorded evidence; only running the extracted kit's complete
browser verifier establishes a new exercise result.

Verification: `yarn nx run @epa-wg/cem-studio:test:participant-kit` (20 tests)
and `yarn nx run @epa-wg/cem-studio:lint` passed on 2026-11-03 using the
existing Linux Node v24.16.0 toolchain. No fresh installation was performed.


## Fresh Node download and full kit repeat — 2026-11-03

The [Node/Linux setup guide](node-linux-setup.md) now supplies an exact Node
archive and preparation commands. Node 24.16.0 Linux x64 was downloaded from
nodejs.org, checked against its official SHA-256 list and extracted into a new
temporary directory. Its actual executable ran the existing extracted-kit
verifier against the unchanged 417-file archive with SHA-256
`2b8cec8f3cebe74c3a705313f9fc8b815056e614a1bd707a336c9b550ccfdf53`.

The complete local production-hosting and offline Login/Supplier acceptance
passed again: 20 Login plus three Supplier configurations, failure/repair,
inspection/formatting, graph provenance, live interaction, independent copies,
URL/tree agreement and unsaved Unicode draft restoration. Inventory checks pass
before and after browser use. Chromium is 148.0.7778.96; the generated report
exactly matches the [retained receipt](participant-kit-readback.json).

The [installation readback](installation-readback.json) records the Node/archive
hashes, runner command and actual host: Ubuntu 26.04 LTS x64 under WSL2 with
glibc 2.43. The helper was invoked directly to preserve the exact accepted kit;
Nx packaging prerequisites would rebuild a different archive. This verifies a
fresh Node extraction on an existing host, not a clean OS or other-platform
installation. Public npm metadata returns 404 for both expected CEM packages;
the downloaded GitHub prerelease CLIs do not match the frozen first Login AST
output. They remain outside the participant route. No kit rebuild, publication,
user-data clearing or credential-policy change was performed.

The presentation resource verification also passed through Nx: 45 guard tests,
12 presenter cues, 30 recorded native slide IDs and three pinned source links.
