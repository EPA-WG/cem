# Todo

This file is the authoritative checklist for remaining execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/).

## Immediate Goal

Start Phase 2.5 by turning the existing common `cem_ml` engine and
`cem_ml_cli` native source projects into a separately deployable,
version-synchronized CEM-ML product family. The canonical deployment decision
is [`cem-ml-deployment-contract.md`](cem-ml-deployment-contract.md); the Phase
2.5 roadmap section supplies product sequencing, while
[`cem-studio.md`](cem-studio.md) remains the broader Studio proposal.

The starting Nx audit confirms that `cem_ml` and `cem_ml_cli` provide Rust
build, test, lint, WASM-build, fixture, and release-publish surfaces. The
workspace does not yet contain the `@epa-wg/cem-ml` WASM npm deployment,
`@epa-wg/cem-ml-cli` universal npm deployment, any of the three native
deployment projects, or the fixed `cem-ml-platform` release group. The common
`packages/cem_ml/Cargo.toml` version is the intended authority, but no accepted
cross-project synchronizer or drift gate exists yet.

The cross-layer architecture remains serializer-free: lifecycle loading, graph
routing, joins, evaluators, CEM-QL, CEMT, and XSLT adapters exchange borrowed
native AST streams or typed evaluator values directly. JSON and other encodings
are allowed only at explicit lifecycle parse, versioned host-wire, report, or
registered export boundaries; no serializer, generic DTO, shape inference, or
replacement tree may mediate between internal engine layers.

### Phase 2.5 — CEM-ML CLI Deployment Foundation

- [x] Accept the deployment-project, platform, and synchronized-version
      contract before scaffolding a deployment package or release target.
    - [x] Review and accept the concrete recommendation in
          [`cem-ml-deployment-contract.md`](cem-ml-deployment-contract.md),
          including its five organization-level review gates.
    - [x] Fix the final workspace roots, Nx project names, npm package identities,
          native artifact identities, and dependency edges while keeping common
          Rust source projects distinct from deployment units.
    - [x] Select the Nx version-synchronization and fixed-release mechanism that
          reads `packages/cem_ml/Cargo.toml` as the sole version authority,
          updates every deployment manifest and exact internal dependency, and
          rejects drift without modifying versions during verification.
    - [x] Pin supported Node versions and host platforms for the portable
          `wasm-node` CLI.
    - [x] Pin the build, cross-compile, signing, archive/installer,
          package-channel, and install-smoke toolchains for Linux AMD64, Homebrew
          macOS ARM64, and Windows AMD64.
    - [x] Freeze the first-release operation/capability matrix, including explicit
          native/WASM gaps, rather than claiming parity from command-name
          availability alone.
    - [x] Freeze one versioned machine contract for requests, progress,
          cancellation, results, diagnostics, reports, source maps, runtime
          identity, target identity, and host-policy differences.

    Completed 2026-08-11: accepted the seven-project source/deployment graph,
    `cem-ml-platform` fixed release group, `cem-ml-v{version}` tags, Cargo version
    authority, Node `^22.12.0 || ^24.0.0`, the representative browser/Node/native
    host matrix, APT/Homebrew/WinGet channels and signing authorities, the
    native/WASM capability gaps, and the bounded typed host protocol. The
    canonical evidence and exact identities are recorded in
    [`cem-ml-deployment-contract.md`](cem-ml-deployment-contract.md). No
    deployment project was scaffolded in this contract-acceptance slice.

- [ ] Stabilize the common library command boundary before packaging it.
    - [ ] Move or expose typed, CLI-independent parse, validate, inspect, convert,
          query, transform, trace, capability, cancellation, and resolver requests
          through `cem_ml`, with native Rust contract tests at the smallest
          relevant layer.
        - [x] Establish the common bounded version/capability manifest and make the
              existing native CLI version adapter consume the library-owned version
              response without changing terminal grammar.
            - Added library-owned `ProductVersion`, runtime identity, target/ABI
              identity bounds, and a versioned 13-operation first-release capability
              matrix covering native, Node/WASM, and browser-worker runtimes. Native
              benchmark support, development-only fixtures, browser fixture exclusion,
              and unavailable schema/plugin mutation remain explicit instead of being
              inferred from command names.
            - The native `version` command now renders the common product-version
              response while preserving its existing terminal text and copyright
              boundary. The public capability projection has stable camel-case fields,
              kebab-case values, exact Cargo version ownership, and native Rust tests
              for matrix behavior, serialization, and identity limits.
            - Restored the dependent CLI gate by treating `ProjectPayload` as the
              childless leaf it is in transform-call protection and by teaching the
              schema-package audit about the intentionally isolated SCSS CLI test file.
              Nx verification passes for common lint/full test/WASM, transform-adapter
              lint and all 89 tests, CLI lint and the full target (493 library plus 114
              integration tests passed; one intentionally ignored), the isolated
              nine-test SCSS target, the focused schema audit, and CLI e2e.
        - [x] Move the high-level query run request/result boundary out of CLI
              dispatch while retaining native query AST, input, result, and source-map
              ownership.
            - Added an owned common `QueryRunRequest` / `QueryRunResponse` boundary for
              data and query sources, identities, native context and bindings, limits,
              resolver/scheduler policy, diagnostics, source maps, and a host-supplied
              abort signal. Typed failures distinguish request-contract errors from
              execution diagnostics and retain both input identities for reporting.
            - Added an `EngineContext` query-runtime registry with built-in CSS Selector
              and XPath preparation/evaluation. The downstream CEM-QL adapter registers
              through the same seam, preserving dependency direction and native owner
              types without a `cem_ml` dependency on `cem_ml_transform_cem_ql`.
            - Reduced CLI query dispatch to source I/O, argument-to-request mapping,
              result exporting, report/output writes, diagnostics, and exit policy.
              Common contract tests cover owned CSS Selector/XPath results, source maps,
              invalid budgets, and host abort propagation; the adapter contract covers
              CEM-QL, and all eight existing CLI query integration cases pass unchanged.
              Nx verification passes for common lint/full test/WASM, transform-adapter
              lint and all 90 tests, CLI lint and all 493 library tests, the focused
              source-boundary guard, and the eight-test CLI query integration target.
        - [x] Establish the root cooperative-cancellation foundation shared by
              common Rust operations and the native CLI.
            - Made `scheduler::AbortSignal` the canonical clone-shared primitive,
              exposed it through `EngineContext`, and retained the plugin signal
              name only as a compatibility re-export.
            - Reused one operation signal at public engine, lifecycle, scheduler,
              resolver, plugin, CSS/SCSS, CEM-QL, CLI input/report/output, and
              source-map boundaries without placing live state in request records.
            - Installed native `SIGINT`/`SIGTERM` ownership and exit status 130;
              added pre-start and focused mid-work cancellation fixtures while
              keeping the common crates compiling for WASM.

            Completed 2026-08-11: the existing implementation supports one root
            cancellation flag and cooperative boundary checks. It does not yet
            implement scoped control, interrupting resource limits, a real parallel
            pool, deep polling in every evaluator, transactional fanout, pause, or
            debugger inspection.
        - [x] Review and accept the canonical worker-pool, scoped cancellation,
              resource-limit, pause/resume, operation-handle, debugger, DAP, and
              stripped-build contract in
              [`cem-ml-operation-control-design.md`](cem-ml-operation-control-design.md).
        - [ ] Implement the canonical operation-control contract in its fixed gate
              order; do not expose later host/debug surfaces over partial common
              semantics.
            - [x] Gate 1 — add `OperationId`, `ExecutionScopeId`, `TaskId`, the
                  mapped execution-scope tree, typed control causes, hierarchical
                  stack/memory/deadline accounting, and the `AbortSignal`
                  compatibility facade; report the new controls and enforcement
                  coverage through the capability manifest.

                Completed 2026-08-11: added stable opaque operation, execution-
                scope, and logical-task identities; an append-mostly mapped scope
                tree with bounded metadata and constrain-only effective policy;
                typed cancellation/resource/queue/worker causes; per-task logical
                stack guards; atomic ancestor memory permits; active-time scope and
                plugin deadlines; and first-reason root cancellation through the
                existing `AbortSignal` API. Normalized run config now owns canonical
                `stackDepth` / `timeoutMs` policy fields and accepted aliases, rejects
                zero memory/stack/timeout limits, and preserves the 256-frame default
                for older policy JSON. Capability contract v2 reports each control's
                exact coverage, the still-sequential executor topology, no accounted
                engine stores yet, and later handles/debug/hard-cancel surfaces as
                unavailable instead of no-ops. Focused control/config/scheduler tests,
                common lint, all 1,841 library tests plus integration suites, and the
                common WASM build pass.
            - [ ] Gate 2 — replace the sequential `WorkerPool` execution model with
                  real bounded native workers, constrain-only per-scope permits,
                  cooperative queue blocking, independent I/O permits, deterministic
                  task paths, staged results, and an ordered commit barrier.
                - [ ] Make report/artifact/primary multi-destination publication
                      transactional so cancellation or failure cannot leave an
                      otherwise successful dispatch partially published.
            - [ ] Gate 3 — add bounded safe-point polling throughout remaining
                  parser and long single-call evaluator paths, beginning with XPath
                  ranges, paths, predicates, quantified/for loops, function calls,
                  template/render recursion, transform stages, and output chunks.
            - [ ] Gate 4 — implement scoped subtree unwind, cleanup, typed delivery
                  to the nearest error boundary, explicit type-compatible recovery,
                  unhandled/fail-fast escalation, and unaffected-sibling behavior.
            - [ ] Add native fixtures at each gate for root/scoped cancellation,
                  stack/memory/timeout/queue failures, worker concurrency and loss,
                  deterministic parallel output, atomic regions, staged output, and
                  exactly one terminal result before advancing to host adapters.
    - [ ] Keep terminal parsing, native filesystem/network adaptation, signals,
          standard streams, and exit-code projection in common `cem_ml_cli`.
    - [ ] Version and bound every host-wire/report projection while preserving
          native typed AST/event/value ownership inside the engine.
        - [ ] Gate 5 — expose the versioned initialize/run/progress/event/result
              envelope as an awaitable operation handle with root/scoped cancel,
              bounded independent event subscriptions, gap reporting, retained stop
              and terminal events, lazy value/artifact handles, and exactly one
              terminal owner.
        - [ ] Gate 6 — behind the default-enabled `debug-control` feature, implement
              pause generations, source/scope breakpoints, all-stop rendezvous,
              immutable stopped snapshots, repeated pause/continue, next/step-in/
              step-out, and bounded thread/stack/scope/variable/native-value
              discovery; cancellation must wake and unwind paused tasks.
        - [ ] Gate 7 — implement DAP as the canonical editor projection plus only
              the versioned `cem/operation`, `cem/executionScopes`, `cem/cancel`,
              `cem/nativeValue`, and `cem/workerTopology` gap requests; expose it via
              explicit `cem-ml debug --stdio|--listen` transports without changing
              ordinary CLI output or cancellation behavior.
        - [ ] Gate 8 — implement Node and browser worker pools with one runtime/WASM
              instance per worker, stable message generations, operation handles,
              all-stop coordination, hard-cancel fallback, single-worker/main-thread
              fallbacks, and native-equivalent transform/query control fixtures.
        - [ ] Gate 9 — add and verify the `--no-default-features` stripped profile:
              debugger APIs, transports, frame capture, and symbols are absent while
              cancellation, stack/memory/timeout enforcement, deterministic output,
              progress, diagnostics, and terminal semantics remain green.

- [ ] Create the separate `@epa-wg/cem-ml` WASM runtime npm deployment
      project.
    - [ ] Add a dedicated Nx project that generates low-level JS/WASM bindings,
          TypeScript declarations, schema-package assets, ABI/capability metadata,
          integrity records, and the synchronized version from common `cem_ml`.
    - [ ] Publish the `./wasm` runtime surface with no npm executable and no
          command/UI policy.
    - [ ] Add an explicit clean-consumer pack/install fixture and Nx checkitem
          proving exports, assets, version/ABI identity, integrity, and direct
          runtime initialization.

- [ ] Create the separate `@epa-wg/cem-ml-cli` universal npm deployment
      project.
    - [ ] Depend on exactly the same version of `@epa-wg/cem-ml` and prove the
          installed consumer resolves one runtime copy.
    - [ ] Add worker-safe `./browser` and Node-hosted `./node` exports plus the
          npm `cem-ml` executable without duplicating engine semantics.
    - [ ] Project the shared command parser, capability discovery, progress,
          cancellation, resolver bridge, reports, signals, and exit policy through
          the appropriate host adapters.
    - [ ] Add explicit browser API, Node API, npm-executable, pack/install, and
          command-round-trip fixtures with Nx verification targets.

- [ ] Create exactly three native CLI deployment projects.
    - [ ] Add `x86_64-unknown-linux-gnu` / `native-linux-amd64`.
    - [ ] Add `aarch64-apple-darwin` through Homebrew /
          `native-macos-arm64`.
    - [ ] Add `x86_64-pc-windows-msvc` / `native-windows-amd64`.
    - [ ] Give each Nx project only its target-specific build, package, sign,
          verify, publish, and install/upgrade/uninstall smoke lifecycle.
    - [ ] Emit target-qualified archives/installers, checksums, signatures, SBOMs,
          provenance, capability/version metadata, and package-channel records.

- [ ] Add the fixed `cem-ml-platform` release family and immutable artifact
      contract.
    - [ ] Synchronize the exact common version and source commit across the common
          crates, both npm deployments, three native deployments, capability
          output, integrity metadata, provenance, SBOMs, and release index.
    - [ ] Stage the complete version-qualified GitHub Release asset set before
          publication; package channels must resolve those immutable assets rather
          than mutable build URLs.
    - [ ] Reject version, dependency, source-commit, target, checksum, signature,
          SBOM, provenance, capability, or release-index drift.

- [ ] Prove and promote the Phase 2.5 deployment gate.
    - [ ] Add an explicit native/WASM parity fixture covering the accepted
          operation matrix, normalized results, diagnostics, reports, source maps,
          capability gaps, progress, cancellation, and runtime/target identity.
    - [ ] Run clean-consumer npm pack/install checks and per-platform
          install/upgrade/uninstall smoke checks through their Nx projects.
    - [ ] Add one Phase 2.5 aggregate Nx target and run the common Rust,
          Node/WASM, package, parity, native-build-where-available, and release
          drift gates before marking the phase complete.

## Current Source Verification Commands

- `yarn nx run cem_ml:lint`
- `yarn nx run cem_ml:test`
- `yarn nx run cem_ml:build:wasm`
- `yarn nx run cem_ml_cli:lint`
- `yarn nx run cem_ml_cli:test`
- `yarn nx run cem_ml_cli:e2e`

The Phase 2.5 aggregate command will be added only after its name and deployment
project graph are accepted. Native target checks remain target-specific; a
missing cross-compile, signing, or package-manager toolchain must be reported as
an explicit unavailable gate rather than silently treated as a pass.

## Deferred Work

### Phase 5 CEM UI Kit

The repository-side ownership plan, native five-mode token import gate, and
48-primitive component inventory are complete and archived. The remaining work
requires reviewed changes in the canonical Figma file.

- [ ] Build and review the `02 Foundations` page from native CEM variables,
      including color, typography, spacing, shape, stroke, layering, and motion
      guidance without raw replacement values.
- [ ] Build the representative `03 Components` pilot for `cem-action`,
      `cem-text-field`, `cem-card`, `cem-nav`, and `cem-dialog`.
    - [ ] Keep variant dimensions independent, use component properties by
          semantic meaning, and test every owned state in all five modes.
    - [ ] Record the pilot fixture and review evidence before expanding to the
          remaining component inventory.
- [ ] Complete `03 Components` for every executable inventory entry, keeping
      inert payloads nested under their consuming visual owners.
- [ ] Build `04 Patterns` for auth, profile, assets, discussion, and settings
      entirely from library instances, then compose `05 Site Demo` from those
      patterns without detached one-off controls.
- [ ] Populate `99 QA`, run offline token/component gates, record the reviewed
      Figma revision and five-mode evidence, and publish the Phase 5 library
      only after raw-value, detached-shape, state, and documentation checks pass.

### Native Theme Compile Gates

- [ ] Run the Swift/Xcode compile gate for
      `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift` when the
      native toolchain is available.
- [ ] Run the Kotlin/Compose Gradle compile gate for
      `packages/cem-theme/dist/lib/token-platforms/android/` when the native
      toolchain is available.
