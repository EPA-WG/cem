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

- [x] Add CI-owned production and upload for the two WASM/npm units and Linux AMD64.
    - [x] Build each unit from the checked-out tag through its Nx package/sign/
          verify targets and run clean-consumer, platform-parity, and Linux
          install/upgrade/uninstall gates.
    - [x] Generate GitHub artifact attestations and publication-ready signing
          evidence without granting any producer permission to publish the draft.
    - [x] Upload version-qualified unit assets idempotently to the existing draft;
          use GitHub Actions artifacts only for job-to-job transport.
    - [x] Record source commit, Nx target, workflow run, toolchain/target identity,
          attestation, and smoke evidence for each CI-owned unit.
    - Completed 2026-08-17: added independent Nx-owned npm/WASM and Linux AMD64
      producer lanes, checksum-subject GitHub attestations, protected Linux GPG
      signing, publication-mode unit verification, Linux lifecycle smoke gates,
      Actions-artifact transport, and byte-verifying no-clobber upload to the
      existing draft. The protected `cem-ml-release` environment owns the GPG
      secret material, signing fingerprint, and both draft/upload opt-ins;
      producer jobs have no `contents: write` authority.
    - Completed 2026-08-17: each CI unit now emits an authoritative,
      version-qualified `*.producer-evidence.json` sidecar after its gates pass.
      The sidecar binds the exact workflow run/attempt, actors, runner image,
      source/workflow commits, Nx targets and gate results, captured toolchain and
      target identities, release-entry/checksum/signing records, and original
      artifact-attestation ID, URL, bundle, and digest. A separate
      `actions/attest@v4` invocation signs the immutable sidecar; Nx verifies that
      detached bundle before Actions transport, draft upload, and aggregate-index
      inclusion. Synthetic coordinator, signing, schema, target-contract, drift,
      aggregate, and idempotence coverage passes 38 tests.

- [ ] Document and verify authorized native-host publication and recovery lanes.
    - [x] Add one exact-tag/source-commit preflight shared by macOS ARM64,
          Windows AMD64, and optional Linux recovery execution.
    - Completed 2026-08-17: all three native deployment projects expose an
      uncached `preflight:release` target and require it before `publish`. The
      shared read-only coordinator requires an explicit exact version tag, the
      tag and clean checkout at the same source commit, the declared native
      OS/architecture and deployment/Nx/Rust identities, and an existing exact
      GitHub draft with matching title and prerelease state. Local failures stop
      before GitHub access, the preflight has no mutation path, and 53
      platform-release/version tests cover the three accepted hosts plus tag,
      commit, checkout, host, deployment, and remote-release drift.
    - [ ] Document and exercise each host's package → sign → verify → lifecycle
          smoke → immutable publish sequence, including required credentials and
          attestation-bundle handoff.
    - Prepared 2026-08-17: the protected, manual
      `cem-ml-native-release.yml` lane now targets exact dedicated macOS ARM64
      and Windows AMD64 runner labels. Separate least-privilege producer and
      publisher jobs execute the Nx lifecycle graph once, attest the signed
      checksum subjects, finalize and preserve the complete unit through Actions
      artifact transport, then publish those same bytes. The operator runbook
      documents runner isolation, signing credentials, protected variables,
      dispatch, attestation handoff, retry, and Linux recovery. The leaf remains
      open until the two self-hosted runners are registered and successful run
      IDs are recorded; no repository-scoped runners are currently registered.
    - [x] Make each native publisher retain identical assets, upload only missing
          names, reject remote extras or byte drift, and never use `--clobber`.
    - Completed 2026-08-17: Linux, macOS, and Windows publishers share one
      no-clobber draft synchronizer. It rejects stale local files and unexpected
      remote assets in the unit namespace, byte-verifies existing names before
      mutation, uploads only missing paths, then redownloads and verifies the
      exact final set while preserving foreign units. Windows additionally
      rechecks downloaded ZIP/MSI Authenticode state. Producer/publisher job
      separation makes retry reuse the preserved finalized bytes instead of
      rebuilding timestamped artifacts. Synthetic drift, idempotence, workflow,
      target, and publisher coverage passes 60 platform-release/version tests.
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
