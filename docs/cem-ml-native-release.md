# CEM-ML native release operators

The native release workflow is manually dispatched and extends only an existing
protected `cem-ml-v<version>` draft. It never creates a tag, creates or publishes
a release, rebuilds a remotely owned native unit, or replaces a release asset.

Linux AMD64 remains owned by the protected `CEM-ML Release` CI workflow. The
native workflow covers the two signing environments that require dedicated
hosts:

| Selection | Required runner labels | Signing authority |
| --- | --- | --- |
| `macos-arm64` | `self-hosted`, `macOS`, `ARM64`, `local-macos-arm64` | Developer ID certificate and `notarytool` keychain profile |
| `windows-amd64` | `self-hosted`, `Windows`, `X64`, `local-windows-amd64` | Microsoft Artifact Signing public-trust profile |

## Protection and runner registration

Register each dedicated machine at repository or organization scope with every
label in its row. Restrict organization runners to this repository. The runner
must accept only trusted release workflows, use an isolated service account and
work directory, run one release job at a time, and never execute untrusted pull
request code. Keep the runner offline when it is not being used for a release.

Before the first run, confirm the exact labels under **Settings → Actions →
Runners** and keep the `cem-ml-release` environment approval policy enabled. A
queued job with no matching runner is a configuration failure; do not weaken the
workflow labels to make it start.

The protected environment requires these variables:

| Variable | Value or source |
| --- | --- |
| `CEM_ML_NATIVE_PUBLISH` | Exactly `1`; the publication opt-in |
| `CEM_ML_APPLE_SIGNING_IDENTITY` | Exact Developer ID Application identity installed on the macOS runner |
| `CEM_ML_APPLE_NOTARY_PROFILE` | Name of the runner-local `notarytool` keychain profile |
| `CEM_ML_ARTIFACT_SIGNING_ENDPOINT` | Microsoft Artifact Signing endpoint |
| `CEM_ML_ARTIFACT_SIGNING_ACCOUNT` | EPA-WG code-signing account |
| `CEM_ML_ARTIFACT_SIGNING_PROFILE` | EPA-WG public-trust certificate profile |
| `CEM_ML_SIGNTOOL` | Optional absolute runner-local SignTool path |
| `CEM_ML_ARTIFACT_SIGNING_DLIB` | Optional absolute runner-local Artifact Signing DLib path |

Apple credentials remain in the runner keychain behind the named profile.
Windows Artifact Signing authentication remains in the dedicated runner's
credential provider. Do not copy those credentials into repository variables or
workflow arguments. The Windows runner also needs the pinned Rust target, WiX 4,
WinGet, the Windows SDK SignTool, Artifact Signing Client Tools, and permission
to exercise per-machine MSI install/upgrade/uninstall. The macOS runner needs
Xcode command-line tools, Homebrew, the Developer ID certificate, and the
preconfigured notary profile.

## Exact release sequence

First run the protected `CEM-ML Release` workflow for the exact tag. It creates
or resumes the draft and produces the npm and Linux units. Confirm that the tag
and draft still match the intended Cargo version and source commit.

Dispatch one native host at a time:

```bash
gh workflow run cem-ml-native-release.yml \
  --repo EPA-WG/cem \
  -f native_host=macos-arm64 \
  -f release_tag=cem-ml-vX.Y.Z

gh workflow run cem-ml-native-release.yml \
  --repo EPA-WG/cem \
  -f native_host=windows-amd64 \
  -f release_tag=cem-ml-vX.Y.Z
```

Replace the example tag with the exact existing tag. The protected environment
approval is intentional and must not be bypassed.

Each producer job performs this sequence:

1. Check out the exact tag and run the shared clean-source/native-host/draft
   preflight.
2. Execute one Nx task graph that packages, release-signs, verifies, and runs
   install → upgrade → uninstall smoke gates. This prevents signing or
   notarization from being repeated between gates.
3. Give the signed checksum manifest to `actions/attest@v4`. The resulting OIDC
   bundle is verified on the same native host and copied into the release unit.
4. Run publication-mode verification, then preserve the complete finalized
   directory as an uncompressed GitHub Actions artifact.

The separate publisher job downloads those exact preserved bytes, reruns the
tag/source/host preflight and publication verification, and then:

- rejects remote assets in its own version-qualified namespace that are not in
  the local unit;
- downloads and byte-verifies every already-present owned asset;
- uploads only missing filenames without `--clobber`;
- redownloads and byte-verifies the complete owned set;
- rechecks downloaded Windows ZIP/MSI Authenticode signatures and timestamps.

Assets belonging to the other four release units are preserved.

## Failure and recovery

If a producer fails before preservation, fix the host and rerun the producer;
no native assets have been uploaded to the draft. If publication fails after any
asset reaches the draft, rerun only the failed publisher job in the same Actions
run so it reuses the preserved release unit. Do not rebuild or re-sign: secure
timestamps and notarization can change bytes, and the immutable uploader will
reject that drift.

Optional Linux recovery uses a protected manual dispatch of
`.github/workflows/cem-ml-release.yml` for the same exact tag. Its Linux producer
packages, obtains the GitHub attestation, GPG-signs, verifies, runs the lifecycle
smokes, records producer evidence, and hands the preserved unit to the protected
CI uploader. Do not use the native macOS/Windows workflow to replace the
CI-owned Linux unit.

The lane is not considered exercised until successful macOS and Windows runs
and their exact run IDs are recorded in the roadmap completion note.
