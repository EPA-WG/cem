# Completed Todo — 2026-08-18

This archive preserves the completed execution checklist for roadmap
[`Phase 2.6 - CEM-ML GitHub Release Artifact Promotion`](../../roadmap.md#phase-26---cem-ml-github-release-artifact-promotion).
The complete protected release and recovery evidence is recorded in
[`../cem-ml-release-rehearsal-0.1.0-rc.2.md`](../cem-ml-release-rehearsal-0.1.0-rc.2.md).

## Phase 2.6 Outcome

CEM-ML now has a protected, resumable, exactly-once GitHub Release path for
three CI-owned units: `@epa-wg/cem-ml`, `@epa-wg/cem-ml-cli`, and
`native-linux-amd64`. The workflow builds and verifies each unit from the exact
tagged commit, uploads only absent assets to a draft, recollects and verifies
the complete remote byte set, and grants publication authority only to the
protected aggregate finalizer. macOS/Homebrew and Windows/WinGet publication
remain explicitly deferred and their retained self-hosted recipe schedules no
jobs.

## Completed Checklist

- [x] Promote Phase 2.6 and audit the Phase 2.5 release foundation.
    - [x] Confirm resolved Nx package, sign, verify, release, and native lifecycle
          surfaces.
    - [x] Confirm immutable upload, publication signing, and post-upload
          redownload foundations.
    - [x] Inventory the missing protected coordinator, producer, collection, and
          promotion boundaries.

- [x] Establish the protected draft coordinator and workflow isolation.
    - [x] Test absent-draft creation, matching draft resumption,
          wrong-tag/non-draft rejection, and the no-publish invariant.
    - [x] Add Nx-owned exact-tag and exact-commit draft coordination.
    - [x] Add the dedicated workflow, per-tag concurrency, minimal permissions,
          and protected `cem-ml-release` environment.
    - [x] Configure reviewer, tag restriction, disabled administrator bypass,
          and opt-in environment policy.
    - [x] Exclude `cem-ml-v*` from the generic npm-family publish workflow.

- [x] Add CI-owned production and upload for both WASM/npm units and Linux AMD64.
    - [x] Run package, signing/attestation, verification, clean-consumer, parity,
          and Linux lifecycle gates from the tagged commit.
    - [x] Generate checksummed producer evidence and GitHub artifact
          attestations without producer publication authority.
    - [x] Use Actions artifacts only for job transport and the draft GitHub
          Release as the durable boundary.
    - [x] Upload absent assets, accept identical bytes, and reject overwrite or
          drift.

- [x] Restrict GitHub Releases to WASM/npm and Linux AMD64.
    - [x] Make the aggregate contract contain exactly the three CI-owned units.
    - [x] Reject macOS and Windows release preflight/upload identities.
    - [x] Retain but hard-disable all macOS/Windows self-hosted jobs.
    - [x] Document Homebrew and WinGet outputs as local validation artifacts, not
          current release promises.

- [x] Collect the complete draft and generate aggregate release evidence.
    - [x] Download and classify the exact expected remote assets.
    - [x] Reject missing, extra, duplicate, misclassified, drifted, unsigned, or
          unattested assets.
    - [x] Run publication-mode aggregate stage/verification.
    - [x] Upload the aggregate index and `SHA256SUMS`, then redownload and verify
          the complete release byte set.

- [x] Add protected, exactly-once GitHub Release promotion.
    - [x] Test incomplete drafts, remote drift, matching published retries, run
          identity drift, and forbidden overwrite/rebuild/repack behavior.
    - [x] Add remote-only `cem_ml:release:promote` behind protected opt-in.
    - [x] Bind promotion to the recorded workflow run and complete producer
          evidence.
    - [x] Require npm/APT channels to consume the immutable release assets without
          rebuild or repack.

- [x] Rehearse, document, and close Phase 2.6.
    - [x] Run the protected `0.1.0-rc.2` release through all three producers,
          immutable upload, aggregate verification, and publication.
    - [x] Verify all 37 remote assets and all 36 `SHA256SUMS` entries.
    - [x] Re-run only the aggregate finalizer on the same workflow run and prove
          the published immutable release remains byte- and timestamp-identical.
    - [x] Prove the generic `cem`, CEM-ML, and disabled native local-host lanes do
          not trigger or publish one another's release groups or tags.
    - [x] Record commands, workflow/job IDs, producer evidence, protected
          approvals, package-channel digests, and operator recovery rules.
    - [x] Promote roadmap Phase 3 into the active todo.

## Accepted Verification

```bash
NX_TUI=false yarn nx run cem_ml:test:platform-release
NX_TUI=false yarn nx run @epa-wg/cem-ml-cli:test
NX_TUI=false yarn nx run cem_ml:verify:platform
```

The final pre-tag run passed 73 platform-release tests, 16 CLI JavaScript tests,
and the aggregate platform target with all 64 dependencies. Protected GitHub
run `32152871319` then passed every producer, uploader, verifier, and promotion
job. Its finalizer-only second attempt proved the same-run immutable recovery
path.

## Deferred Work

- macOS ARM64/Homebrew and Windows AMD64/WinGet publication remain outside the
  current GitHub Release contract.
- npm registry and APT repository publication may consume the immutable release
  assets in a later explicitly authorized operation; this rehearsal did not
  publish either channel.
- Phase 5 Figma UI Kit work and Phase 8 Swift/Kotlin compile gates remain owned
  by their roadmap phases.
