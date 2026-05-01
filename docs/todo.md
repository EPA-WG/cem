# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).

## Phase 0 — Repo Spine And Docs

Goal (from roadmap): make the repo understandable from the root.

Investigation summary:

- Root `README.md` lines 1–44 are CEM-specific; **lines 45–90 still hold generated Nx boilerplate** ("New Nx
  Repository", "Generate a library", "Run tasks", "Versioning and releasing").
- `docs/index.md` exists and covers project planning, theme docs, components, examples, and release. Missing entries:
  native token-platform outputs and `cem-dom-ac.md`.
- `packages/cem-theme/README.md` and `packages/cem-components/README.md` are **11-line Nx boilerplate** —
  no CEM content, no consistent structure.

### Root README cleanup

- [x] Remove the Nx-generated boilerplate from `README.md` lines 45–90 (Nx logo, "Generate a library", "Run tasks",
      "Versioning and releasing").
- [x] Add a **Package map** section covering `@epa-wg/cem-theme`, `@epa-wg/cem-components`, and the upcoming
      `@epa-wg/cem-dom`, each with a one-line purpose and a link to its package README.
- [x] Add a **Build / test / release quickstart** that points at `yarn build`, `yarn build:css`, `yarn build:theme`,
      `yarn nx test`, and `docs/npm-publish.md`. No raw `npx nx ...` commands.
- [x] Cross-check the existing **Project documentation** list — every link 200-OKs and points at a CEM-owned doc.

### Docs index extensions (`docs/index.md`)

- [x] Add a **Native outputs** section linking the iOS Swift report (`packages/cem-theme/dist/lib/token-platforms/ios/ios-report.md`),
      the Android report (`.../android/android-report.md`), and the per-mode Style Dictionary JSON folder.
- [x] Add a link to `docs/cem-dom-ac.md` under **Project Planning**.
- [x] Confirm every doc referenced from the root README also appears in `docs/index.md`, so the index is the
      single canonical map.

### Package README structure

Define a shared template covering, in order: one-line summary, install command, primary exports, build/test commands,
key paths, related docs.

- [x] Rewrite `packages/cem-theme/README.md` to the template. Replace Nx boilerplate. Cover canonical token paths,
      generated CSS/JSON/TS/Figma outputs, and link to `token-export.md` / `token-figma.md` / `docs-generation.md` /
      `html-compile.md`.
- [x] Rewrite `packages/cem-components/README.md` to the template. State current shell-only status, link to
      `component-mvp.md` and the upcoming `cem-dom` integration.
- [ ] When `@epa-wg/cem-dom` lands, instantiate the same template — tracked under the Phase 2 section.

### Verification (Phase 0 exit criteria)

- [x] Walk the contributor path: root README → build, docs, tokens, components, examples, release. Every
      destination MUST be reachable in ≤ 2 clicks from the root README.
- [x] Grep the docs corpus for deep `dist/` paths that aren't reached via `docs/index.md` — flag any consumer-facing
      references that bypass the documented entry points. _Result: only `npm-publish.md` (intentional CDN URLs) and
      `cem-theme/README.md` "Key paths" (intentional package documentation). No undocumented deep paths._
- [x] Confirm package READMEs only reference build commands that exist in `package.json` / `project.json`.

## Phase 2 — CEM DOM Library (`@epa-wg/cem-dom`)

Brings the existing fixtures in `examples/semantic/` into a real schema → parse → validate → transform pipeline.
Acceptance criteria: [`cem-dom-ac.md`](cem-dom-ac.md). Plan: [`dom-library-plan.md`](dom-library-plan.md). Component
vocabulary: [`component-mvp.md`](component-mvp.md).

### Package scaffold

- [ ] Create `packages/cem-dom` with `package.json`, `project.json`, `tsconfig*.json`, and `README.md`, mirroring
      `packages/cem-components` shape.
- [ ] Wire `build`, `lint`, `test`, and `validate-fixtures` Nx targets.
- [ ] Add the package to the root README package map and `docs/index.md`.

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

## Phase 1 — Native CI Compile Gates (blocked on toolchains)

Tracked here so the gap stays visible; not actionable until macOS/Gradle CI runners are available.

- [ ] Add a Swift compile step to `.github/workflows/ci.yml` that builds `dist/lib/token-platforms/ios/CEMTokens.swift`.
- [ ] Add a Kotlin/Compose compile step that builds `dist/lib/token-platforms/android/`.
