# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).

## Phase 2 — CEM DOM Library (`@epa-wg/cem-dom`)

Brings the existing fixtures in `examples/semantic/` into a real schema → parse → validate → transform pipeline.
Acceptance criteria: [`cem-dom-ac.md`](cem-dom-ac.md). Plan: [`dom-library-plan.md`](dom-library-plan.md). Component
vocabulary: [`component-mvp.md`](component-mvp.md).

### Package scaffold

- [ ] Create `packages/cem-dom` with `package.json`, `project.json`, `tsconfig*.json`, and `README.md`, mirroring
      `packages/cem-components` shape.
- [ ] Wire `build`, `lint`, `test`, and `validate-fixtures` Nx targets.
- [ ] Add the package to the root README package map and `docs/index.md`.
- [ ] When `@epa-wg/cem-dom` lands, instantiate the shared README template (one-line summary, install, primary
      exports, build/test commands, key paths, related docs).

### Schema

- [ ] Author `src/schema/cem.schema.md` covering the vocabulary used by the existing five fixtures: `data-cem-screen`,
      `data-cem-form`, `data-cem-action`, plus implied field/list/thread shapes. Cross-reference component IDs from
      `docs/component-mvp.md`.
- [ ] Define the allowed state attribute set against the `component-mvp.md` state matrix.
- [ ] Compile schema markdown → XHTML via the existing docs pipeline.

### Parser & DOM model

- [ ] Implement `src/parser/parse.ts` — HTML → typed normalized model with semantic roles, state, labels, and refs.
- [ ] Implement query helpers in `src/query/` for roles, state lookups, validation messages, label resolution, and
      reference traversal.
- [ ] Unit tests covering each fixture's parsed shape.

### Validation

- [ ] Implement `src/validate/validate.ts` checking: unknown elements/attributes, invalid state combinations, missing
      accessible names, broken `id`/`for`/`aria-*` references, and unsafe content.
- [ ] Emit `dist/cem-dom.report.md` and `cem-dom.report.json` (mirror the `validate-platforms.mjs` report convention).
- [ ] Add `scripts/validate-fixtures.mjs` that runs validation across `examples/semantic/*.html` and fails non-zero on
      hard violations.

### Transform

- [ ] Author `src/transform/cem-to-ce.xsl` — semantic CEM markup → light-DOM custom-element markup compatible with
      `@epa-wg/custom-element`.
- [ ] Add a Node-side transform helper that runs the XSL over a fixture and returns the rendered HTML string.
- [ ] Snapshot the transform output for each fixture under `test/__snapshots__/`.

### Verification

- [ ] All five fixtures parse, validate clean, and transform successfully end-to-end.
- [ ] `yarn build` includes `cem-dom` build + fixture validation; report shows zero hard violations.
- [ ] Document the round-trip in `packages/cem-dom/README.md` with a worked example using `login.html`.
