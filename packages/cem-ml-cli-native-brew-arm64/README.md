# CEM-ML native macOS ARM64/Homebrew deployment

This Nx deployment project owns only the `aarch64-apple-darwin` /
`native-macos-arm64` CEM-ML CLI artifacts. It builds the common
`packages/cem_ml_cli` binary without copying semantic implementation code.

The package target emits a deterministic versioned `.tar.gz`, target-qualified
SHA-256 metadata, a Syft-generated SPDX 2.3 SBOM, the common native capability
manifest, an unsigned build-provenance record, an immutable Homebrew channel
record and formula projection, and a release-index entry. The sign target uses
a deterministic ad-hoc hardened-runtime signature for ordinary local checks.
Authorized local release runs replace it with the EPA-WG Developer ID signature,
submit an exact binary copy in a transient ZIP to Apple's notary service, and
verify the signed and notarized CLI before publication.

```bash
yarn nx run cem_ml_cli_native_brew_arm64:build
yarn nx run cem_ml_cli_native_brew_arm64:package
yarn nx run cem_ml_cli_native_brew_arm64:verify
yarn nx run cem_ml_cli_native_brew_arm64:smoke:install
yarn nx run cem_ml_cli_native_brew_arm64:smoke:upgrade
yarn nx run cem_ml_cli_native_brew_arm64:smoke:uninstall
```

Generic CI intentionally skips this native executable. macOS ARM64 is excluded
from the current GitHub Release contract, and every job in the retained
[self-hosted native recipe](../../docs/cem-ml-native-release.md) is disabled.
The local Nx lifecycle remains available for platform development and package
validation only.

For native target development without publication, export the Apple
signing/notarization variables and run:

```bash
CEM_ML_RELEASE_SIGNING=required \
yarn nx run cem_ml_cli_native_brew_arm64:smoke:release
```

`preflight:release` and `publish` reject macOS while the platform is outside the
tested release-unit set. The generated Homebrew projection is validation output
only and is not published to `EPA-WG/homebrew-cem`.
