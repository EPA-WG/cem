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

No placeholder-only substrate was found. The canonical inline vertical slice now
uses the selected worker-backed Phase 3A topology; the main sequencing risk has moved
to extending that boundary to URI/resource work without bypassing its identities,
cancellation, or retained-plan protocol.

## Implementation and fixture inventory

| Area | Status | Existing evidence | Missing or incorrectly sequenced work |
|---|---|---|---|
| Declaration shape and registration | Implemented | `analyzeDeclarationShape()` enforces inline/source shape. The opaque `CemDeclarationScope` API locks default roots, explicit same-document parents, nearest inherited lookup, aliases, and disposal. `CemElementRuntime` selects explicit/default scopes for inline and external declarations, derives source/language/behavior-version registration identities, calls the pure decision core, and marks CEM-owned document-global constructors. Unit and browser fixtures cover missing behavior identity, same-scope rejection, identical inherited reuse, incompatible inherited/CEM-browser collisions, and foreign browser collisions before mutation. Retained worker artifacts carry inline/fragment/URL/specifier source refs plus resolver and scope-policy identity. | Anonymous instance `src` remains migration work; declaration-registry rules are unchanged. |
| Data document | Implemented, bounded helper | `data-document.ts` provides DOM-record and table-row projections with executable Storybook assertions. The runtime stories separately exercise CEM-QL `/datadom` selection. | The helper is story-local rather than the general worker data-AST transport; it must not be treated as that transport. |
| Disposition | Implemented | `disposition.ts`, `disposition.spec.ts`, and `projection.disposition.spec.ts` provide tested run-mode and contract-version decisions. Stable worker startup/execution fallback diagnostics surface through the normal declaration/render diagnostic channel, while URI and resource resolution retain explicit resolver/context/policy stamps. | Broader host policy disposition remains a future extension, not an inline/URI worker rule. |
| Browser projection and DOM ownership | Implemented for canonical CEM-ML | `projection.ts` implements serializable render plans, deterministic node identities, scoped CSS, materialization, identity-aware range merge, revision metadata, buffered patch-frame validation/application, focus/selection restoration, desired-attribute reconciliation, and atomic target-mismatch recovery. Worker output lowers interpolated `<http-request>` nodes to clone-safe controls before retained-plan diffing, so transient controls never enter browser DOM. | Explicit superseded render-job cancellation still needs the same transaction guarantees. |
| Processing boundary | Partial | Structured-clone guards, chunked text sources, snapshots, render-plan identity, revisions, resource controls, diff frames, privacy/export behavior, and edge-state primitives have focused unit coverage. The package-private `cem-processing-host-v1` transport wires one module worker per logical root to the same retained compile/render-diff engine as main-thread fallback; monotonic requests, root ownership, URI/resolver cache identity, injected construction, fallback, and scope disposal are executable. | Transferable binary source chunks and runtime-driven cancellation of superseded render jobs remain. |
| Runtime support | Implemented for canonical CEM-ML URI and HTTP Phase 1 paths | `processing-engine.ts` retains inline/URI artifacts and DOM-only plans around the generated `cem_ql` WASM module in both worker and fallback modes, returns clone-safe HTTP controls, and preserves source-map refs. `cem-ql-query.ts` maps query bindings/results. The cached package build vendors byte-identical CEM-QL assets and includes the engine, host runtime, and worker entry in the verified npm archive. | Progressive resource AST consumption remains Phase 2; Phase 1 materializes stream-derived projections before render. |
| Browser runtime and resources | Implemented for URI declarations and Phase 1 HTTP | `CemElementRuntime` streams or adapts declaration sources, retains fragment/URL/specifier plus resolver identities, resolves imported-template resources against the imported base URL, and routes canonical CEM-ML through the scope-owned worker host. The main thread owns HTTP resolution/policy, multi-chunk response loading, abort/stale guards, portable lifecycle envelopes, JSON/XML projections, and final patch application. Legacy and the other resource primitives retain their established paths. | `module-url`, `local-storage`, and `location-element` worker controls remain outside this bounded HTTP slice. |
| Storybook/browser evidence | Implemented through URI and HTTP Phase 1 | The focused worker fixture constructs a real module worker, separately rooted startup fallback, and post-handshake execution failure, then proves semantic output, patch identity, focus/selection, and inert-island preservation. URI fixtures cover local, document-relative, absolute, and module-map sources plus imported-resource base resolution. HTTP fixtures cover portable states, multi-chunk JSON/XML projections, `cem:for-each`, abort, and stale-response rejection. All 97 Chromium stories remain green. | Phase 3A and the six Edge/SSR stories still share `cem-elements.stories.ts`; phase-specific Edge/SSR selection remains deferred until Phase 3.5 activates. |
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
| `verify-package` | Phase 3A distribution evidence | Depends on the package build, verifies byte-identical package-local CEM-QL ESM/declaration/WASM assets and rewritten runtime imports, inspects the npm archive, and imports it from a clean temporary consumer. |
| `verify:phase3a`, `verify` | Phase 3A aggregate | Current browser gate. It includes lint, typecheck, unit/browser tests, Phase 2 engine legs, structural parity inventories, demos, and CEMT story evidence. `verify` is an alias for this current phase. |
| `verify-edge-ssr` | Phase 3.5 only | Explicitly invoked deferred evidence. It is no longer reachable from `verify` or `verify:phase3a`. |
| `nx-release-publish` | Package release infrastructure | Nx package publication plumbing; not a Phase 3A/3B/3C feature or acceptance leg. |

There are no resolved Phase 3B worker-pool/cache/scheduler targets and no Phase 3C
precompiled-template targets. Adding either before the Phase 3A worker/fallback
contract is green would reverse the accepted sequence.

## Ordered gaps and next work

1. Wire superseded render jobs to the locked cancel operation and preserve the same
   no-partial-commit recovery behavior already proven by the inline slice.
2. Separate Phase 3.5 story/unit selection before Phase 3.5 becomes active; until
   then keep `verify-edge-ssr` opt-in and outside `verify`.
3. Turn the structural legacy/material inventories into full rendered and
   accessibility acceptance after the Phase 3A architecture is authoritative.
4. Leave the Phase 3B pool/cache/scheduler and Phase 3C precompiled path deferred.

The declaration-scope, registration-identity, processing-host API, canonical
inline/URI worker paths, and Phase 1 `<http-request>` extension are implemented. The
next substrate gap is wiring superseded render jobs to the locked cancel operation
without changing the proven revision, fallback, or atomic patch semantics. Legacy
compatibility remains on its established path.
