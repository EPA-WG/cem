# CEM UI Kit Plan

**Status:** Accepted Phase 5 contract. The native token collection, offline
token gates, and executable 48-primitive component inventory exist;
foundations, canvas component sets, patterns, and reviewed UI Kit evidence
remain pending.

The CEM UI Kit is the native Figma design library for CEM. It mirrors the
generated token and public semantic-component surface without making Figma a
source of truth. Repository artifacts define the contract and offline release
gate. Canvas changes remain an explicitly reviewed design-library operation;
local builds and CI do not write to Figma.

## Canonical inputs and ownership

| Concern | Canonical source | Figma projection |
| --- | --- | --- |
| Visual values and modes | CEM token markdown and generated `figma/cem-*.tokens.json` | One native `CEM Tokens` collection with `Light`, `Dark`, `Contrast Light`, `Contrast Dark`, and `Native` modes |
| Public component identity | `CEM_COMPONENT_PRIMITIVES` and `docs/component-mvp.md` | One classified inventory entry per public `cem-*` primitive |
| States and coexistence | Executable component state matrix and focused component contracts | Independent variant or boolean properties only where the public component owns the state |
| Author attributes and content | Public component reference and component contracts | Variant, text, boolean, instance-swap, or slot properties selected by semantic meaning |
| Workflow composition | Auth, profile, assets, discussion, and settings fixtures | Reusable patterns and site-demo frames composed from library instances |

Generated Figma mode files are imported with Figma's native DTCG **Import
mode** action. No token plugin is required. The five theme modes are variable
modes, not duplicated component variants. Export or write-back from Figma never
updates token markdown.

## Component representation contract

Every public primitive must be classified before the component canvas expands:

- `component-set`: a visual owner with multiple independent semantic variants;
- `component`: a visual owner whose variation is expressed by variables,
  properties, or nested instances without a variant matrix;
- `payload`: an inert authored element such as an option, step, or tree item,
  represented only inside the visual owner that consumes it; or
- `structural`: a layout/composition owner whose primary contract is Auto
  Layout, slots, or nested instances.

Figma properties mirror public CEM meaning rather than browser mechanics.
`State`, `Intent`, `Size`, `Orientation`, and other variant dimensions stay
independent; compound labels such as `Primary Large Disabled` are forbidden.
Theme mode is never a component property. Hover, focus-visible, active,
disabled, selected, checked, expanded, loading, invalid, current, and other
states are included only for components that own them in executable evidence.
Text properties expose authored labels, boolean properties expose optional
layers or binary public facts, instance swaps expose bounded replaceable
children, and slots cover intentionally free-form authored content.

Each library asset carries its `cem-*` tag, public documentation link, supported
properties, accessibility/state notes, and token families in its description.
Unsupported Angular-specific extensions recorded by partial parity rows are not
invented as Figma variants.

## Pages

| Page | Purpose |
| --- | --- |
| `00 Cover` | Library title, version, source links, status, and release notes. |
| `01 Tokens` | `CEM Tokens` variable collection, token demos, generator category demos, and validation notes. |
| `02 Foundations` | Color, type, spacing, shape, stroke, layering, and motion examples. |
| `03 Components` | Component sets and variants mapped to CEM tokens. |
| `04 Patterns` | Composite layouts using component sets: forms, lists, profile, assets, and messages. |
| `05 Site Demo` | End-to-end screen examples using library components. |
| `99 QA` | Smoke fixtures, contrast checks, mode checks, and visual parity references. |

## Token Mapping

| UI Surface | Figma Token Binding |
| --- | --- |
| Surface fill | `cem/palette/comfort`, `cem/palette/calm`, `cem/zebra/color/*` |
| Text fill | `cem/palette/*/text`, semantic action text variables |
| Action fill | `cem/action/*` and palette action endpoints |
| Spacing | `cem/gap/*`, `cem/inset/*`, `cem/layout/*/gap` |
| Control geometry | `cem/control/*`, `cem/icon/button/*`, `cem/coupling/*` |
| Corner radius | `cem/bend/*` |
| Stroke | `cem/stroke/*` |
| Typography | `cem/typography/*` size variables and derived text styles |
| Motion notes | `cem/duration/*` string variables and generated motion documentation |

## Component Mapping

| Figma family | Source component scope | Required variable bindings |
| --- | --- | --- |
| Actions | action, icon button, menu item | action fill/text, control height, inset, bend |
| Inputs and choices | fields, pickers, select/options, checkbox, radio, switch, slider | surface, text, input/choice state, stroke, bend, gap, control, typography |
| Layout and content | surface, text, icon, stack, grid, divider, list, card, expansion, table/sort, chip, badge, avatar, media, tree | surface, content state, separator, stroke, bend, gap, inset, typography |
| Navigation | app bar, nav, tabs, stepper, paginator | navigation/workflow/action state, gap, inset, stroke, control, typography |
| Feedback | tooltip, dialogs, sheet, toast, progress, spinner, skeleton, alert | surface, feedback/progress state, stroke, bend, gap, inset, layering |

## Delivery sequence

1. Freeze an executable component-library inventory that accounts for every
   public primitive, its representation class, properties, states, token
   families, docs, and manual Figma evidence location.
2. Build `02 Foundations` from the imported variables, including composite text
   styles where variables alone cannot express the approved typography bundle.
3. Build a representative pilot across all semantic families: `cem-action`,
   `cem-text-field`, `cem-card`, `cem-nav`, and `cem-dialog`. Review property
   naming and state density before expanding the library.
4. Complete `03 Components`, including nested payload/composition examples,
   across every inventory entry without detached shapes or raw visual values.
5. Build `04 Patterns` from component instances for auth, profile, assets,
   discussion, and settings. Then compose `05 Site Demo` from those patterns.
6. Populate `99 QA` with the token smoke fixture, pilot state boards, five-mode
   checks, accessibility annotations, and release evidence.

## Verification boundary

The repository gate must remain credential-free and deterministic:

- `yarn nx run @epa-wg/cem-theme:test:figma` verifies generated native DTCG
  mode files, aliases, types, provenance, report health, and token propagation.
- `yarn nx run @epa-wg/cem-components:verify-figma-inventory` verifies the
  executable component inventory and review fixture, depends on the native
  token gate, and emits JSON/Markdown reports under `dist/reports/`.
- The component inventory gate rejects missing/extra public primitives,
  unsupported states, stale documentation, missing token families, or absent
  manual-evidence locators.
- Component and state-matrix Nx gates remain authoritative for runtime behavior;
  a Figma screenshot cannot promote a component state.

Manual library review records the Figma file revision, reviewed pages, five-mode
coverage, detached/raw-value findings, and publication result. REST API reads may
later corroborate that evidence when credentials and plan access exist. REST API
writes, source write-back, and credentialed CI are outside Phase 5 unless a
separate governance decision accepts them.

## QA Expectations

- Each component set has the states required by `docs/component-mvp.md#category-state-coverage`.
- Each variant uses bound variables rather than copied hex or pixel literals when Figma supports the property.
- Each component is tested in `Light`, `Dark`, `Contrast Light`, `Contrast Dark`, and `Native` modes.
- Screens on `05 Site Demo` are composed from `03 Components` instances rather than detached shapes.
- The executable inventory accounts for every public primitive and distinguishes visual owners from inert payloads.
- The five workflow families can be composed without one-off components or unsupported states.
