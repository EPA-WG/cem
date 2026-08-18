# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/).

## Immediate Goal

Execute
[`Phase 2.6 - CEM-ML GitHub Release Artifact Promotion`](../roadmap.md#phase-26---cem-ml-github-release-artifact-promotion).
Connect the two WASM/npm and Linux AMD64 CI artifact generators, complete
three-unit draft collection, remote verification, and protected GitHub Release
promotion without allowing any lane to rebuild or overwrite an already uploaded
release unit. macOS and Windows native releases are explicitly deferred.

Phase 2.5 is complete. Its full deployment checklist, rationale, platform
lifecycle evidence, and verification commands are preserved in
[`archive/todo-completed-2026-08-17.md`](archive/todo-completed-2026-08-17.md).

Recommended execution order: keep release policy in tested Nx-owned scripts,
then make GitHub Actions thin producers around those targets. Establish the
draft boundary first, add the three independent CI-owned units, collect and
remotely verify them, and grant publication authority only to the final
promotion step.

## Phase 2.6 Checklist

- [x] Promote Phase 2.6 and audit the Phase 2.5 release foundation.
    - [x] Confirm the resolved Nx graph exposes package/sign/verify surfaces for
          both WASM/npm units and package/sign/verify/publish plus lifecycle
          smoke surfaces for Linux AMD64, macOS ARM64, and Windows AMD64.
    - [x] Confirm the original aggregate stage/verify/upload implementation
          enforced publication signing state, immutable existing bytes,
          missing-only upload, and post-upload redownload verification before
          applying the narrower three-unit release decision below.
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

- [x] Restrict GitHub Releases to WASM/npm and Linux AMD64; defer self-hosted native lanes.
    - [x] Make the tested aggregate contract contain exactly
          `@epa-wg/cem-ml`, `@epa-wg/cem-ml-cli`, and `native-linux-amd64`.
    - [x] Remove macOS and Windows artifact roots from the Nx aggregate release
          target and reject their release preflight/upload paths before GitHub
          mutation.
    - [x] Keep the reviewed macOS/Windows self-hosted workflow recipe in-tree,
          but hard-disable all four producer/publisher jobs and remove its
          publication opt-in from the protected environment.
    - [x] Update the roadmap, deployment docs, and native project READMEs so
          Homebrew/WinGet outputs are local validation artifacts, not release
          promises.
    - Completed 2026-08-17: the release contract is now wholly CI-owned. The
      self-hosted runner recipes remain reviewable for a future scope decision,
      but dispatch schedules no jobs and the shared uploader accepts Linux only.
      No macOS/Windows runner registration or release credential setup is needed;
      62 platform-release/version tests verify the narrowed contract.

- [x] Collect the complete draft and generate aggregate release evidence.
    - [x] Add an Nx-owned collection target that downloads the draft into the three
          expected unit roots and rejects missing, extra, duplicate, or
          misclassified assets.
    - [x] Run publication-mode aggregate stage/verification over the downloaded
          units, upload the aggregate release index and `SHA256SUMS` idempotently,
          and redownload the complete remote set.
    - [x] Verify every remote filename, size, digest, version, source commit,
          runtime/target/capability identity, SBOM, provenance, signature/
          attestation, and immutable package-channel URL.
    - Completed 2026-08-17: added uncached `cem_ml:release:collect-draft`
      classification by the three exact asset coordinates. Collection validates
      the exact tag, clean source commit, draft state, complete per-unit file set,
      checksums, GPG-signature record, GitHub attestations, producer evidence,
      SBOM, provenance, capabilities, and APT URL before replacing any local
      artifact root. The protected aggregate job then runs publication-mode
      stage/verify, uploads only missing aggregate evidence, redownloads the
      complete draft, and verifies the remote bytes again. Retry preserves
      identical assets and repairs a one-file aggregate interruption without
      clobbering; 68 platform-release/version tests cover collection, drift,
      classification, remote extras, and idempotence.

- [x] Add protected, exactly-once GitHub Release promotion.
    - [x] Add synthetic promotion tests covering incomplete drafts, remote drift,
          already-published matching releases, and forbidden overwrite/rebuild
          paths.
    - [x] Add an Nx promotion target that consumes only remotely verified evidence
          and changes the matching draft to published exactly once.
    - [x] Restrict promotion authority to the protected finalizer and record the
          final workflow run and complete producer evidence in the release index.
    - [x] Prove APT and npm publication consume the published immutable assets
          and cannot repack or rebuild them.
    - Completed 2026-08-17: added uncached, remote-only
      `cem_ml:release:promote` behind the protected
      `CEM_ML_PLATFORM_PROMOTE` gate. The aggregate index records the stable
      finalizer `GITHUB_RUN_ID`, complete per-unit producer evidence, and exact
      npm/APT release-asset inputs with rebuild/repack forbidden. Promotion
      redownloads and publication-verifies every remote byte before and after
      the single draft transition, accepts an identical published retry, and
      requires GitHub to report the result immutable. Interrupted aggregate
      finalization must use **Re-run jobs** on the recorded workflow run; a new
      dispatch cannot replace its run-bound evidence. The 72
      platform-release/version tests cover incomplete/drifted drafts,
      already-published idempotence, run drift, and forbidden asset mutation.

- [ ] Rehearse, document, and close Phase 2.6.
    - [ ] Run one protected release rehearsal across the three CI-owned units,
          interruption/resume paths, complete remote verification, and promotion.
    - [ ] Prove generic `cem`, CEM-ML, and disabled native local-host lanes cannot
          trigger or publish one another's release groups/tags.
    - [ ] Record the exact commands, workflow runs, CI producer evidence,
          disabled-native behavior, and any operator recovery steps.
    - [ ] Archive the completed checklist and promote the next roadmap goal.

## Deferred Roadmap Work

The externally reviewed Figma UI Kit work remains owned by roadmap Phase 5.
Swift/Xcode and Kotlin/Compose compile gates remain owned by roadmap Phase 8.
Their prior detailed deferred checklists are preserved in the Phase 2.5 archive;
neither is an active workspace task in this file.
