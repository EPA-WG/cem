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
| Declaration shape and registration | Implemented | `analyzeDeclarationShape()` enforces inline/source shape. The opaque `CemDeclarationScope` API locks default roots, explicit same-document parents, nearest inherited lookup, aliases, and disposal. `CemElementRuntime` selects explicit/default scopes for inline and external declarations, derives source/language/behavior-version registration identities, calls the pure decision core, and marks CEM-owned document-global constructors. Unit and browser fixtures cover missing behavior identity, same-scope rejection, identical inherited reuse, incompatible inherited/CEM-browser collisions, and foreign browser collisions before mutation. Retained worker artifacts carry that registration identity. | URI artifact identity must add resolver/source identity in the next bounded extension without changing declaration-registry rules. |
| Data document | Implemented, bounded helper | `data-document.ts` provides DOM-record and table-row projections with executable Storybook assertions. The runtime stories separately exercise CEM-QL `/datadom` selection. | The helper is story-local rather than the general worker data-AST transport; it must not be treated as that transport. |
| Disposition | Implemented | `disposition.ts`, `disposition.spec.ts`, and `projection.disposition.spec.ts` provide tested run-mode and contract-version decisions. Stable worker startup/execution fallback diagnostics now surface through the normal declaration/render diagnostic channel. | URI/resource policy disposition remains part of the next bounded extension, not a missing inline-worker rule. |
| Browser projection and DOM ownership | Implemented for the inline vertical slice | `projection.ts` implements serializable render plans, deterministic node identities, scoped CSS, materialization, identity-aware range merge, revision metadata, buffered patch-frame validation/application, focus/selection restoration, desired-attribute reconciliation, and atomic target-mismatch recovery. | URI/resource transient controls and explicit superseded-job cancellation still need the same transaction guarantees. |
| Processing boundary | Partial | Structured-clone guards, snapshots, render-plan identity, revisions, diff frames, privacy/export behavior, and edge-state primitives have focused unit coverage. The package-private `cem-processing-host-v1` transport now wires one module worker per logical root to the same retained compile/render-diff engine as main-thread fallback; monotonic requests, root ownership, injected construction, fallback, and scope disposal are executable. | URI source streams, transferable/binary chunks, and runtime-driven cancellation of superseded work remain. |
| Runtime support | Implemented for canonical inline CEM-ML | `processing-engine.ts` retains artifacts/plans around the generated `cem_ql` WASM module in both worker and fallback modes; `cem-ql-query.ts` maps query bindings/results. The cached package build vendors byte-identical CEM-QL assets and includes the engine, host runtime, and worker entry in a verified 53-file npm archive. | Local/remote streaming and URI/resource artifact retention are intentionally not routed through the worker yet. |
| Browser runtime and resources | Partial | `CemElementRuntime` routes canonical inline CEM-ML through the scope-owned worker host, consumes committed patch frames on the main thread, and retains its existing scoped `src`, resource, legacy, event, form, and diagnostic behavior. | URI declarations and transient resource directives stay on the established path until the next item adds their source/abort protocol without regressing stale-response behavior. |
| Storybook/browser evidence | Implemented for the inline vertical slice | The focused fixture constructs a real module worker, a separately rooted throwing-factory startup fallback, and a post-handshake execution failure, compares semantic output, rerenders through patches, and proves identity, focus/selection, and inert-island preservation. All 97 Chromium stories remain green. | Phase 3A and the six Edge/SSR stories still share `cem-elements.stories.ts`; phase-specific Edge/SSR selection remains deferred until Phase 3.5 activates. |
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

1. Move resolved URI declaration sources through the retained processing host while
   preserving source-ref, resolver, scope-policy, artifact, and source-map identity.
2. Define transient resource-control handling at the worker diff boundary, then prove
   remote/local streaming, abort/stale-response protection, JSON/XML projections, and
   the fixture-backed `cem:for-each` flow without transporting live browser handles.
3. Wire superseded render jobs to the locked cancel operation and preserve the same
   no-partial-commit recovery behavior already proven by the inline slice.
4. Separate Phase 3.5 story/unit selection before Phase 3.5 becomes active; until
   then keep `verify-edge-ssr` opt-in and outside `verify`.
5. Turn the structural legacy/material inventories into full rendered and
   accessibility acceptance after the Phase 3A architecture is authoritative.
6. Leave the Phase 3B pool/cache/scheduler and Phase 3C precompiled path deferred.

The declaration-scope, registration-identity, processing-host API, and canonical
inline worker vertical slice are implemented. The next work item is the bounded URI
declaration and `<http-request>` extension: reuse the same handles, full revisions,
diagnostics, fallback, and patch transactions while adding source streaming and
abort/stale-response semantics. Legacy compatibility remains on its established path.
