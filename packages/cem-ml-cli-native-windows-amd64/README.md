# CEM-ML native Windows AMD64 deployment

This Nx deployment project owns only the `x86_64-pc-windows-msvc` /
`native-windows-amd64` CEM-ML CLI artifacts. It builds the common
`packages/cem_ml_cli` binary without copying semantic implementation code.

The package target emits a versioned portable `.zip`, a WiX v4 per-machine
`.msi`, target-qualified SHA-256 metadata, a Syft-generated SPDX 2.3 SBOM, the
common native capability manifest, an unsigned build-provenance record,
versioned WinGet manifest projections, and a release-index entry. The MSI adds
`%ProgramFiles%\EPA-WG\CEM-ML` to the machine `PATH` and supports quiet install,
major upgrade, and uninstall through Windows Installer.

```powershell
yarn nx run cem_ml_cli_native_windows_amd64:build
yarn nx run cem_ml_cli_native_windows_amd64:package
yarn nx run cem_ml_cli_native_windows_amd64:verify
yarn nx run cem_ml_cli_native_windows_amd64:smoke:install
yarn nx run cem_ml_cli_native_windows_amd64:smoke:upgrade
yarn nx run cem_ml_cli_native_windows_amd64:smoke:uninstall
```

Local and ordinary CI runs record deterministic unsigned state. A protected
release job supplies `CEM_ML_ARTIFACT_SIGNING_ENDPOINT`,
`CEM_ML_ARTIFACT_SIGNING_ACCOUNT`, and
`CEM_ML_ARTIFACT_SIGNING_PROFILE`, along with optional explicit
`CEM_ML_SIGNTOOL` and `CEM_ML_ARTIFACT_SIGNING_DLIB` paths. The `sign` target
then Authenticode-signs and securely timestamps both the executable and MSI
through the EPA-WG Microsoft Artifact Signing public-trust profile, verifies
the signatures before and after packaging, and regenerates all dependent
integrity metadata.

`publish` is deliberately inert unless `CEM_ML_NATIVE_PUBLISH=1` is present,
the signing record is publication-ready, and `cem-ml-v<version>` already exists
as a draft GitHub Release. It verifies the supplied GitHub artifact-attestation
bundle, uploads immutable assets, downloads the ZIP and MSI again, and rechecks
their digests and Authenticode timestamps before completing.

GitHub-hosted `windows-2025` owns the build, package, `winget validate`, and
direct MSI lifecycle. The stronger Sandbox lifecycle runs only when repository
variable `CEM_ML_WINDOWS_SANDBOX_ENABLED` is `1`, on a self-hosted runner with
labels `self-hosted`, `Windows`, `X64`, and `windows-11-sandbox-x64`. That host
must be Windows 11 Pro or Enterprise, expose nested virtualization, have the
Windows Sandbox optional feature enabled, and run the Actions runner in an
interactive session.
