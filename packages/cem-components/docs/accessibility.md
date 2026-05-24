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
| `data-state="loading"` | `aria-busy="true"` for the duration of the loading state. |
| `data-state="empty"` | None (the empty content carries its own semantics). |
| `aria-invalid="true"` | Required when the field validity is failed. Pair with `aria-describedby` pointing at the error message. |
| `aria-expanded` | Required on disclosure / popover / menu triggers; reflects open/closed. |
| `aria-selected` | Required on selectable list/nav rows. |
| `aria-current` | Required on the active nav item; value `"page"` or `"step"` per WHATWG/ARIA. |

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
- A component MUST NOT suppress the focus ring via `outline: none` without
  providing a replacement that meets WCAG 2.2 SC 2.4.11 (Focus Not Obscured) and
  SC 1.4.11 (Non-Text Contrast).

### 5.3 Focus restoration

- Components that open transient surfaces (`cem-dialog-shell`, popovers) MUST
  return focus to the previously focused element on dismissal.
- The previously focused element is captured at open time, not at activation
  time, so a programmatic open from non-focused context returns to the document's
  active element at that moment.

## 6. Keyboard behavior

Each component MUST implement the keyboard pattern documented for its role. The
patterns below are the contract for the Phase 3 primitive set.

| Component | Required keys |
| --- | --- |
| `cem-button` | `Enter`, `Space` activate. `Escape` cancels when inside a transient surface. |
| `cem-text-field` / `cem-select-field` | Native input behavior. `Escape` clears the field's `aria-invalid` state on next valid input. |
| `cem-checkbox` | `Space` toggles. `Enter` MUST NOT toggle (matches native checkbox). |
| `cem-navigation-list` | `ArrowUp`/`ArrowDown` move focus; `Home`/`End` jump to ends; `Enter` activates. Composite tabstop = single. |
| `cem-data-list` | `ArrowUp`/`ArrowDown` move focus among rows; `Enter` activates row's primary action. |
| `cem-message-thread` | `ArrowUp`/`ArrowDown` move between messages; `Home`/`End` for ends. `role="log"` does not normally take focus; the thread does so its messages are reachable. |
| `cem-dialog-shell` | Focus is trapped while open. `Escape` dismisses if non-modal-blocking allows. `Tab`/`Shift+Tab` cycle within. |
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
- `cem-dialog-shell` MUST use `role="dialog"` with `aria-modal="true"` when the
  dialog blocks the rest of the page.

## 8. Live regions

Components that announce updates use ARIA live regions with the following rules:

| Component | Live region | Politeness |
| --- | --- | --- |
| `cem-alert` (info, success) | `role="status"` | polite |
| `cem-alert` (error, destructive) | `role="alert"` | assertive |
| `cem-message-thread` incoming message | `role="log"` (or `role="feed"`) | polite |
| Form field `cem-invalid` event | `aria-live="polite"` on the linked error region | polite |
| Loading state for long async ops (>1s) | `aria-busy` flips; no extra live region | n/a |

Rules:

- Components MUST NOT use `aria-live="assertive"` for routine status updates;
  reserve it for error states.
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
- Phase 3 test harness (item 4 of [`docs/todo.md`](../../../docs/todo.md)) — will
  cover the runtime side (focus order, keyboard, live-region timing) once it
  lands. This document defines the contract that harness will assert.

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
