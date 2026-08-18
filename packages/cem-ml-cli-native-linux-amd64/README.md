# CEM-ML native Linux AMD64 deployment

This Nx deployment project owns only the `x86_64-unknown-linux-gnu` /
`native-linux-amd64` CEM-ML CLI artifacts. It builds the common
`packages/cem_ml_cli` binary without copying semantic implementation code.

The package target emits versioned GNU/Linux `.tar.gz` and `.deb` artifacts,
target-qualified SHA-256 metadata, a Syft-generated SPDX 2.3 SBOM, the common
native capability manifest, an unsigned build-provenance record, an immutable
APT channel record, and a release-index entry. The sign target records local
unsigned state by default; release jobs add the EPA-WG GPG signature and GitHub
artifact attestation before publication can proceed.

```bash
yarn nx run cem_ml_cli_native_linux_amd64:build
yarn nx run cem_ml_cli_native_linux_amd64:package
yarn nx run cem_ml_cli_native_linux_amd64:verify
yarn nx run cem_ml_cli_native_linux_amd64:smoke:install
yarn nx run cem_ml_cli_native_linux_amd64:smoke:upgrade
yarn nx run cem_ml_cli_native_linux_amd64:smoke:uninstall
```

`publish` is deliberately inert unless `CEM_ML_NATIVE_PUBLISH=1` is present,
the signing record is publication-ready, and `cem-ml-v<version>` already exists
as a draft GitHub Release. The APT record points at that immutable release; the
separate `EPA-WG/cem-apt` repository consumes it and never rebuilds the binary.
Protected CI publication and optional Linux recovery are documented in
the protected [CEM-ML release workflow](../../.github/workflows/cem-ml-release.yml).
The separate [self-hosted native recipe](../../docs/cem-ml-native-release.md)
is deferred and does not own Linux publication.

The `build` target separates the expensive Rust compilation from the lightweight
release-provenance stamp. Nx keys the `compile` dependency by Rust sources and
toolchain and caches only its staged binary and capability manifest; the current
Git commit and `SOURCE_DATE_EPOCH` invalidate only the fast `build` assembly.
