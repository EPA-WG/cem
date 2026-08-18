# CEM Elements Phase 3 Substrate Audit

**Status:** completed 2026-08-18 against the locked declaration and registration
contract in [`cem-element-design.md`](./cem-element-design.md).

This audit classifies the current `@epa-wg/cem-elements` substrate, maps its
resolved Nx targets to the roadmap, and identifies the first missing Phase 3A
contract. It is an implementation inventory, not a claim that the Phase 3 browser
production trigger is complete.

## Status vocabulary

- **Implemented:** executable code and focused evidence exist for the bounded
  responsibility named in the row.
- **Partial:** useful implementation exists, but a required Phase 3 contract or
  acceptance leg is missing.
- **Placeholder:** a surface exists without meaningful executable behavior.
- **Deferred:** the code may contain exploratory evidence, but the roadmap does not
  accept it in the current phase.

No placeholder-only substrate was found. The main risk is substantial main-thread
and post-browser implementation being mistaken for the selected worker-backed
Phase 3A topology.

## Implementation and fixture inventory

| Area | Status | Existing evidence | Missing or incorrectly sequenced work |
|---|---|---|---|
| Declaration shape and registration | Implemented | `analyzeDeclarationShape()` enforces inline/source shape. The opaque `CemDeclarationScope` API locks default roots, explicit same-document parents, nearest inherited lookup, aliases, and disposal. `CemElementRuntime` selects explicit/default scopes for inline and external declarations, derives source/language/behavior-version registration identities, calls the pure decision core, and marks CEM-owned document-global constructors. Unit and browser fixtures cover missing behavior identity, same-scope rejection, identical inherited reuse, incompatible inherited/CEM-browser collisions, and foreign browser collisions before mutation. | Worker artifact ownership must preserve the same registration identity, but that is processing-host work rather than a missing declaration-registry rule. |
| Data document | Implemented, bounded helper | `data-document.ts` provides DOM-record and table-row projections with executable Storybook assertions. The runtime stories separately exercise CEM-QL `/datadom` selection. | The helper is story-local rather than the general worker data-AST transport; it must not be treated as that transport. |
| Disposition | Implemented | `disposition.ts`, `disposition.spec.ts`, and `projection.disposition.spec.ts` provide tested run-mode and contract-version decisions. | Worker startup/fallback diagnostics do not yet flow through this policy because the worker host does not exist. |
| Browser projection and DOM ownership | Partial | `projection.ts` implements serializable render plans, deterministic node identities, scoped CSS, materialization, identity-aware range merge, revision metadata, and patch-frame generation/application. Browser stories cover focus and runtime-owned attribute preservation. | Patch frames are produced in the same browser thread; there is no dedicated-worker producer/consumer protocol, startup failure recovery, or stale worker-job cancellation. |
| Processing boundary | Partial | Structured-clone guards, snapshots, render-plan identity, revisions, diff frames, privacy/export behavior, and edge-state primitives have focused unit coverage. The package-private `cem-processing-host-v1` request/response types, monotonic job sequence, root-owner resolver, injected module-worker factory, and pure failure-transition table now lock the worker/fallback boundary. | There is no transport-wired `Worker` host, transferable/binary chunk flow, or browser parity proof between worker and fallback execution. |
| Runtime support | Partial | The package-private `cem-ql-render.ts` initializes the generated `cem_ql` WASM module and compiles/renders canonical CEM-ML; `cem-ql-query.ts` maps query bindings and results. | WASM runs directly on the main thread. The locked single dedicated worker is absent, so Option B is not primary and Option A is not an observable fallback transition. Local/remote streaming and retained worker artifacts are also absent. |
| Browser runtime and resources | Partial | `CemElementRuntime` implements scoped inline and `src` declarations, inert instance islands, CEM-QL rendering, light-DOM patching, events/forms, diagnostics, module URLs, and completed-response JSON/XML/local/location resource slices. | Processing still calls the main-thread WASM support directly. Resource support precedes the worker/artifact boundary and therefore cannot yet prove URI identity and worker/fallback equivalence required by the later URI slice. |
| Storybook/browser evidence | Partial | Browser tests cover declaration guardrails, the focused logical-scope/global-registry contract, data-island isolation, canonical CEM-ML/WASM rendering, payload/slice/form flows, URI/resources, DOM identity, diagnostics, legacy behavior, and eight material examples. | Phase 3A and the six Edge/SSR stories share `cem-elements.stories.ts`; the broad browser target can execute them together even though explicit Edge/SSR acceptance is deferred. The worker/fallback fixture is the next missing focused browser evidence. |
| Legacy parity | Partial | Twelve manifest-backed legacy/CEM-ML file pairs, six executable legacy-XSLT stories, runtime bridge stories, and Rust/TypeScript contract-alignment tests exist. `custom-element-v0` routes through the bounded `custom-element-xslt` compatibility adapter. | `verify-legacy-fixtures` verifies inventory and markers, not rendered equivalence of every paired file. Full browser behavior and accessibility parity remain production-trigger work. |
| Material parity | Partial | Eight manifest-backed pairs cover action, autocomplete, badge, dropdown, icon, icon-link, input, and menu; executable stories cover the eight examples plus scoped-style policy and first paint. | `verify-material-fixtures` is structural. It does not by itself prove all paired-file output, interaction, keyboard, and accessibility equivalence. |
| Edge/SSR | Deferred | Snapshot hydration, rejection/fallback, edge patch frames, export policy, hybrid render-state storage, and supporting unit primitives already exist. | Roadmap Phase 3.5 begins only after the browser worker substrate is stable. These prototypes remain useful but are not a Phase 3A release prerequisite. |

## Resolved Nx target map

The global Vitest plugin is now excluded from this project because `cem-elements`
has two explicit configurations. This retires the broken inferred `test-ci--...`
atomics, which ran unit files through the Storybook-only default configuration.
`cem-elements:test:unit` remains the accepted unit gate and `cem-elements:test`
remains the explicit Storybook browser gate.

| Resolved target(s) | Roadmap ownership | Role and gate status |
|---|---|---|
| `build`, `typecheck` | Phase 3A | Compile and declaration/type safety; both depend on the browser WASM build. |
| `build-deps`, `watch-deps` | Phase 3A authoring support | Nx TypeScript dependency/watch orchestration; not acceptance by themselves. |
| `lint` | Cross-phase quality gate | Required by the Phase 3A aggregate. Its prior 15 module-boundary errors and two warnings are resolved without weakening the project-wide rule. |
| `storybook`, `build-storybook`, `test` | Phase 3A browser evidence | Interactive, static-build, and Playwright-backed Storybook surfaces. The files still contain deferred Edge/SSR stories, but Edge/SSR is no longer an explicit dependency of the browser aggregate. |
| `test:unit` | Phase 3A and shared-boundary evidence | Accepted Node unit gate for declaration, disposition, projection, processing, legacy alignment, query mapping, and CEMT helpers. |
| `verify-substrate` | Phase 3A structural evidence | Parses and idempotently roundtrips four `examples/cem-elements` CEM files through the real CLI; it is not rendered parity. |
| `verify-legacy-fixtures`, `verify-material-fixtures` | Phase 3A parity inventory | Enforce manifests, paired files, dependency order, and markers. They are structural legs, complemented by Storybook rather than equivalent to full parity. |
| `verify-demo-fixtures` | Phase 3A browser evidence | Builds and drives the executable HTML demo set in Chromium. |
| `verify-cemt-pipeline-story` | Phase 3A tooling evidence | Builds Storybook and checks the formatter/colorizer pipeline story; supporting evidence rather than core runtime architecture. |
| `verify:phase3a`, `verify` | Phase 3A aggregate | Current browser gate. It includes lint, typecheck, unit/browser tests, Phase 2 engine legs, structural parity inventories, demos, and CEMT story evidence. `verify` is an alias for this current phase. |
| `verify-edge-ssr` | Phase 3.5 only | Explicitly invoked deferred evidence. It is no longer reachable from `verify` or `verify:phase3a`. |
| `nx-release-publish` | Package release infrastructure | Nx package publication plumbing; not a Phase 3A/3B/3C feature or acceptance leg. |

There are no resolved Phase 3B worker-pool/cache/scheduler targets and no Phase 3C
precompiled-template targets. Adding either before the Phase 3A worker/fallback
contract is green would reverse the accepted sequence.

## Ordered gaps and next decision

1. Add the one-dedicated-worker processing host and deterministic main-thread
   fallback behind one semantic result/diagnostic/patch contract.
2. Prove startup failure, pre-commit execution failure, committed-job suppression,
   cancellation, and worker/fallback semantic parity in the one focused browser fixture.
3. Separate Phase 3.5 story/unit selection before Phase 3.5 becomes active; until
   then keep `verify-edge-ssr` opt-in and outside `verify`.
4. Turn the structural legacy/material inventories into full rendered and
   accessibility acceptance after the Phase 3A architecture is authoritative.
5. Leave the Phase 3B pool/cache/scheduler and Phase 3C precompiled path deferred.

The declaration-scope, registration-identity, and processing-host API decisions are
implemented. The next work item is the smallest worker-backed browser vertical slice.
It should move only canonical inline CEM-ML compile/render/diff work behind the locked
host first, keep DOM projection and legacy compatibility on their existing paths, and
prove the required main-thread fallback with the same handles, diagnostics, full
revision, and patch-frame result before expanding to URI streaming.
