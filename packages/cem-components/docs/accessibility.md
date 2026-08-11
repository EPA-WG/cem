# CEM Accessibility Contract

**Status:** Phase 3, item 3 of [`docs/todo.md`](../../../docs/todo.md). Pairs with
[`conventions.md`](./conventions.md) and [`light-dom-rendering.md`](./light-dom-rendering.md).

This document is the binding accessibility contract for every component in
`@epa-wg/cem-components`. It maps directly to the Tier A semantic-validation rule
catalog already shipped in `cem_ml` (Phase 2 / Phase 13 of the CLI plan). When a
component violates this contract, the catalog flags it as a hard violation; passing
the catalog is necessary but not sufficient — the component MUST also satisfy the
behaviors below at runtime.

## 1. Scope and authority

- Normative for `@epa-wg/cem-components`.
- The validation-catalog side (AC-V-6, AC-X-3) is owned by `cem_ml`. This document
  is the *runtime* side of the same contract; it specifies what the upgraded
  component MUST do that static catalog rules cannot fully verify.
- Where this document and `cem_ml`'s catalog disagree, the catalog wins as the
  shipping gate. File a follow-up to align this doc.

## 2. Accessible names

Every component listed in [`component-mvp.md`](../../../docs/component-mvp.md) with
interactive or labeled role MUST resolve an accessible name. Resolution order, per
component, follows the WHATWG / ARIA accessible-name computation:

1. `aria-labelledby` (ID-reference list, single space-separated tokens).
2. `aria-label` (string literal).
3. The component's documented label slot: an author child with `slot="label"` or a
   sibling `<label for="...">` for form-associated components.
4. The component's own visible text content where that content acts as the label
   (`cem-button` with author text, `cem-nav-list` item text).
5. The component's `name` / `placeholder` only as a last-resort fallback for form
   fields, NEVER for buttons or navigation.

If none of the above resolves to a non-empty string, the component:

- MUST log `cem.component.accessible_name_missing` at `warning` severity through
  the `cem-error` event (per [`conventions.md §5`](./conventions.md));
- MUST NOT silently invent a name.

Form components additionally MUST link to their associated `<label for="...">` by
ID. The `cem_ml` reference-slot rule (AC-F-5) verifies the link is resolvable; this
contract requires the component to actually reflect the linkage at runtime via
`ElementInternals` or `aria-labelledby`.

## 3. Descriptions

- Long-form descriptions, hints, and validation messages are linked via
  `aria-describedby` referencing one or more author-supplied IDs.
- A component MAY supply a description ID for its own decorative help text;
  decorative help text MUST be marked with `data-cem-decorative` and MUST NOT
  carry the accessible name.
- Validation messages (per [`conventions.md §5`](./conventions.md)) MUST be
  reachable via `aria-describedby` so screen readers announce them when the
  field receives focus.
- `cem-tooltip` appends its stable generated description ID to the exact native
  trigger's existing `aria-describedby` token list. Its separate visible
  `role="tooltip"` copy is non-focusable and transient; disabling or removing
  the component removes only the ID it owns.

## 4. ARIA wiring

Components are responsible for choosing the right ARIA role and keeping ARIA
attributes consistent with reflected state.

### 4.1 Implicit vs. explicit roles

- A component that wraps a native interactive element (`cem-button` → `<button>`,
  `cem-text-field` → `<input>`) MUST NOT set an explicit `role` on the inner
  element. The native role is correct and is the source of truth for assistive
  technologies.
- A component that builds a composite (e.g. `cem-navigation-list`, `cem-message-thread`)
  MUST set the correct ARIA role on the host element (`role="navigation"`,
  `role="log"`).
- A component MUST NOT alter the role between renders. Role is set once at upgrade
  and remains stable for the host's lifetime.

### 4.2 State attribute mirroring

| Reflected state | Required ARIA attribute |
| --- | --- |
| `disabled` | `aria-disabled="true"` on non-form composites (form components rely on the native `disabled`). |
| `data-state="loading"` | `aria-busy="true"` for the duration of the loading state. `cem-card[busy]` and `cem-surface[busy]` place both on their stable named sections and remove both when their respective content or layout workflow settles. |
| `data-state="empty"` | No ARIA attribute. `cem-surface[empty]` reflects this marker on its named section while the visible authored guidance and next action carry their own semantics; the surface does not become a live region. |
| `aria-invalid="true"` | Required when the field validity is failed. Pair with `aria-describedby` pointing at the error message. |
| `aria-expanded` | Required on disclosure / popover / menu triggers; reflects open/closed. `cem-nav[collapsible]` puts it on its native button and keeps the sibling content container's `hidden` state in exact agreement. `cem-expansion` additionally exposes persistent header/panel IDs: its native header mirrors the live host `expanded` attribute and controls the panel while reciprocal `aria-labelledby` names that panel. Applications opening a transient `cem-dialog`, `cem-dialog-shell`, or `cem-sheet` put it on their own opener alongside `aria-controls`; the controlled feedback surface does not describe itself as expanded. |
| `aria-selected` | Required on selectable list options and navigation rows. `cem-list[selectable]` mirrors the native option selectedness exactly; passive lists and static table rows do not expose it. |
| `checked` | A `cem-chip[checkable]` native toggle button MUST expose the current boolean state through `aria-pressed`; passive chips do not expose pressed state. |
| `aria-current` | Required on the active nav item; value `"page"` or `"step"` per WHATWG/ARIA. |
| `role="separator"` / `aria-orientation` | `cem-divider` exposes both for meaningful horizontal or vertical separation. `cem-divider[decorative]` removes both and sets `aria-hidden="true"`; neither form is focusable. |
| `role="progressbar"` / `aria-valuenow` | `cem-progress-spinner` always exposes a labeled read-only progressbar. Determinate mode exposes normalized min/max/now; indeterminate mode omits `aria-valuenow`. Its SVG is hidden and non-focusable. |
| `aria-sort="ascending|descending"` | `cem-sort-header` places sort state only on its generated `role="columnheader"`; none/invalid state omits the attribute. Its direct native button is named `Sort by <label>`. |
| Pagination landmark and boundaries | `cem-paginator` renders a labeled native navigation region, labeled page-size select and actions, an atomic polite range status, and `aria-disabled="true"`/`tabindex="-1"` on unavailable boundary actions. Global `disabled` additionally uses native disabled controls. |
| Native slider values and bounds | `cem-slider` keeps each authored native range input as the slider role/value/bounds/step owner. A single thumb requires one accessible name; range start/end inputs require distinct names. Generated track, ticks, and value labels are `aria-hidden`. |
| Timepicker combobox/listbox | `cem-timepicker` keeps its authored native text input as the labeled value, validation, event, and form owner while adding stable combobox/listbox references. DOM focus remains on the input through `aria-activedescendant`; the optional native toggle shares controls/expanded references and requires its own accessible name. |
| Datepicker combobox/dialog/grid | `cem-datepicker` keeps its authored native text input as the labeled value, validation, event, reset, and form owner while adding stable combobox/dialog references. The optional native toggle shares controls/expanded references and requires its own accessible name. The modal dialog contains a labeled grid with localized column headers, exactly one roving day, `aria-selected` for the draft/committed date, `aria-current="date"` for today, and disabled out-of-range dates. |
| Stepper workflow | `cem-stepper` exposes a labeled region containing an ordered step list. Exact native header buttons use `aria-current="step"`, stable `aria-controls`, visible completion/optional/error copy, `aria-invalid`, and native or focusable ARIA-disabled semantics as appropriate. Stable `role="region"` panels use reciprocal `aria-labelledby`; generic tab roles are not substituted. |
| Tree hierarchy | `cem-tree` exposes one labeled `role="tree"`. Exact native button treeitems expose stable IDs, explicit level/position/set metadata, parent-only `aria-expanded`, optional truthful `aria-selected`, disabled state, and loading `aria-busy`; stable sibling groups are connected through `aria-owns`. |
| Tooltip description and presentation | `cem-tooltip` keeps a stable hidden plain-text description connected to exactly one supported native trigger through `aria-describedby`. Its separate manual Popover copy has `role="tooltip"`, no focusable descendants, and does not replace the trigger's accessible name. |

The catalog enforces presence; runtime enforces *timing* — the attribute MUST
update in the same task that the state changes, not in a deferred callback.

### 4.3 Reference-slot integrity

For every component that emits `id`/`for`/`aria-*` references at runtime:

- The reference target MUST exist in the document before the reference is set.
- When a referenced element is removed from the document, the component MUST
  remove the dangling reference within the same render cycle. Stale references
  trip the catalog's `cem.aria.broken_reference` rule.

## 5. Focus management

### 5.1 Focusability

- Native interactive components inherit focus from their inner element. They MUST
  NOT add `tabindex` to the host element.
- `cem-list[selectable]` keeps its single tab stop, visible focus, pointer
  selection, and keyboard navigation on the rendered native `<select>`. Its
  option payload MUST NOT introduce roving `tabindex`, nested controls, or a
  second focus model.
- `cem-sort-header` keeps its single tab stop, pointer/keyboard activation, and
  interaction paint on the direct native button. The host, `cem-table`, and
  generated column-header wrapper MUST NOT gain `tabindex` or focus paint;
  native `disabled` removes the button from sequential focus navigation.
- `cem-paginator` keeps focus on its native page-size select and action buttons.
  Initially unavailable boundary actions use `tabindex="-1"`; when activation
  moves a focused action onto a boundary, that same surviving button retains
  focus while subsequent sequential navigation skips it. Global `disabled`
  uses native disabled controls and removes every control from the tab order.
- `cem-slider` keeps focus on its one or two authored native range inputs in
  authored order. The host and generated visual/input wrappers add no tab stop;
  global `disabled` removes every thumb from sequential navigation.
- `cem-timepicker` keeps combobox focus on its direct authored native text
  input while the popup uses `aria-activedescendant`. The optional native toggle
  remains a separate authored tab stop, but activation returns focus to the
  input for list navigation. Popup options and wrappers add no tab stop;
  global `disabled` removes both native owners.
- `cem-datepicker` keeps closed-state focus on its direct authored native text
  input or optional native toggle. Opening the native modal dialog moves focus
  to exactly one enabled roving day; native modal behavior makes background
  content inert. Apply, Cancel, Escape, and backdrop dismissal return focus to
  the input. Global `disabled` removes both authored native owners and prevents
  the dialog from opening.
- `cem-stepper` keeps one enabled header in the roving tab order. Horizontal
  Left/Right or vertical Up/Down plus Home/End move focus without selection,
  wrap, and skip native-disabled steps. Focus may expose an `aria-disabled`
  linear/editable destination, but every activation path remains suppressed.
  Host-disabled owners expose no tab stop; authored panel controls participate
  in normal document order only while their selected panel is visible.
- `cem-tree` keeps one visible enabled treeitem in the roving tab order. Focus
  movement skips disabled subtrees and collapsed descendants, never selects or
  activates, and recovers to the nearest visible ancestor when programmatic
  collapse hides the focused descendant. A disabled host exposes no tab stop.
- `cem-tooltip` keeps focus on its one authored native trigger. The host,
  persistent description, and visible tooltip add no tab stop; opening,
  dismissing, pointer travel, and Escape never move focus.
- `cem-nav[collapsible]` keeps focus on its native disclosure button after a
  toggle. Open projected links follow the button in normal tab order; native
  `hidden` removes closed content from sequential focus navigation.
- `cem-surface[empty]` does not receive focus or move focus during state
  transitions. Surviving projected controls retain focus; a workflow that
  removes a focused descendant owns recovery because it knows the valid target.
- `cem-card[busy]` does not receive focus, disable descendants, make its subtree
  inert, or move focus during state transitions. A surviving focused descendant
  retains focus; the workflow owns recovery when final payload replacement
  removes it.
- `cem-surface[busy]` follows the same focus and operability boundary for a
  whole workflow layout. Its section, surviving descendants, placement, and
  focused control remain stable through busy transitions; the workflow owns
  recovery when it replaces the focused node.
- Static `cem-dialog`, `cem-dialog-shell`, and `cem-sheet` owners remain
  structural and do not acquire `tabindex` or move focus. In transient mode,
  the two dialog tags render a native `<dialog>` and let `showModal()` choose an
  authored `autofocus` target or the browser fallback. A transient sheet
  remains a focus-neutral region whose authored controls participate in normal
  document order.
- Composite components decide tabindex per the WAI-ARIA Authoring Practices for
  their composite pattern (e.g. menubar = one tabstop, internal arrow keys). Per
  pattern, the component MUST set `tabindex="0"` on the entrypoint and
  `tabindex="-1"` on the rest, then move focus programmatically.
- A `disabled` component MUST be removed from the tab order. For form-associated
  components, the native `disabled` does this; for composites, the component MUST
  set `tabindex="-1"` and `aria-disabled="true"`.

### 5.2 Focus indication

- Components MUST render a visible focus ring under `:focus-visible`, using
  cem-theme tokens (`--cem-stroke-focus`, `--cem-control-focus-ring`).
- When a transient `cem-dialog` or `cem-dialog-shell` is itself the browser's
  native fallback focus owner, its direct `<dialog>` receives the external D5
  `--cem-stroke-focus` / `--cem-stroke-indicator-offset` ring with zebra focus
  color. Forced colors retain that geometry with `CanvasText` and
  `forced-color-adjust: auto`.
- Static dialog wrappers, feedback hosts, and `cem-sheet` regions MUST NOT gain
  `tabindex`, `:focus-within` paint, or a descendant-wide ring. Eligible
  authored dialog and sheet controls retain their own focus indicators.
- A component MUST NOT suppress the focus ring via `outline: none` without
  providing a replacement that meets WCAG 2.2 SC 2.4.11 (Focus Not Obscured) and
  SC 1.4.11 (Non-Text Contrast).

### 5.3 Focus restoration

- Transient `cem-dialog` and `cem-dialog-shell` delegate normal close
  restoration to the native dialog lifecycle. The component separately
  remembers the active element at `showModal()` time only to recover focus when
  a still-open dialog host disconnects and that element remains connected.
- Focus is captured at open time, not at application-trigger activation time,
  so programmatic opening uses the document's active element at that moment.
- `cem-sheet[transient]` does not move or restore focus. The application owns
  recovery if it removes a focused authored descendant.

## 6. Keyboard behavior

Each component MUST implement the keyboard pattern documented for its role. The
patterns below are the contract for the Phase 3 primitive set.

| Component | Required keys |
| --- | --- |
| `cem-button` | `Enter`, `Space` activate. `Escape` cancels when inside a transient surface. |
| `cem-nav[collapsible]` | Native disclosure-button behavior: `Enter` and `Space` toggle; `Tab` reaches projected links only while open. |
| `cem-expansion` | Native header-button behavior: `Enter` and `Space` toggle the live `expanded` state; collapsed panel content leaves the tab sequence; disabled suppresses user toggling without preventing programmatic state control. |
| `cem-sort-header` | Native button behavior: `Enter` and `Space` each cycle none -> ascending -> descending -> none exactly once; disabled suppresses user activation while programmatic direction remains available. |
| `cem-paginator` | The page-size control retains native select keys. Available first/previous/next/last native buttons use Enter/Space exactly once. Boundary and global-disabled actions suppress pointer, programmatic, Enter, and Space activation without emitting `cem-page`; no arrow-key roving model is added. |
| `cem-slider` | Each native range input retains ArrowLeft/ArrowDown decrement, ArrowRight/ArrowUp increment, PageUp/PageDown larger change, and Home/End bounds. Range mode clamps the changing thumb at its peer and emits no replacement event. |
| `cem-timepicker` | On the native text input, ArrowUp/ArrowDown open and navigate enabled options, Enter commits one canonical value, Escape closes without a value change, and Tab closes without trapping focus. Home/End, horizontal arrows, character editing, clipboard, and undo remain native text-input behavior. The optional toggle retains native Enter/Space activation. |
| `cem-datepicker` | ArrowDown or Alt+ArrowDown on the native text input opens the modal calendar; the optional toggle retains native Enter/Space activation. In the grid, arrows move by day/week, Home/End move to locale week edges, PageUp/PageDown move by month, Shift/Alt+PageUp/PageDown move by year, and Enter/Space drafts one enabled date. Apply commits; Escape, Cancel, or backdrop dismissal closes silently. |
| `cem-stepper` | Horizontal Left/Right or vertical Up/Down moves roving header focus, wraps, and skips native-disabled steps; Home/End reaches the first/last enabled header. Enter/Space follows native button activation and commits only an eligible non-current step. The other-axis arrows remain native. |
| `cem-tree` | Up/Down traverses visible enabled nodes without wrapping; Right opens a closed parent or enters its first enabled child; Left closes an open parent or reaches its nearest enabled ancestor; Home/End reaches boundaries; printable typeahead searches visible labels; native Enter/Space toggles a parent or activates a leaf. |
| `cem-tooltip` | Native trigger keys remain unchanged. Keyboard focus presents the same description as hover; Escape dismisses immediately without moving focus, trapping focus, or synthesizing activation. Blur dismisses unless pointer or declarative `open` still supplies a visibility reason. |
| `cem-text-field` | Native text-input behavior. `Escape` does not mutate authored validation state. |
| `cem-select` | Dropdown arrows/Home/End/Page/typeahead move the preview; Enter/Space/Tab commit and Escape cancels. Sized single listboxes commit movement. Multiple listboxes use modifier-free Space/click toggle, Shift range, and Ctrl/Cmd+A. |
| `cem-checkbox` | `Space` toggles. `Enter` MUST NOT toggle (matches native checkbox). |
| `cem-navigation-list` | `ArrowUp`/`ArrowDown` move focus; `Home`/`End` jump to ends; `Enter` activates. Composite tabstop = single. |
| `cem-data-list` | `ArrowUp`/`ArrowDown` move focus among rows; `Enter` activates row's primary action. |
| `cem-message-thread` | `ArrowUp`/`ArrowDown` move between messages; `Home`/`End` for ends. `role="log"` does not normally take focus; the thread does so its messages are reachable. |
| `cem-dialog[transient]`, `cem-dialog-shell[transient]` | Native modal Tab/Shift+Tab containment. Escape dispatches the cancellable native `cancel` request; successful native dismissal closes, restores focus, removes host `expanded`, and then emits `cem-dismiss`. Prevented cancel stays open. Static mode adds no component keyboard handling. |
| `cem-sheet[transient]` | No component-owned keys. Escape is not intercepted, and authored controls keep their native behavior and document tab order. |
| `cem-app-shell` | Skip-link target MUST be focusable (`tabindex="-1"`). |
| `cem-top-bar` | Native focus order; primary actions follow `cem-button` rules. |
| `cem-form` | `Enter` in any text field submits if the form has exactly one submit button; otherwise activates the default submit per WHATWG. |
| `cem-alert` | If interactive (dismissible), Tab reaches dismiss control; `Escape` dismisses when the alert has been acknowledged. |
| `cem-badge` | Non-interactive; no keyboard handling. |
| `cem-card` | Non-interactive by default. When the card is a link, native anchor keyboard behavior applies. |

For composite focus management, the component MUST update `tabindex` reflectively
so the catalog can verify there is exactly one entrypoint per composite.

## 7. Roles and landmarks

- `cem-app-shell` MUST render WHATWG landmarks: `<header>` (or `role="banner"`),
  `<nav>`, `<main>`, `<footer>` as appropriate. Each landmark MUST be unique per
  document unless labeled distinctly via `aria-label` / `aria-labelledby`.
- `cem-navigation-list` MUST use `role="navigation"` on its host and a labeled
  navigation region. The label MUST be either a slotted heading or `aria-label`.
- `cem-message-thread` MUST use `role="log"` (or `role="feed"` when the message
  count exceeds the documented threshold) with `aria-live` per §8.
- `cem-alert` MUST use `role="alert"` (assertive) for error/destructive intent or
  `role="status"` (polite) for info/success intent.
- Static `cem-dialog` and `cem-dialog-shell` retain their labeled
  `div[role="dialog"][aria-modal="true"]` compatibility owner. With
  `transient`, each renders a labeled native `<dialog>` and MUST NOT add
  redundant `role`, `aria-modal`, or `tabindex`; the browser owns modal state.
- `cem-sheet` remains a labeled `<aside role="region">` in both modes. Its
  transient visibility uses native `hidden` and never claims dialog semantics.
- `cem-sort-header` renders one `role="columnheader"` with a direct native
  button. Only ascending/descending state exposes `aria-sort`; user activation
  clears active peers in the nearest table, while applications must keep
  authored/programmatic table state single-valued.
- `cem-paginator` renders one labeled native `<nav>` for each pagination region.
  Identical top/bottom instances may share the same label; otherwise multiple
  navigation landmarks require distinct names. Its range/actions wrapper has
  no role, and each control retains its native semantics.
- `cem-slider` renders no competing slider role. Its structural track, active
  range, tick marks, and optional value labels stay inside one `aria-hidden`
  visual branch while the native input branch owns accessibility and forms.
- `cem-timepicker` renders no competing value or form role. Its exact authored
  text input owns the combobox, one generated Popover owns the listbox, and
  generated options are not focusable. The host and layout wrapper remain
  semantic-neutral.
- `cem-datepicker` renders no competing value or form role. Its exact authored
  text input owns the combobox and validation, one native modal dialog owns the
  calendar surface, and its labeled grid owns generated roving day gridcells.
  The host and layout wrapper remain semantic-neutral.
- `cem-stepper` renders no tablist/tab/tabpanel substitute. Its labeled section
  and ordered list provide workflow structure; exact native buttons own focus
  and activation, `aria-current="step"` names current position, and reciprocal
  labeled regions own persistent panels. Inert `cem-step` payloads add no live
  role or tab stop.
- `cem-tree` renders one labeled tree and no navigation/listbox/menu substitute.
  Exact native buttons own treeitem focus and activation; sibling role=group
  containers own recursive structure through stable `aria-owns` references.
  Inert `cem-tree-item` payloads add no live role, tab stop, or loading/request
  behavior.
- `cem-tooltip` renders no competing trigger role. Its stable hidden description
  supplements the native trigger through `aria-describedby`; only the separate
  non-interactive visible Popover carries `role="tooltip"`.

## 8. Live regions

Components that announce updates use ARIA live regions with the following rules:

| Component | Live region | Politeness |
| --- | --- | --- |
| `cem-alert` (info, success) | `role="status"` | polite |
| `cem-alert` (error, destructive) | `role="alert"` | assertive |
| `cem-message-thread` incoming message | `role="log"` (or `role="feed"`) | polite |
| Form field `cem-invalid` event | `aria-live="polite"` on the linked error region | polite |
| Loading state for long async ops (>1s) | `aria-busy` flips; no extra live region | n/a |
| `cem-paginator` current range | atomic `role="status"` on `start – end of length` | polite |

Rules:

- Components MUST NOT use `aria-live="assertive"` for routine status updates;
  reserve it for error states.
- A busy card or surface MUST NOT become `role="status"`, `role="alert"`, or an
  `aria-live` region. Its visible authored loading text and `aria-busy` property
  expose the waiting state. A workflow that requires a separate announcement
  may author a dedicated non-interactive status node outside the busy subtree.
- `cem-progress-spinner` is not a live region and does not set `aria-busy` on
  itself or another region. Applications put `aria-busy="true"` on the affected
  region, keep the spinner's label meaningful, and remove both when work settles.
- `cem-sort-header` does not announce before data changes. Applications consume
  `cem-sort`, reorder their data, then update a localized polite status region;
  the component does not create or mutate that region.
- `cem-paginator` announces only its requested range. Applications consume
  `cem-page` to load and render data and own any separate result-count or
  loading announcement; the paginator does not claim that data has arrived.
- A live region's text content MUST NOT include the accessible name of the
  triggering component (avoid duplicate announcements).
- Live region updates MUST be debounced so a burst of updates within 250 ms
  collapses to a single announcement.

## 9. SVG and embedded content

For components that embed SVG (icons, illustrations, charts):

- Decorative SVGs MUST carry `aria-hidden="true"` and have empty/absent
  `<title>` / `<desc>`.
- Informational SVGs MUST carry `role="img"` and a `<title>` with the accessible
  name as the first child, optionally followed by `<desc>`.
- Focusable SVGs are forbidden in the primitive set; charts that require focus
  promote a wrapping `cem-` component to own the focus and ARIA semantics.
- `cem-progress-spinner` follows the decorative-SVG branch: its wrapping
  progressbar owns the accessible name and value, while the two-circle SVG is
  `aria-hidden="true"` and `focusable="false"`.

The catalog's SVG-in-HTML accessibility rules (Phase 2) enforce these statically
on rendered output.

## 10. Unsafe content

Components MUST refuse to render content that fails the catalog's unsafe-content
rules:

- Inline `on*` event handlers from author input.
- `javascript:` URLs in `href`, `src`, `action`, `formaction`, etc.
- `srcdoc`, external entities, and other policy-gated resource hooks.

When the catalog flags such content during `cem_ml` validation, the component
MUST surface the diagnostic and refuse to render the offending node, not silently
strip it.

## 11. Verification

- `nx run cem_ml_cli:validate-fixtures` — catches static a11y rule violations on
  the canonical and HTML parity fixtures.
- `nx run cem_ml_cli:e2e` — catches a11y rule violations on rendered round-trip
  output.
- Phase 3 test harness (item 4 of [`docs/todo.md`](../../../docs/todo.md)) — covers
  the runtime side through browser-backed assertions for light-DOM output,
  component events, accessible names, ARIA/reference integrity, focus indicators,
  deterministic visual snapshots, and a Chromium screenshot smoke path.

## 12. AC and design references

- [`docs/cem-element-design.md`](../../../docs/cem-element-design.md) — `<cem-element>`
  substrate that hosts these components; the production-ready criteria require this
  contract to pass on the material parity fixtures.
- AC-V-6 (semantic validation surface, accessible-name detection).
- AC-F-5 (CEM reference slots for `id`/`for`/`aria-*`).
- AC-X-3 (unsafe-content semantic rules).
- AC-I-6 (WHATWG HTML DOM compliance).
- Tier A semantic-rule catalog (Phase 13 / Phase 2 close-out).
- WAI-ARIA Authoring Practices — patterns referenced in §6 by composite name.
- WCAG 2.2 SC 1.4.11 (Non-Text Contrast), SC 2.4.11 (Focus Not Obscured),
  SC 2.5.8 (Target Size Minimum). Token-driven sizing in cem-theme is the
  enforcement mechanism for size minima.
