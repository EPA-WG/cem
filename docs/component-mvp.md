# CEM Component MVP

This document defines the accepted Phase 4 component MVP list, state matrix, and first app workflow surfaces. It is the
contract for `@epa-wg/cem-components`, the CEM core schema state vocabulary, and the future Figma UI Kit mapping.

## Scope

- Components are CEM semantic components, not direct Material clones. Angular Material remains a coverage and ergonomics
  benchmark only.
- The MVP favors components required by auth, profile, asset, discussion, and settings workflows before expanding into
  specialized controls.
- All components render in the light DOM through the `<cem-element>` substrate. No Phase 4 component may depend on the
  legacy `<custom-element>` authoring surface.

## Component List

| Category | Component ID | Element name | Primary use | Required token families |
| --- | --- | --- | --- | --- |
| Action | `action` | `cem-action` | Text action, submit, and command buttons | action, control, palette, bend, typography |
| Action | `icon-button` | `cem-icon-button` | Compact icon-only command with required accessible name | action, control, palette, stroke, bend |
| Action | `menu-item` | `cem-menu-item` | Command or navigation row inside menus and action lists | action, palette, gap, inset, typography |
| Input | `field` | `cem-field` | Generic labeled field wrapper for simple form controls | palette, stroke, bend, gap, typography |
| Input | `text-field` | `cem-text-field` | Single-line text entry with label, help, and validation | palette, stroke, bend, gap, typography |
| Input | `textarea` | `cem-textarea` | Multi-line text entry with label, help, and validation | palette, stroke, bend, gap, typography |
| Input | `autocomplete` | `cem-autocomplete` | Form-associated editable combobox with declarative suggestions | palette, select, stroke, bend, layering, control, typography |
| Input | `select` | `cem-select` | Form-associated rich single/multiple choice | palette, select, stroke, bend, layering, control, typography |
| Input | `option` | `cem-option` | Canonical rich option payload for select and autocomplete | palette, typography |
| Input | `option-group` | `cem-option-group` | Labeled option grouping payload for select and autocomplete | palette, typography |
| Input | `checkbox` | `cem-checkbox` | Binary consent, settings, and filters | palette, stroke, control, bend, typography |
| Input | `radio` | `cem-radio` | Mutually exclusive choice inside a radio group | palette, stroke, control, typography |
| Input | `switch` | `cem-switch` | Immediate boolean setting toggle | palette, stroke, action, control, bend |
| Input | `slider` | `cem-slider` | Single-value or range input with native thumb/form ownership | slider, coupling, stroke, bend, gap, typography |
| Layout | `surface` | `cem-surface` | Section surface for grouped content and workflow regions | palette, stroke, bend, gap, inset |
| Content | `text` | `cem-text` | Token-scoped inline text and typography variant wrapper | typography, palette |
| Content | `icon` | `cem-icon` | Decorative or labeled icon text primitive | action, palette, stroke, typography |
| Layout | `stack` | `cem-stack` | Single-axis layout container | gap, responsive |
| Layout | `grid` | `cem-grid` | Responsive grid layout container | gap, responsive |
| Layout | `divider` | `cem-divider` | Semantic or decorative sibling-separation track | separator, stroke, gap, inset, coupling |
| Content | `list` | `cem-list` | Ordered or unordered collection, including empty state | palette, content, stroke, gap, typography |
| Content | `card` | `cem-card` | Summary container for profile, asset, and message content | palette, stroke, bend, gap, inset |
| Content | `expansion` | `cem-expansion` | Independent general-purpose disclosure panel | action, palette, stroke, bend, gap, inset, coupling, control, typography |
| Content | `table` | `cem-table` | Structured data comparison and asset grids | palette, stroke, gap, typography |
| Content | `sort-header` | `cem-sort-header` | Sortable column action with application-owned row ordering | action, control, stroke, bend, gap, coupling, typography |
| Content | `chip` | `cem-chip` | Compact filter, token, or removable label | palette, content, action, stroke, bend, inset, typography |
| Content | `badge` | `cem-badge` | Status, count, priority, and severity labels | palette, bend, inset, typography |
| Content | `avatar` | `cem-avatar` | Person or organization visual identity | palette, bend, typography |
| Content | `media-preview` | `cem-media-preview` | Asset thumbnail, file, or object preview | palette, stroke, bend, gap |
| Navigation | `app-bar` | `cem-app-bar` | Product title, global actions, and current context | palette, stroke, gap, inset, typography |
| Navigation | `nav` | `cem-nav` | Labeled navigation region and item list | palette, navigation, gap, inset, typography |
| Navigation | `tabs` | `cem-tabs` | Local view switching | palette, navigation, stroke, gap, typography |
| Navigation | `paginator` | `cem-paginator` | Paged-content navigation with application-owned data | action, palette, select, control, stroke, bend, gap, inset, typography |
| Feedback | `dialog` | `cem-dialog` | Modal decision or focused task | palette, stroke, bend, gap, inset |
| Feedback | `dialog-shell` | `cem-dialog-shell` | Labeled dialog shell for focused light-DOM task content | palette, stroke, bend, gap, inset |
| Feedback | `sheet` | `cem-sheet` | Non-modal or edge-attached task surface | palette, stroke, bend, gap, inset |
| Feedback | `toast` | `cem-toast` | Transient status message | palette, action, stroke, gap, typography |
| Feedback | `progress` | `cem-progress` | Determinate and indeterminate progress | palette, action, control, typography |
| Feedback | `progress-spinner` | `cem-progress-spinner` | Circular determinate and indeterminate progress | progress, timing |
| Feedback | `skeleton` | `cem-skeleton` | Loading placeholder preserving layout | palette, control, bend |
| Feedback | `alert` | `cem-alert` | Inline info, success, warning, and error feedback | palette, action, stroke, gap, typography |

## Deferred From MVP

The roadmap still includes split actions, date/time affordances, side-nav variants, breadcrumbs, and richer
menu/dropdown families. They are deferred until the MVP workflows prove the shared component states,
accessibility behavior, and token usage.

## State Matrix

States are exposed as CEM semantic state names and mirrored to host attributes or ARIA according to the component docs.
`focus` in planning conversations maps to the canonical state name `focus-visible`.

| State | Applies to | Required behavior |
| --- | --- | --- |
| `default` | All components | Uses mode-aware palette, type, shape, spacing, and stroke variables. |
| `hover` | Interactive actions, inputs, nav, tabs, rows, chips | Uses action hover treatment without changing layout. |
| `focus-visible` | Keyboard-focusable actions, inputs, nav, tabs, dialogs, sheets | Shows a visible focus ring using CEM focus tokens. |
| `active` | Actions, menu items, tabs, nav items, chips | Uses active action treatment and preserves text contrast. |
| `disabled` | Actions, inputs, nav items, tabs, menu items, chips | Removes activation and tab stop where appropriate while keeping readable labels. |
| `loading` | Actions, inputs, lists, tables, cards, dialogs, sheets, progress, skeletons | Preserves dimensions and reflects busy status. |
| `selected` | Nav items, tabs, menu items, table/list rows, chips | Distinguishes current selection from hover and focus. |
| `expanded` | Nav groups, select, menu item submenus, sheets, dialogs | Mirrors disclosure state with `aria-expanded` where applicable. |
| `invalid` | Text fields, textareas, selects, checkbox/radio groups, switches, forms | Reflects validation failure with error relationship and error tokens. |
| `required` | Text fields, textareas, selects, checkbox/radio groups | Exposes required semantics without relying on a visual mark alone. |
| `readonly` | Text fields, textareas, select-like read views | Allows focus and submission while preventing edits. |
| `checked` | Checkbox, radio, switch, filter chips | Mirrors native checked semantics and selected visual treatment. |
| `indeterminate` | Checkbox, aggregate selection controls, and progress indicators | Communicates mixed selection or unknown progress through native or ARIA state. |
| `empty` | Lists, tables, cards, media preview, discussion surfaces | Provides visible empty-state content and a next action path. |

## Category State Coverage

| Category | Required MVP states |
| --- | --- |
| Action | `default`, `hover`, `focus-visible`, `active`, `disabled`, `loading` |
| Input | `default`, `hover`, `focus-visible`, `disabled`, `loading`, `expanded`, `invalid`, `required`, `readonly`, `checked`, `indeterminate` |
| Navigation | `default`, `hover`, `focus-visible`, `active`, `disabled`, `selected`, `expanded` |
| Layout | `default`, `loading`, `empty` |
| Content | `default`, `hover`, `focus-visible`, `selected`, `loading`, `empty`, `checked` |
| Feedback | `default`, `focus-visible`, `loading`, `expanded`, `invalid`, `indeterminate` |

### Executable State Coverage

[`../packages/cem-components/tests/state-matrix-coverage.json`](../packages/cem-components/tests/state-matrix-coverage.json)
is the machine-readable audit of this table. Each category/state requirement names the affected or evidenced
components, the required interaction or transition, and one of three evidence states:

- `covered`: an exact browser test and assertion own the requirement;
- `static-only`: fixture markup authors the state but no browser assertion proves its behavior;
- `gap`: no executable owner exists yet.

`yarn nx run @epa-wg/cem-components:verify-state-matrix` rejects missing requirements, unknown components, stale
browser tests/assertions, and stale static fixture markers. It emits deterministic JSON and Markdown reports under
`packages/cem-components/dist/reports/`. Classified gaps do not fail the audit; the inventory's priority list selects
the next gap that must receive a fixture before its status can change to `covered`.

## First App Workflow Surfaces

The MVP is complete only when these workflows can be built without one-off UI controls:

1. Auth forms: login, registration, password reset, and required/invalid/loading form states.
2. Profile editor: avatar, editable fields, preference toggles, validation feedback, and save/cancel actions.
3. Asset browser: filter controls, table/list results, media preview, empty/loading states, badges, and row actions.
4. Discussion thread: message list, composer textarea, status badges, loading/empty feedback, and toast/error handling.
5. Settings page: grouped cards, switches, checkbox/radio groups, navigation, and confirmation dialog/sheet flows.

## First Validation Flow

Use tests and fixtures outside `examples/` as the executable coverage. Example-shaped cases may mirror
`examples/cem-ml/` and `examples/semantic/`, but test data should live with the package or crate that owns the behavior.

1. Render each workflow-shaped fixture through the DOM/XSLT or CEM-ML pipeline.
2. Confirm every component maps to a component row above.
3. Confirm every visible component state maps to a state row above.
4. Confirm every visual value comes from CEM token CSS or native Figma variables.
5. Confirm accessible names, ARIA state mirrors, keyboard behavior, and reference integrity through package tests.
