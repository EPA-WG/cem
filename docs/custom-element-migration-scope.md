# `@epa-wg/custom-element` Migration Scope

This is the Phase 3.6 scope and branch strategy for moving the legacy
`@epa-wg/custom-element` package from `/home/suns/aWork/custom-element/` into this
workspace as `packages/custom-element/`.

## Scope

Phase 3.6 migrates the published package and replaces its next-major implementation
with an adapter over the parity-proven `cem-element` substrate.

In scope:

- preserve the published package name, public `<custom-element>` tag, browser module
  entrypoints, IDE metadata, docs, demos, license, and release metadata;
- import the legacy repository history where practical, using the local checkout as
  the history source and the currently installed npm package as the published-baseline
  source;
- keep `custom-element.js` and package-level browser imports usable while the
  implementation moves behind the adapter;
- translate legacy declarations (`tag`, `src`, inline templates, data islands, slices,
  host attributes, and event-to-data wiring) into the same runtime records used by
  `packages/cem-elements`;
- make compatibility decisions for adoption-phase legacy gaps from
  [`packages/cem-elements/docs/legacy-parity-inventory.md`](../packages/cem-elements/docs/legacy-parity-inventory.md);
- rewire downstream workspace consumers, especially `packages/cem-theme`, without
  breaking HTML generator workflows.

Out of scope for the first migration pass:

- reintroducing the legacy XSLT/XPath engine as a second rendering engine;
- changing the public semantics of `cem-element`;
- changing material component authoring before the migrated package fixtures are green;
- relying on the old package demos as the primary test runner. They are parity sources;
  Storybook/package fixtures remain the acceptance surface.

## Baseline Sources

Use two baselines because the local source checkout and workspace dependency are not
currently the same version:

- **History source:** `/home/suns/aWork/custom-element/`, local Git repo
  `git@github.com:EPA-WG/custom-element.git`, `main` at `0282a74`, package version
  `0.0.37`, 273 commits, release tags through `0.0.37`.
- **Consumed package baseline:** root `package.json` currently depends on
  `@epa-wg/custom-element@0.0.39`. The next baseline task must inspect the installed
  npm package contents before code import so version `0.0.38`/`0.0.39` deltas are not
  lost.

## Branch Strategy

Use staged branches so history import, adapter implementation, downstream rewiring,
and release readiness can be reviewed independently.

1. `phase-3.6/custom-element-scope`
   - Docs-only planning branch.
   - Defines migration scope, branch strategy, and acceptance gates.

2. `phase-3.6/custom-element-history-import`
   - Imports the legacy repository into `packages/custom-element/`.
   - Add the local checkout as a temporary remote or subtree source:
     `git remote add custom-element-legacy /home/suns/aWork/custom-element`.
   - Fetch with `--no-tags` to avoid un-namespaced legacy tags such as `0.0.37`
     colliding with CEM repo tags.
   - Prefer a non-squashed subtree/read-tree import so file history remains inspectable
     under `packages/custom-element/`.
   - Record legacy release tags in docs or recreate them only as namespaced tags such
     as `custom-element/v0.0.37` after release policy is explicit.

3. `phase-3.6/custom-element-package-scaffold`
   - Adds workspace package metadata, Nx targets, package exports, build output,
     IDE assets, docs, and release-pack shape.
   - Keeps legacy browser entrypoint filenames available while moving implementation
     internals toward the substrate adapter.

4. `phase-3.6/custom-element-adapter`
   - Implements `<custom-element>` as an adapter over `packages/cem-elements`.
   - Shares data-island lifecycle, invalidation, event-to-data wiring, render plans,
     and light-DOM patching with the substrate.
   - Does not keep a separate legacy parser/render engine.

5. `phase-3.6/custom-element-consumers`
   - Rewires root dependency/web-types and `packages/cem-theme` references that load
     `node_modules/@epa-wg/custom-element/{custom-element.js,http-request.js}`.
   - Keeps browser-served paths stable where possible, otherwise documents the new
     import path.

6. `phase-3.6/custom-element-release`
   - Adds changelog, migration guide, bridge-window support matrix, package contents
     checks, and rollback notes.
   - Verifies package-local fixtures plus legacy parity, material parity, Phase 3.5
     Edge/SSR fixtures, `cem-elements:verify`, and affected `cem-theme` workflows.

## Acceptance Gate For This Scope Item

This scope item is complete when:

- Phase 3.6 has an explicit branch/import strategy;
- local history source and consumed package baseline are separated;
- generic legacy tags are not imported into the CEM repo without namespacing;
- first-pass in-scope and out-of-scope migration boundaries are documented;
- the next todo item can begin with package baseline capture rather than strategy
  discussion.
