# CEM-ML self-hosted native release recipe (deferred)

**Status:** Deferred 2026-08-17.

The CEM-ML GitHub Release contract contains exactly three CI-owned units:

- `@epa-wg/cem-ml` WASM/npm artifacts;
- `@epa-wg/cem-ml-cli` WASM/npm CLI artifacts;
- `native-linux-amd64` archives and Debian/APT artifacts.

macOS ARM64/Homebrew and Windows AMD64 are not GitHub Release units. Their Nx
build, package, sign, verify, and lifecycle-smoke targets remain available for
local platform development, but their release preflight and immutable uploader
reject publication while they are outside the release-unit contract.

`.github/workflows/cem-ml-native-release.yml` retains the reviewed self-hosted
producer/publisher recipe for reference. Every job is hard-disabled with
`if: ${{ false }}`, so a manual dispatch schedules no self-hosted work. Do not
register release runners or populate Apple/Windows release credentials for this
deferred workflow.

Linux AMD64 production and optional exact-tag recovery remain in the protected
`.github/workflows/cem-ml-release.yml` workflow. That workflow creates or
resumes the draft, builds the two WASM/npm units and Linux unit, records their
attestations and producer evidence, and uploads only missing immutable bytes.

Re-enabling either self-hosted platform requires an explicit future roadmap
decision. That change must add the platform identity back to the tested GitHub
Release unit set, restore the aggregate collector/finalizer expectation, remove
the corresponding workflow job gates, configure protected signing authority,
and record a successful release rehearsal. Merely removing `if: ${{ false }}`
is insufficient.
