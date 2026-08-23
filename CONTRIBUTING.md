# Contributing to CEM

Work from `develop`, follow `AGENTS.md` and `CLAUDE.md`, and keep changes inside
the owning package. Use conventional commits (`fix:`, `feat:`, or an explicit
breaking-change footer). Do not publish packages, create release tags, or alter
protected release assets from a development change.

## Development checks

Use the workspace package manager and Nx targets so task dependencies and cache
inputs remain truthful:

```bash
yarn nx show project <project> --json
yarn nx run <project>:lint
yarn nx run <project>:test
yarn nx run <project>:build
```

Run the smallest owning target while developing, then the package aggregate.
Browser-backed targets require Playwright Chromium. Native Swift and Android
compile claims require their documented supported hosts; static inspection is
not a substitute.

## Token specifications and native projections

Edit canonical token Markdown under `packages/cem-theme/src/lib/tokens/`.
Preserve the manifest table contract in `CLAUDE.md`, including the final `tier`
column and token manifest index. Do not hand-edit generated CSS, DTCG JSON,
Swift, Android, or Figma files.

Run the theme lint/tests and the Phase 8 aggregate. A public token rename,
removal, type/semantic change, native package-layout change, or minimum-platform
increase also updates `docs/migrations-and-deprecations.md`, the changelog input,
and the version decision in `docs/versioning-and-compatibility.md`.

## Components and application UI

Reusable interactive behavior belongs in `@epa-wg/cem-components` on the
`cem-elements`/`@epa-wg/custom-element` light-DOM substrate. Update the primitive
declaration, reference docs, accessibility/convention contracts, examples,
state matrix, styles, browser tests, and generated catalog together. CEM Site or
Studio may own orchestration and persistence, but must not replace an available
shared control with an application-local widget.

Run the component `verify` target and the relevant application verifier. Treat a
changed public name, state, event, keyboard rule, accessibility meaning, or
required markup shape as a compatibility decision.

## CEM-ML, CEM-QL, CLI, and schemas

Start transformation behavior with the smallest Rust fixture/test described in
`CLAUDE.md`, then verify the WASM/browser/CLI integration. Schema identity is its
stable URI plus descriptor SemVer; keep generated artifacts synchronized with
the descriptor. The CEM-ML platform version comes only from
`packages/cem_ml/Cargo.toml` and is synchronized through its Nx targets—never
edit downstream runtime, CLI, Studio, or native versions independently.

Changes to commands or capabilities must retain normalized native/WASM parity,
diagnostics, source maps, exit policy, progress, and cancellation semantics.

## Docs, examples, Studio, and generated files

Documentation links and examples are executable release contracts. Update the
owning example and verifier with the implementation; do not document an export
or capability that is absent from its package. Studio changes retain its exact
CEM-ML dependency, deterministic static build, single runtime copy, service
worker update metadata, accessibility/security boundaries, and clean-consumer
package check.

When adding a fixture, first add an explicit actionable fixture item to
`docs/todo.md`, then implement its positive and negative assertions. Generated
reports belong under `dist/reports`; checked-in source inventories and policies
must remain human-reviewable.

## Deprecations and releases

Every deprecation records its owner, introduction version, replacement,
searchable registry/report entry, migration instructions, and earliest removal.
Do not remove a supported form until its published window and zero-usage gate
both pass.

Before proposing a release, run:

```bash
yarn nx run @epa-wg/cem:verify:phase9-readiness --parallel=1
```

The readiness and Phase 9 closure targets are credential-free. Registry,
GitHub Release, and static/PWA publication are deliberately deferred in
`docs/wishlist.md` and are not roadmap-closure requirements. If publication is
resumed, the protected workflows must publish the exact verified archives and
record immutable public evidence; retries may retain identical bytes but never
replace them.
