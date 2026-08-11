# Figma Component Library Review Fixture

This fixture proves the repeatable review path from the executable CEM component
inventory to the native `CEM UI Kit`. It is a review contract, not evidence that
planned canvas assets already exist.

## Representative classification cases

| Inventory entry | Required representation | Review composition |
| --- | --- | --- |
| `cem-action` | `component-set` | State variants and public properties on one reusable action set |
| `cem-icon` | `component` | One reusable icon owner using a bounded instance-swap property, never one variant per icon name |
| `cem-tree-item` | `payload` | Nested only inside the `cem-tree` visual owner, with no standalone interaction asset |
| `cem-stack` | `structural` | Auto Layout composition owner using the declared gap property |

## Review procedure

1. Run `yarn nx run @epa-wg/cem-components:verify-figma-inventory` and use
   `examples/figma/component-library.json` as the only component checklist.
2. Locate each entry through its exact `03 Components / Category / cem-*`
   locator. A `planned` entry may have no node revision; a `reviewed` entry must
   record the reviewed Figma node URL or immutable review revision.
3. Match component properties to the inventory. Do not expose browser-only
   mechanics or an Angular-specific feature absent from the CEM public contract.
4. Match state variants only to the inventory's executable `states`. Runtime
   component/state tests remain authoritative; the Figma canvas cannot promote
   a state.
5. Bind every supported value through `CEM Tokens`; do not substitute raw color,
   spacing, size, stroke, shape, layering, typography, or motion values.
6. Exercise the same asset in `Light`, `Dark`, `Contrast Light`, `Contrast Dark`,
   and `Native` modes without duplicating theme variants.
7. Confirm payload entries remain nested, structural entries retain composition
   ownership, and patterns use library instances without detached replacements.

## Deliberate rejection cases

The review fails if an inventory primitive is missing or duplicated, a property
or state is not traceable to public CEM evidence, a locator is absent, a planned
entry claims a review revision, a reviewed entry lacks one, a payload becomes an
independent interaction component, a raw visual value replaces a CEM variable,
or a theme mode is modeled as a component variant.
