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

Generic CI intentionally skips this native executable. Lifecycle and release
targets run on an authorized Apple Silicon host through the protected manual
workflow. To publish the local build, use the operator sequence in
[`docs/cem-ml-native-release.md`](../../docs/cem-ml-native-release.md). It keeps
the Apple credentials in the runner keychain, obtains the GitHub attestation in
the same protected job, and preserves the exact finalized bytes for a separate
publisher job.

For native target development without publication, export the Apple
signing/notarization variables and run:

```bash
CEM_ML_RELEASE_SIGNING=required \
yarn nx run cem_ml_cli_native_brew_arm64:smoke:release
```

`publish` remains inert until the release controls and finalized attestation are
present and `cem-ml-v<version>` exists as a draft GitHub Release. The generated formula
points at that immutable release archive; `EPA-WG/homebrew-cem` consumes the
projection and never rebuilds the executable.
