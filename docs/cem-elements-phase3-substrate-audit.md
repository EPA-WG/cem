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
| Declaration shape and registration | Partial | `analyzeDeclarationShape()` and its unit cases enforce inline/source shape; `CEM_DECLARATION_REGISTRATION_CONTRACT` and `analyzeDeclarationRegistration()` encode the seven locked duplicate/reuse/collision cases. The opaque `CemDeclarationScope` host API now locks default roots, explicit same-document parents, nearest inherited lookup, aliases, and disposal. | `CemElementRuntime` still owns one flat `Map` and rejects any existing name before compilation. It does not yet select a logical scope, derive the full registration identity, reuse an inherited binding, mark CEM-owned constructors, or call the locked decision core. Behavior-bearing declarations first need a stable host identity that does not hash callback source text. |
| Data document | Implemented, bounded helper | `data-document.ts` provides DOM-record and table-row projections with executable Storybook assertions. The runtime stories separately exercise CEM-QL `/datadom` selection. | The helper is story-local rather than the general worker data-AST transport; it must not be treated as that transport. |
| Disposition | Implemented | `disposition.ts`, `disposition.spec.ts`, and `projection.disposition.spec.ts` provide tested run-mode and contract-version decisions. | Worker startup/fallback diagnostics do not yet flow through this policy because the worker host does not exist. |
| Browser projection and DOM ownership | Partial | `projection.ts` implements serializable render plans, deterministic node identities, scoped CSS, materialization, identity-aware range merge, revision metadata, and patch-frame generation/application. Browser stories cover focus and runtime-owned attribute preservation. | Patch frames are produced in the same browser thread; there is no dedicated-worker producer/consumer protocol, startup failure recovery, or stale worker-job cancellation. |
| Processing boundary | Partial | Structured-clone guards, snapshots, render-plan identity, revisions, diff frames, privacy/export behavior, and edge-state primitives have focused unit coverage. | There is no message envelope, `Worker` host, transferable/binary chunk flow, worker lifecycle, or parity proof between worker and fallback execution. |
| Runtime support | Partial | The package-private `cem-ql-render.ts` initializes the generated `cem_ql` WASM module and compiles/renders canonical CEM-ML; `cem-ql-query.ts` maps query bindings and results. | WASM runs directly on the main thread. The locked single dedicated worker is absent, so Option B is not primary and Option A is not an observable fallback transition. Local/remote streaming and retained worker artifacts are also absent. |
| Browser runtime and resources | Partial | `CemElementRuntime` implements inline and `src` declarations, inert instance islands, CEM-QL rendering, light-DOM patching, events/forms, diagnostics, module URLs, and completed-response JSON/XML/local/location resource slices. | Registration still violates the new scope contract. Resource support precedes the worker/artifact boundary and therefore cannot yet prove URI identity and worker/fallback equivalence required by the later URI slice. |
| Storybook/browser evidence | Partial | Browser tests cover declaration guardrails, data-island isolation, canonical CEM-ML/WASM rendering, payload/slice/form flows, URI/resources, DOM identity, diagnostics, legacy behavior, and eight material examples. | Phase 3A and the six Edge/SSR stories share `cem-elements.stories.ts`; the broad browser target can execute them together even though explicit Edge/SSR acceptance is deferred. A focused registration-scope fixture does not yet exist. |
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

1. Finish registration-identity derivation for declarations carrying the optional
   browser behavior adapter. Template source and language are content-addressable;
   JavaScript callbacks are not, so the host contract must supply a stable behavior
   identity or explicitly forbid cross-scope reuse for behavior-bearing declarations.
2. Add `CemDeclarationScope` selection to `CemElementRuntime`, wire registration
   through the pure decision core, and mark CEM-owned browser constructors before
   adding any new compilation topology.
3. Add the one-dedicated-worker processing host and deterministic main-thread
   fallback behind one semantic result/diagnostic/patch contract.
4. Separate Phase 3.5 story/unit selection before Phase 3.5 becomes active; until
   then keep `verify-edge-ssr` opt-in and outside `verify`.
5. Turn the structural legacy/material inventories into full rendered and
   accessibility acceptance after the Phase 3A architecture is authoritative.
6. Leave the Phase 3B pool/cache/scheduler and Phase 3C precompiled path deferred.

The scope-object decision is now locked: object identity, one default root per
`Document`, optional explicit same-document parent, no DOM-ancestry inference, and
idempotent logical disposal. The next item is a real registration-identity decision.
The recommended direction is a required, non-empty `behaviorIdentity` whenever a
host supplies `CemProducedElementBehavior`; the runtime then content-addresses the
produced tag, resolved template source, template language, and that opaque behavior
version. Behavior-less declarations need no extra option. Hashing callback
`Function#toString()` output or silently treating separate behavior objects as
compatible would not be stable across builds and must remain rejected.
