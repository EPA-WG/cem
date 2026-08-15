# CEM-ML native macOS ARM64/Homebrew deployment

This Nx deployment project owns only the `aarch64-apple-darwin` /
`native-macos-arm64` CEM-ML CLI artifacts. It builds the common
`packages/cem_ml_cli` binary without copying semantic implementation code.

The package target emits a deterministic versioned `.tar.gz`, target-qualified
SHA-256 metadata, a Syft-generated SPDX 2.3 SBOM, the common native capability
manifest, an unsigned build-provenance record, an immutable Homebrew channel
record and formula projection, and a release-index entry. The sign target uses
a deterministic ad-hoc hardened-runtime signature in local/PR builds. Protected
release jobs replace it with the EPA-WG Developer ID signature, submit an exact
binary copy in a transient ZIP to Apple's notary service, and verify the signed
and notarized CLI before publication.

```bash
yarn nx run cem_ml_cli_native_brew_arm64:build
yarn nx run cem_ml_cli_native_brew_arm64:package
yarn nx run cem_ml_cli_native_brew_arm64:verify
yarn nx run cem_ml_cli_native_brew_arm64:smoke:install
yarn nx run cem_ml_cli_native_brew_arm64:smoke:upgrade
yarn nx run cem_ml_cli_native_brew_arm64:smoke:uninstall
```

All lifecycle targets require an Apple Silicon host. `publish` is deliberately
inert unless `CEM_ML_NATIVE_PUBLISH=1` is present, Apple signing/notarization is
complete, a GitHub artifact-attestation bundle is supplied, and
`cem-ml-v<version>` already exists as a draft GitHub Release. The generated
formula points at that immutable release archive; `EPA-WG/homebrew-cem` consumes
the projection and never rebuilds the executable.
