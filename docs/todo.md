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

The starting Nx audit confirmed that `cem_ml` and `cem_ml_cli` provide Rust
build, test, lint, WASM-build, fixture, and release-publish surfaces. The
workspace now also contains the policy-free `@epa-wg/cem-ml` browser/Node WASM
deployment and the first browser/Node worker-host slices of
`@epa-wg/cem-ml-cli`, both with direct common-version drift gates. The universal
CLI's worker operation dispatch, browser resolver/command API, npm executable,
and complete command surface remain open, as do the three
native deployment projects, complete family synchronizer, and fixed
`cem-ml-platform` release group. The common `packages/cem_ml/Cargo.toml` version
remains the authority.

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
            - [x] Gate 2 — replace the sequential `WorkerPool` execution model with
                  real bounded native workers, constrain-only per-scope permits,
                  cooperative queue blocking, independent I/O permits, deterministic
                  task paths, staged results, and an ordered commit barrier.
                - [x] Make report/artifact/primary multi-destination publication
                      transactional so cancellation or failure cannot leave an
                      otherwise successful dispatch partially published.

                Completed 2026-08-12: added one fixed, operation-owned native CPU
                executor and one independent external-I/O executor; logical child
                scopes consume constrain-only permits from themselves and every
                ancestor without creating nested OS pools. Bounded CPU/I/O queues
                implement typed reject, cooperative submitter-side block, and
                spill-to-parent admission while queued work observes cancellation
                and deadlines. Stable task paths, declared dependencies (including
                failure propagation), staged task results, canonical ordered commit,
                caught worker panics, and deterministic trace projection are covered
                by concurrency, queue, deadline, failure, and randomized-delay tests.
                Validate/check now schedule real dependent lifecycle-load and parse-
                validate tasks across documents; the former `WorkerPool` remains only
                as the compatible FIFO phase-trace facade for dependency-ordered
                stages. Capability output now reports the native thread-pool topology
                and hierarchical CPU/queue/I/O enforcement without claiming those
                executors for WASM.

                `ResourceResolver` now advertises direct-only or transactional
                publication and can return prepared commit/rollback writes. CLI
                convert/transform publication preflights every multi-destination
                participant, stages local files beside their destinations, rejects
                direct-only custom resolvers before mutation, commits in stable order,
                and rolls back primary output, artifacts, source maps, and reports on
                failure or cancellation. Single-destination writes retain direct
                resolver semantics; stdout remains a stream rather than a rollback-
                capable destination. Tests cover duplicates, existing-file restore,
                late destination appearance, unsupported resolver preflight, and
                rollback after a later participant fails. All 1,854 `cem_ml` library
                tests, the complete `cem_ml_cli` Nx test/integration graph, both lint
                targets, the final 13-test scheduler and 7-test publication fixture
                sets, and the common WASM build pass.
            - [x] Gate 3 — add bounded safe-point polling throughout remaining
                  parser and long single-call evaluator paths, beginning with XPath
                  ranges, paths, predicates, quantified/for loops, function calls,
                  template/render recursion, transform stages, and output chunks.
                - [x] Add Gate 3 native fixtures proving the fixed work-quota
                      boundary for root cancellation and deadlines across parser
                      tokens/events, XPath and CEM-QL loops, template/render
                      recursion, resolver/plugin acceptance, transform stages, and
                      output chunks; verify unchanged successful output and common
                      WASM compilation.

                Completed 2026-08-12: introduced one common `SafePointPoller` with
                a fixed 64-work-unit maximum between full root/scoped operation-
                control checks. CEM/HTML/XML tokenization, XPath and CEM-QL
                evaluation, template rendering and output encoding, recursive
                transform expansion, and resolver/plugin host boundaries now poll
                cooperatively. Entry, host-dispatch, host-acceptance, and final-
                acceptance boundaries force checks so late cancellation or deadline
                expiry discards partial values, trees, bytes, and host results.
                Native fixtures cover the quota boundary, cancellation and deadline
                suppression, unchanged successful output, recursive paths, and
                resolver/plugin late-result rejection. The four-project Nx test
                graph and its 32 dependency tasks, all four lint targets, and both
                common WASM build targets pass.
            - [x] Gate 4 — implement scoped subtree unwind, cleanup, typed delivery
                  to the nearest error boundary, explicit type-compatible recovery,
                  unhandled/fail-fast escalation, and unaffected-sibling behavior.
                - [x] Add Gate 4 native fixtures for deterministic descendant-first
                      cleanup, stack/memory/task release, nearest-boundary delivery,
                      accepted and rejected cause kinds, subsystem-validated typed
                      replacement, unhandled/fail-fast bubbling, unaffected siblings,
                      root escalation, and exactly-once failure settlement.

                Completed 2026-08-12: execution scopes can now declare bounded,
                subsystem-owned error-boundary descriptors with explicit accepted-
                cause sets or fail-fast policy. Scoped cancellation and resource
                failures stop new subtree work, wait for logical tasks to release
                scheduler permits, then release stack frames, memory charges, and
                registered cleanup actions descendant-first and LIFO per scope.
                The nearest surviving accepting boundary receives a single-use
                delivery token; the owning subsystem validates its typed replacement
                against its own stable result contract before recovery is committed.
                Rejected replacements remain pending for explicit decline/bubbling,
                while unhandled, root, and cleanup-failure paths settle once at the
                root. Unaffected siblings retain their work and resource charges.

                Native fixtures cover task-drain ordering, deterministic cleanup,
                stack/memory release, cause filtering, fail-fast and explicit
                decline, typed replacement rejection and recovery, root escalation,
                cleanup panic promotion, stale-token rejection, and exactly-once
                settlement. All 1,865 `cem_ml` unit tests and its integration suites,
                the four-project Nx test graph and 32 dependency tasks, all four
                lint targets, focused formatting checks, and both common WASM build
                targets pass.
            - [ ] Add native fixtures at each gate for root/scoped cancellation,
                  stack/memory/timeout/queue failures, worker concurrency and loss,
                  deterministic parallel output, atomic regions, staged output, and
                  exactly one terminal result before advancing to host adapters.
    - [ ] Keep terminal parsing, native filesystem/network adaptation, signals,
          standard streams, and exit-code projection in common `cem_ml_cli`.
    - [ ] Version and bound every host-wire/report projection while preserving
          native typed AST/event/value ownership inside the engine.
        - [x] Gate 5 — expose the versioned initialize/run/progress/event/result
              envelope as an awaitable operation handle with root/scoped cancel,
              bounded independent event subscriptions, gap reporting, retained stop
              and terminal events, lazy value/artifact handles, and exactly one
              terminal owner.
            - [x] Add native Gate 5 fixtures for protocol/version and effective-limit
                  discovery, awaitable completion, root/scope/source cancellation,
                  cancellation/completion races, independent filtered cursors, slow-
                  subscriber gaps, retained stopped/continued/terminal events, lazy
                  typed value/artifact ownership and disposal, terminal
                  classification, bounded terminal metadata, and exactly one result.

              Completed 2026-08-12: the common engine now owns protocol-v1
              initialize/run/progress/event/result envelopes, exact effective-limit
              negotiation, awaitable typed results, root/direct-scope/source-selector
              cancellation, and a serialized terminal claim. Independent bounded
              subscriptions report gaps without blocking execution and retain the
              current stop, matching continue, and terminal records outside their
              rings. Native values and artifacts remain typed
              behind operation-bound lazy handles; terminal diagnostics, recovered
              failures, artifact references, payloads, subscriptions, and handles
              all enforce disclosed caps and report truncation counts.

              Eight native fixtures cover version rejection and stricter limit
              discovery, future wakeup and terminal races, repeated cancellation/
              completion races, target resolution with no invalid-state mutation,
              filtered cursor gaps and critical retention, typed handle ownership
              and disposal, cap enforcement, and terminal classification. All 1,873
              `cem_ml` tests and the complete `cem_ml_cli` test target pass, as do
              the CEM-QL and transform suites exercised by the four-project graph,
              the serial retry of its one shared-Cargo-artifact race, all four lint
              targets, and both common WASM builds.
        - [x] Gate 6 — behind the default-enabled `debug-control` feature, implement
              pause generations, source/scope breakpoints, all-stop rendezvous,
              immutable stopped snapshots, repeated pause/continue, next/step-in/
              step-out, and bounded thread/stack/scope/variable/native-value
              discovery; cancellation must wake and unwind paused tasks.
            - [x] Add native Gate 6 fixtures for compiled/active capability and
                  dependency feature forwarding, manual/scope/source breakpoint
                  resolution, persistent and conditional hits, all-stop queued/
                  running/external-wait/atomic rendezvous, repeated generations,
                  stale and foreign stop rejection, next/step-in/step-out dependency
                  closure, breakpoint precedence while stepping, bounded immutable
                  thread/frame/scope/variable/native-value inspection, stopped-time
                  exclusion, event retention, and cancellation/terminal wakeup.

              Completed 2026-08-12: `cem_ml` now compiles debug control by default
              behind an explicitly forwarded `debug-control` feature while every
              dependent crate disables implicit dependency defaults. Hosts activate
              debug control per operation with negotiated page, preview, breakpoint,
              and suspended-snapshot limits. The common control core owns persistent
              manual, exact-source, and execution-scope triggers; conditional and
              counted hits; generation-bound stop tokens; scheduler-aware all-stop
              rendezvous for queued, running, external-wait, and atomic tasks; and
              repeated continue and dependency-closure stepping where breakpoints
              retain precedence. Logical task IDs are debugger threads and physical
              worker IDs remain optional snapshot metadata.

              Each completed all-stop publishes one immutable, byte-bounded
              snapshot with paged threads, logical frames, execution scopes,
              variables, cycle-safe values, and operation-owned typed native values.
              Resume invalidates snapshot references, stale and foreign tokens fail
              closed, paused time is excluded from active deadlines, critical debug
              events survive subscription-ring gaps until their breakpoint is
              removed, and cancellation or terminal completion wakes parked tasks.
              `cem_ql` supplies the bounded boolean condition evaluator over captured
              read-only `frame`, `scope`, `task`, and lexical values without host
              resolvers or mutation hooks.

              Ten native fixtures plus the CEM-QL adapter fixture cover the Gate 6
              contract. The complete 1,883-test `cem_ml` suite, CEM-QL, 91-test
              transform, and full CLI suites pass; default and no-default-feature
              four-crate builds, all four lint targets, and both common WASM builds
              also pass.
        - [x] Gate 7 — implement DAP as the canonical editor projection plus only
              the versioned `cem/operation`, `cem/executionScopes`, `cem/cancel`,
              `cem/nativeValue`, and `cem/workerTopology` gap requests; expose it via
              explicit `cem-ml debug --stdio|--listen` transports without changing
              ordinary CLI output or cancellation behavior.
            - [x] Add native Gate 7 fixtures for initialize/capability negotiation,
                  source breakpoint replacement and executable locations, stopped/
                  continued events, threads, paged stack frames, scopes, variables,
                  stepping, root termination, scoped `cem/cancel`, the remaining
                  versioned `cem/` projections, launch/attach disconnect ownership,
                  DAP framing, loopback-only TCP binding, ordinary CLI parity, and
                  stripped-build command/API absence, including a non-BMP source
                  fixture proving canonical UTF-16 editor columns.

              Completed 2026-08-12: the default-enabled `debug-control` feature now
              exposes a typed DAP session over operation handles, with negotiated
              one-based UTF-16 editor coordinates, path/URI projection, bounded
              positive protocol identifiers, standard launch/attach/configuration,
              breakpoint replacement and locations, threads, stacks, scopes,
              variables, pause/continue/stepping, termination, cancellation, and
              disconnect ownership. Only the five accepted, versioned `cem/`
              requests extend the standard protocol, and conditional-breakpoint
              capability is derived from the bounded host evaluator.

              `cem-ml debug` provides explicit stdio and loopback-only TCP hosts.
              Stdio remains exclusively DAP-framed, launched commands wait for
              `configurationDone`, ordinary dispatch output is projected as output
              events without changing the command contract, and debug APIs plus the
              CLI command disappear from no-default-feature builds. Native fixtures
              cover protocol negotiation and failure shapes, non-BMP coordinates,
              source and stopped-state projections, custom requests, ownership,
              framing, transport policy, and CLI output parity.

              The complete 1,887-test `cem_ml` suite, full CLI, CEM-QL, and 91-test
              transform suites pass. Default native builds for all four projects,
              stripped core and CLI builds, both common WASM builds, final lint
              targets, and a live eight-frame stdio launch/configuration/termination
              session also pass.
        - [x] Gate 8 — implement Node and browser worker pools with one runtime/WASM
              instance per worker, stable message generations, operation handles,
              all-stop coordination, hard-cancel fallback, single-worker/main-thread
              fallbacks, and native-equivalent transform/query control fixtures.
            - [x] Add the versioned common worker envelope and coordinator fixtures
                  first: stable slot/generation/sequence validation, operation/scope/
                  task/subscription/retained-handle routing, transferable-buffer
                  bounds, all-stop acknowledgement, replacement invalidation, late-
                  message rejection, and exactly-one terminal claim.

              Completed 2026-08-12: common `cem_ml` now exposes a host-neutral
              worker protocol layered over the existing operation-host envelope.
              Nonzero slot/generation addresses, strict per-generation sequences,
              initializing/ready lifecycle, bounded unique transferable-buffer
              descriptors, and coordinator-owned operation/scope/task/subscription/
              retained-handle routes fail closed before payload observation.
              Debug builds additionally route stop-local snapshot references and
              require every participating worker to classify one stop generation
              as parked or external-wait before completion.

              Worker replacement advances the slot generation, resets initialization
              and sequencing, invalidates every previous route and active rendezvous,
              reports affected operations, and rejects late old-generation messages.
              Terminal arbitration retains exactly one summary. Five native fixtures
              cover wire shape/bounds, non-mutating sequence/version rejection,
              routing and replacement, complete all-stop coordination, and terminal
              races. All 1,892 common tests and integration suites pass, as do common
              lint, the common WASM build, and the stripped common-core build.
            - [x] Implement the bounded Node worker-thread host with one initialized
                  runtime per worker and the accepted single-worker fallback.

              Completed 2026-08-12: `packages/cem-ml-cli-npm` now exposes a
              bounded `@epa-wg/cem-ml-cli/node` worker-thread pool. Each stable
              slot/generation owns one independently initialized
              `@epa-wg/cem-ml/wasm` runtime, validates sequence-one initialization
              through the versioned common worker/operation envelope, and derives
              protocol limits and Node-pool capability semantics from Rust-owned
              exports. The default pool is capped at eight available processors,
              explicit one-worker mode is supported, and failed multi-worker
              initialization is torn down before the accepted one-worker fallback.
              Focused and packed clean-consumer fixtures prove bounded policy,
              stable identities, distinct worker runtimes, exact versions, and one
              resolved runtime-package copy. Operation dispatch, replacement, and
              hard-cancel behavior remain intentionally assigned to the later
              transform/query control checkitem.
            - [x] Implement the bounded browser dedicated-worker host with one WASM
                  instance per worker and the accepted dedicated-worker/main-thread
                  fallback chain, without requiring shared-memory WASM.

              Completed 2026-08-13: `@epa-wg/cem-ml-cli/browser` now creates a
              bounded dedicated-worker pool sized by the accepted browser host
              default and explicit policy limits. Every stable slot/generation
              initializes one isolated web WASM runtime through the common worker
              and operation envelopes, validates Rust-owned protocol bounds and
              browser-pool capabilities, retains the exact runtime ABI/version, and
              reports worker failures without requiring `SharedArrayBuffer` or
              cross-origin isolation.

              Pool initialization failure falls back to one dedicated worker; an
              unavailable or failed worker host falls back to one main-thread WASM
              runtime with the truthful sequential capability projection. A real
              Chromium Nx fixture proves two distinct worker runtimes, explicit and
              fallback single-worker modes, both main-thread fallback causes,
              policy-bound rejection, stable identities, version/capability shape,
              and the absence of shared-memory requirements. Common lint, all 1,892
              unit tests plus integration suites, common WASM compilation, both npm
              lints, low-level runtime verification, and browser/Node CLI deployment
              verification pass. Hard cancellation and operation dispatch remain
              unavailable and are assigned to the next checkitem.
            - [x] Add hard-cancel termination/replacement and native-equivalent Node,
                  browser, single-worker, and main-thread transform/query control
                  fixtures, including all-stop and late-message behavior.
                - [x] Add a common resumable transform/query operation driver with
                      bounded start/poll/control/event/terminal steps and WASM
                      bindings so message-passing workers and main-thread fallback
                      can service controls between deterministic work chunks.
                    - [x] Add the coordinator-owned, versioned, bounded
                          `OperationWorkPacket`/`OperationWorkResult` contract and
                          native fixtures for multi-worker routing, deterministic
                          staged commit, cancellation, all-stop, replacement
                          invalidation, and stale-result rejection.
                - [x] Add native-first driver fixtures for success, cooperative
                      cancellation, pause/continue/step, exactly-one terminal, and
                      deterministic transform/query output across chunk sizes.
                - [x] Add Node, Chromium, single-worker, and main-thread host
                      fixtures for transform/query control, worker hard-cancel
                      termination/replacement, all-stop rendezvous invalidation,
                      and rejection of late-generation messages.

              Completed 2026-08-13: common `cem_ml` now owns a resumable
              transform/query driver and versioned, bounded stateless work packets.
              The coordinator retains continuation, deterministic staged commit,
              cancellation, all-stop state, terminal arbitration, and retry after
              generation replacement; worker runtimes validate generation and
              sequence metadata before accepting payloads. Native fixtures prove
              transform/query parity across packet budgets and out-of-order results,
              pause/continue/step, cancellation, replacement retry, stale-result
              rejection, and exactly-one terminal publication.

              Node and browser pools expose awaitable operation handles with event
              subscriptions and cooperative controls. Dedicated workers terminate
              and advance generation after the bounded hard-cancel grace, while the
              browser main-thread fallback executes the same packets and truthfully
              reports hard cancellation unavailable. Node and real Chromium fixtures
              cover multi-worker, one-worker, and main-thread transform/query runs,
              deterministic commit order, all-stop stepping, hard replacement,
              unrelated-operation retry, and late-result rejection. The complete
              1,900-test native suite plus all integrations, common lint and WASM
              compilation, both npm lints, low-level runtime ABI verification, Node
              fixtures, Chromium fixtures, and CLI distribution verification pass.
        - [x] Gate 9 — add and verify the `--no-default-features` stripped profile:
              debugger APIs, transports, frame capture, and symbols are absent while
              cancellation, stack/memory/timeout enforcement, deterministic output,
              progress, diagnostics, and terminal semantics remain green.
            - [x] Add explicit cached Nx targets for stripped common native tests,
                  common WASM compilation, native CLI build/tests, and low-level
                  WASM runtime generation without transitive default features.
            - [x] Add native stripped-profile fixtures for capability truthfulness,
                  cancellation, stack/memory/timeout failures, deterministic staged
                  output, progress/diagnostics, and exactly-one terminal settlement.
            - [x] Add compile-surface, CLI-help, native-symbol, WASM-binding, and
                  package-manifest checks proving debugger APIs, transports, frame/
                  variable capture, DAP, and `cem/` debugger requests are absent.
            - [x] Give default and stripped WASM artifacts distinct profile-qualified
                  ABI/capability identities, target/cache roots, integrity manifests,
                  and verification so release packaging cannot substitute profiles.

              Completed 2026-08-13: added isolated cached Nx build/test targets for
              stripped common native, common WASM, native CLI, and low-level release
              WASM artifacts. Default and stripped packages now use distinct target
              roots, output roots, `debug-control` / `stripped` feature identities,
              profile-qualified ABI strings, capability projections, and complete
              SHA-256 integrity manifests. Native stripped-only fixtures prove
              truthful debug rejection while cancellation, stack, memory, deadline,
              deterministic resumable query commit, progress/diagnostic events, and
              exactly-one terminal settlement remain available.

              The aggregate verifier proves debugger Rust imports do not compile,
              the CLI command/transport and native symbols are absent, generated
              pause/acknowledge/continue/step WASM bindings and declarations are
              absent, and transitive Cargo features stay stripped. Both 1,900-test
              default and 1,888-test stripped common suites plus integrations pass;
              the stripped CLI passes 501 unit and 114 integration tests with one
              documented ignored recursive fixture. Default runtime, clean-consumer,
              and all three lint targets pass. The stripped WASM is 43,446 bytes
              smaller per browser/Node target and the stripped native CLI is
              15,305,392 bytes smaller than its default counterpart.

- [x] Create the separate `@epa-wg/cem-ml` WASM runtime npm deployment
      project.
    - [x] Add a dedicated Nx project that generates low-level JS/WASM bindings,
          TypeScript declarations, schema-package assets, ABI/capability metadata,
          integrity records, and the synchronized version from common `cem_ml`.
    - [x] Publish the `./wasm` runtime surface with no npm executable and no
          command/UI policy.
    - [x] Add an explicit clean-consumer pack/install fixture and Nx checkitem
          proving exports, assets, version/ABI identity, integrity, and direct
          runtime initialization.
    - Completed 2026-08-12: `packages/cem-ml-npm` now builds release-mode web
      and Node `wasm-bindgen` loaders from `cem_ml`, exposes both through the
      conditional `./wasm` subpath, ships 25 runtime schema packages, derives
      both capability projections from a common Rust-owned WASM export, records
      SHA-256 integrity for every artifact, and rejects Cargo/npm or resolved
      crate/CLI `wasm-bindgen` version drift. Its Nx `build`, `lint`, `test`,
      `verify`, `package`, `verify:consumer`, and aggregate `check` targets cover
      generated declarations and ABI, Node/browser initialization, tarball
      installation in a clean temporary consumer, metadata/assets, and the
      absence of `bin`, command, UI, and independent engine dependencies.

- [ ] Create the separate `@epa-wg/cem-ml-cli` universal npm deployment
      project.
    - [x] Depend on exactly the same version of `@epa-wg/cem-ml` and prove the
          installed consumer resolves one runtime copy.

      Started 2026-08-12: the Node-host slice establishes the publishable Nx/npm
      project, exact common-version dependency, supported-Node engine boundary,
      package/verification targets, and clean installation proof. The browser worker
      slice now also exposes `./browser`; the parent stays open until the shared
      command service, npm executable, and complete cross-host fixtures land.
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
- `yarn nx run @epa-wg/cem-ml:check`

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
