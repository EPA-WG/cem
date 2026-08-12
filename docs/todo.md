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
        - [ ] Thread one host-supplied cooperative cancellation handle through every
              public operation request and resolver/evaluator path.
            - [x] Make `scheduler::AbortSignal` the canonical clone-shared primitive;
                  expose one Rust-native operation control through `EngineContext`,
                  remove the query-only signal field, and retain the plugin signal
                  name only as a compatibility re-export.
            - [ ] Make every common engine entrypoint reject pre-cancelled work,
                  reuse the request signal in lifecycle and scheduler phases, poll at
                  evaluator/work-unit boundaries, preserve cancellation source maps
                  and scheduler abort traces, and suppress incomplete results.
                - [x] Reuse the operation signal at every public engine boundary,
                      lifecycle/scheduler phase, plugin invocation, CSS/SCSS work
                      unit, and CEM-QL IR/budget unit; preserve typed cancellation
                      and source-map ownership while suppressing cancelled results.
                - [ ] Thread cooperative polling through remaining long single-call
                      evaluators and template/render internals, beginning with XPath,
                      rather than relying only on their enclosing phase boundary.
            - [x] Add cancellation-aware resolver read/list/write boundaries without
                  changing equatable request records; reject work before host I/O,
                  re-check after reads/lists, and prevent cancelled output commits.
            - [ ] Give each CLI dispatch one signal shared by input/query/template
                  loading, engine execution, reports, and output writes; install
                  `SIGINT`/`SIGTERM` ownership in the native executable and project
                  cancellation to a stable non-success exit without partial output.
                - [x] Own one signal per native dispatch, install `SIGINT`/`SIGTERM`
                      handling, reuse it through reads, engine contexts, reports, and
                      writes, and project observed cancellation to exit status 130.
                - [ ] Define and implement staged multi-destination output commits so
                      cancellation between report/artifact/primary destinations cannot
                      leave an otherwise successful dispatch partially published.
            - [ ] Verify native pre-start and mid-operation cancellation for
                  parse/validate/convert/transform/query/resolver/plugin/scheduler
                  paths, then keep the common crate compiling for WASM hosts.

            Completed 2026-08-11: established the common clone-shared operation
            control; removed the query-only signal; connected engine, lifecycle,
            scheduler, resolver, plugin, CLI I/O, and CEM-QL evaluator boundaries;
            and gave the native executable signal ownership with cancellation exit
            status 130. Remaining work is explicit above: deep XPath/template polling,
            staged multi-output commit semantics, complete mid-operation fixtures, and
            the final native/WASM verification gate. WASM operation handles remain
            intentionally deferred to the versioned worker run/cancel envelope.
    - [ ] Keep terminal parsing, native filesystem/network adaptation, signals,
          standard streams, and exit-code projection in common `cem_ml_cli`.
    - [ ] Version and bound every host-wire/report projection while preserving
          native typed AST/event/value ownership inside the engine.
        - [ ] Add the versioned initialize/run/progress/event/result/cancel envelope
              only after the common operation requests expose cancellation and
              bounded result ownership.
            - [ ] Expose WASM operation handles that run in a worker and accept
                  cancel messages; prove transform/query cancellation, exactly one
                  terminal result, and no post-cancel resolver or output commits in
                  browser and Node hosts.
        - [ ] Design resumable debugger control separately from terminal
              cancellation.
            - [ ] Define cooperative pause/resume safe points for parser,
                  resolver, scheduler, query, template, transform, and plugin work;
                  specify which atomic host-I/O/output-commit regions cannot pause
                  and how queued cancellation behaves while paused.
            - [ ] Define runtime discovery for execution threads/tasks, scheduler
                  scopes, call stacks, stack frames, lexical/dynamic scopes,
                  variables, native AST/value handles, and source-map-projected
                  locations without exposing unbounded engine ownership.
            - [ ] Evaluate the Debug Adapter Protocol (DAP) as the canonical debugger
                  projection before defining any CEM-specific protocol; document the
                  capability gaps and use custom requests/events only where CEM
                  scheduler scopes or native semantic values have no DAP mapping.
            - [ ] Specify CLI stdio and browser/Node worker transports, deterministic
                  stopped/continued/terminated events, concurrent-operation identity,
                  bounded inspection, disconnect behavior, and native/WASM parity.
            - [ ] Implement and fixture-test pause, thread/stack/scope discovery,
                  resume, step, cancellation-while-paused, and exactly one terminal
                  result only after the run/cancel envelope is canonical.

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
