# CEM-ML native Windows AMD64 deployment

This Nx deployment project owns only the `x86_64-pc-windows-msvc` /
`native-windows-amd64` CEM-ML CLI artifacts. It builds the common
`packages/cem_ml_cli` binary without copying semantic implementation code.

The package target emits a versioned portable `.zip`, a WiX v4 per-machine
`.msi`, target-qualified SHA-256 metadata, a Syft-generated SPDX 2.3 SBOM, the
common native capability manifest, an unsigned build-provenance record,
versioned WinGet manifest projections, and a release-index entry. The MSI adds
`%ProgramFiles%\EPA-WG\CEM-ML` to the machine `PATH` and supports quiet install,
major upgrade, and uninstall through Windows Installer. Both portable payloads
use the deployment-owned static MSVC runtime contract; build and verification
reject dynamic MSVC/UCRT imports so they run on clean offline Windows hosts.

```powershell
yarn nx run cem_ml_cli_native_windows_amd64:build
yarn nx run cem_ml_cli_native_windows_amd64:package
yarn nx run cem_ml_cli_native_windows_amd64:verify
yarn nx run cem_ml_cli_native_windows_amd64:smoke:install
yarn nx run cem_ml_cli_native_windows_amd64:smoke:upgrade
yarn nx run cem_ml_cli_native_windows_amd64:smoke:uninstall
yarn nx run cem_ml_cli_native_windows_amd64:smoke:sandbox
```

Ordinary local runs record deterministic unsigned state. An authorized local
release run supplies `CEM_ML_ARTIFACT_SIGNING_ENDPOINT`,
`CEM_ML_ARTIFACT_SIGNING_ACCOUNT`, and
`CEM_ML_ARTIFACT_SIGNING_PROFILE`, along with optional explicit
`CEM_ML_SIGNTOOL` and `CEM_ML_ARTIFACT_SIGNING_DLIB` paths. The `sign` target
then Authenticode-signs and securely timestamps both the executable and MSI
through the EPA-WG Microsoft Artifact Signing public-trust profile, verifies
the signatures before and after packaging, and regenerates all dependent
integrity metadata.

`publish` is deliberately inert unless `CEM_ML_NATIVE_PUBLISH=1` is present,
the signing record is finalized with a verified GitHub artifact attestation,
and `cem-ml-v<version>` already exists as a draft GitHub Release. It uploads
only missing immutable assets, downloads the complete Windows unit again, and
rechecks its digests and Authenticode timestamps before completing.

Generic CI intentionally skips this native executable. Build, package,
`winget validate`, direct MSI lifecycle checks, and publication run through the
Nx targets on an authorized Windows AMD64 host. The protected self-hosted
workflow, required runner state, credential boundaries, attestation handoff, and
retry procedure are documented in
[`docs/cem-ml-native-release.md`](../../docs/cem-ml-native-release.md).

For signed native target development without publication, export the Artifact
Signing variables and run:

```powershell
$env:CEM_ML_RELEASE_SIGNING = 'required'
yarn nx run cem_ml_cli_native_windows_amd64:smoke:release
```

The Sandbox lifecycle requires Windows 11 Pro or Enterprise, nested
virtualization, the Windows Sandbox optional feature, and an interactive local
session.
