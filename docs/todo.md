# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/).

## Immediate Goal

Execute
[`Phase 2.6 - CEM-ML GitHub Release Artifact Promotion`](../roadmap.md#phase-26---cem-ml-github-release-artifact-promotion).
Connect the existing WASM/npm and Linux CI artifact generators, authorized
macOS and Windows native-host publication lanes, complete draft collection,
remote verification, and protected GitHub Release promotion without allowing
any lane to rebuild or overwrite an already uploaded release unit.

Phase 2.5 is complete. Its full deployment checklist, rationale, platform
lifecycle evidence, and verification commands are preserved in
[`archive/todo-completed-2026-08-17.md`](archive/todo-completed-2026-08-17.md).

Recommended execution order: keep release policy in tested Nx-owned scripts,
then make GitHub Actions and authorized native hosts thin producers around those
targets. Establish the draft boundary first, add independent producer lanes,
collect and remotely verify all five units, and grant publication authority
only to the final promotion step.

## Phase 2.6 Checklist

- [x] Promote Phase 2.6 and audit the Phase 2.5 release foundation.
    - [x] Confirm the resolved Nx graph exposes package/sign/verify surfaces for
          both WASM/npm units and package/sign/verify/publish plus lifecycle
          smoke surfaces for Linux AMD64, macOS ARM64, and Windows AMD64.
    - [x] Confirm `cem_ml:release:stage`, `release:verify`, and
          `release:upload-draft` enforce the five-unit aggregate, publication
          signing state, immutable existing bytes, missing-only upload, and
          post-upload redownload verification.
    - [x] Confirm the remaining boundary gaps: the generic npm publish workflow
          is not contractually isolated from `cem-ml-v{version}` tags, no
          protected CEM-ML workflow creates/resumes the draft, CI does not
          produce/upload its three units, and no final promotion target exists.
    - Completed 2026-08-17: the roadmap phase is now active and the verified
      foundation/gap inventory above defines the execution boundary.

- [x] Establish the protected draft coordinator and release-workflow isolation.
    - [x] Add synthetic coordinator tests for absent-draft creation, identical
          draft resumption, wrong-tag/non-draft rejection, and the no-publish
          invariant before adding workflow wiring.
    - [x] Add an Nx-owned create/resume-draft target that validates the exact
          `cem-ml-v{version}` tag and tagged source commit and never publishes,
          deletes, replaces, or supersedes a release.
    - [x] Add the dedicated CEM-ML workflow scaffold with exact tag and manual
          retry inputs, per-tag concurrency, the `cem-ml-release` environment
          reference, and job-scoped minimal `contents` permission for draft
          creation. Producer jobs add `id-token` and `attestations` only when
          their later checklist item requires them.
    - [x] Configure the `cem-ml-release` repository environment with required
          reviewer `sashafirsov`, solo-maintainer self-review, administrator
          bypass disabled, the `cem-ml-v*` tag-only deployment restriction, and
          environment variable `CEM_ML_PLATFORM_DRAFT=1`.
    - [x] Prove the generic `{version}` npm-family workflow rejects CEM-ML tags
          before it can invoke the `cem` Nx release group.
    - Completed 2026-08-17: applied the approved deterministic `CEM-ML {version}`
      title, generated notes bounded by the preceding reachable CEM-ML tag,
      environment name `cem-ml-release`, and required `release_tag` manual input.
      The Nx-owned coordinator is opt-in, creates or resumes drafts only, and
      reads the created release back before success.

- [ ] Add CI-owned production and upload for the two WASM/npm units and Linux AMD64.
    - [x] Build each unit from the checked-out tag through its Nx package/sign/
          verify targets and run clean-consumer, platform-parity, and Linux
          install/upgrade/uninstall gates.
    - [x] Generate GitHub artifact attestations and publication-ready signing
          evidence without granting any producer permission to publish the draft.
    - [x] Upload version-qualified unit assets idempotently to the existing draft;
          use GitHub Actions artifacts only for job-to-job transport.
    - [ ] Record source commit, Nx target, workflow run, toolchain/target identity,
          attestation, and smoke evidence for each CI-owned unit.
    - Completed 2026-08-17: added independent Nx-owned npm/WASM and Linux AMD64
      producer lanes, checksum-subject GitHub attestations, protected Linux GPG
      signing, publication-mode unit verification, Linux lifecycle smoke gates,
      Actions-artifact transport, and byte-verifying no-clobber upload to the
      existing draft. The protected `cem-ml-release` environment owns the GPG
      secret material, signing fingerprint, and both draft/upload opt-ins;
      producer jobs have no `contents: write` authority. Synthetic coordinator,
      signing, target-contract, drift, and idempotence coverage passes 30 tests.
    - Next decision point: producer evidence must be emitted after all smoke gates,
      but mutating an already checksummed and attested release unit would invalidate
      its evidence. Recommended design is a separate version-qualified
      `*.producer-evidence.json` sidecar per unit containing the workflow run and
      attempt, Nx target, pinned toolchain/target identity, attestation reference,
      and gate results, with its own attestation rather than rewriting package
      subjects. Confirm that schema and trust boundary before implementing it.

- [ ] Document and verify authorized native-host publication and recovery lanes.
    - [ ] Add one exact-tag/source-commit preflight shared by macOS ARM64,
          Windows AMD64, and optional Linux recovery execution.
    - [ ] Document and exercise each host's package → sign → verify → lifecycle
          smoke → immutable publish sequence, including required credentials and
          attestation-bundle handoff.
    - [ ] Make each native publisher retain identical assets, upload only missing
          names, reject remote extras or byte drift, and never use `--clobber`.
    - [ ] Record authorized host, signing/notarization/Authenticode evidence,
          target/toolchain identity, smoke results, and uploaded digests.

- [ ] Collect the complete draft and generate aggregate release evidence.
    - [ ] Add an Nx-owned collection target that downloads the draft into the five
          expected unit roots and rejects missing, extra, duplicate, or
          misclassified assets.
    - [ ] Run publication-mode aggregate stage/verification over the downloaded
          units, upload the aggregate release index and `SHA256SUMS` idempotently,
          and redownload the complete remote set.
    - [ ] Verify every remote filename, size, digest, version, source commit,
          runtime/target/capability identity, SBOM, provenance, signature/
          attestation, and immutable package-channel URL.

- [ ] Add protected, exactly-once GitHub Release promotion.
    - [ ] Add synthetic promotion tests covering incomplete drafts, remote drift,
          already-published matching releases, and forbidden overwrite/rebuild
          paths.
    - [ ] Add an Nx promotion target that consumes only remotely verified evidence
          and changes the matching draft to published exactly once.
    - [ ] Restrict promotion authority to the protected finalizer and record the
          final workflow run and complete producer evidence in the release index.
    - [ ] Prove APT, Homebrew, WinGet, and npm publication consume the published
          immutable assets and cannot repack or rebuild them.

- [ ] Rehearse, document, and close Phase 2.6.
    - [ ] Run one protected release rehearsal across the CI/manual handoff,
          interruption/resume paths, complete remote verification, and promotion.
    - [ ] Prove generic `cem`, CEM-ML, and native local-host lanes cannot trigger or
          publish one another's release groups/tags.
    - [ ] Record the exact commands, workflow runs, host evidence, expected
          unavailable-host behavior, and any operator recovery steps.
    - [ ] Archive the completed checklist and promote the next roadmap goal.

## Deferred Roadmap Work

The externally reviewed Figma UI Kit work remains owned by roadmap Phase 5.
Swift/Xcode and Kotlin/Compose compile gates remain owned by roadmap Phase 8.
Their prior detailed deferred checklists are preserved in the Phase 2.5 archive;
neither is an active workspace task in this file.
