# Angular Material Parity Inventory

**Status:** Pinned inventory landed; product mappings remain intentionally
unreviewed. The active audit is tracked in
[`docs/todo.md`](../../../docs/todo.md).

## Benchmark

The M4 comparison baseline is the latest stable official Angular Material
release available when this inventory was captured:

| Field | Pinned value |
| --- | --- |
| Product release | Angular Material `22.1.1` |
| Git tag | `v22.1.1` |
| Git commit | `0b67c3c38141049657b1167479accc80e455d2bd` |
| Capture date | 2026-08-10 |
| Public catalog | <https://material.angular.dev/components/categories> |
| Tagged catalog source | <https://github.com/angular/components/blob/v22.1.1/docs/src/app/shared/documentation-items/documentation-items.ts> |
| Catalog entries | 37 |

The tagged `DOCS.components` list is the reproducible source of entry IDs and
display names. Pre-releases are not silently adopted. Updating the benchmark
requires a deliberate tag/commit change plus an exact inventory reconciliation.

## Package boundary

`@epa-wg/cem-elements` is the barebone layer: it supplies `<cem-element>` as the
declarative base plus browser/API primitives such as URL and HTTP resource
access. Its eight legacy Material fixtures prove template/runtime authoring
compatibility for the old component samples; they are not the product catalog
and cannot satisfy an Angular Material parity row.

`@epa-wg/cem-components` is the Material-superset UI layer. It builds on
`cem-elements`, supplies public UI components, and owns Consumer Semantic Theme
styling, state, keyboard, accessibility, forced-colors, and workflow evidence.
Only evidence rooted in this product layer can promote a catalog row to covered.

## Executable inventory

[`angular-material-parity.json`](../tests/angular-material-parity.json) contains
one row for each pinned official entry in official order. New rows start as
`unreviewed` with a null mapping. This is intentional: matching names or an
existing custom-element tag do not establish behavioral parity.

The mapping audit may use these states:

- `unreviewed`: no product mapping decision has been accepted;
- `gap`: required Material-facing behavior has no accepted CEM owner;
- `partial`: an accepted CEM component or cross-cutting behavior covers only a
  documented subset; and
- `covered`: the accepted mapping has exact state, keyboard, accessibility, and
  executable product-layer evidence.

Every reviewed row must record one mapping kind (`component`, `behavior`, or
`gap`), exact owners, required states, keyboard behavior, accessibility
semantics, evidence, and notes. Component owners must be public
`CEM_COMPONENT_PRIMITIVES` tags. Behavior owners use a `behavior:` identity.
Evidence cannot point only at `packages/cem-elements` compatibility fixtures.

Run the invariant with:

```bash
yarn nx run @epa-wg/cem-components:verify-material-parity
```

The gate currently verifies the exact 37-entry pin and reports all 37 rows as
unreviewed, with `autocomplete` recommended as the first mapping audit. It does
not select the first implementation gap; implementation priority is accepted
only after all mappings are reviewed.
