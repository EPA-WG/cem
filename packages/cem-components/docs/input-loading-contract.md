# Input Loading Contract

**Status:** Implemented Phase 4 contract. This contract is promoted by
[`docs/todo.md`](../../../docs/todo.md) and covered by the focused browser test
in [`states.browser.spec.ts`](../src/lib/states.browser.spec.ts).

## Decision

`busy` is a presence-only state-projection attribute on `cem-field`,
`cem-text-field`, `cem-textarea`, `cem-select`, `cem-checkbox`, `cem-radio`, and
`cem-switch`. It is the truthful input-loading source supplied by the
application or workflow that owns the pending validation, persistence, option,
or preference update. The component does not infer pending work from a busy
ancestor, network activity, value changes, validation, or elapsed time.

While `busy` is present, the same interactive input, textarea, combobox, or listbox gains
exactly `data-state="loading"` and `aria-busy="true"`. The label, help text,
value, checked state, native node, dimensions, focus, tab order, and ordinary
editability remain unchanged. `busy` is not `disabled`, `readonly`, or `inert`;
authors add those separate states only when the owning workflow requires them.

The public API uses the package's existing `busy` projection vocabulary rather
than `loading`, whose platform meaning already applies to resource elements and
whose existing `cem-action[loading]` spelling predates the presence-only
content/layout contracts. `busy="false"` is therefore present and true; remove
the attribute when the operation settles.

## Alternatives considered

| Shape | Decision | Reason |
| --- | --- | --- |
| Explicit host `busy` projected to the interactive control | Accepted | Supplies a deterministic state source without making the primitive own asynchronous work. |
| Inherit from `cem-card[busy]` or `cem-surface[busy]` | Rejected | A pending region does not imply that each descendant value is pending, and inherited state would obscure the narrowest meaningful owner. |
| Infer from input/change, validation, resource, or timer activity | Rejected | The primitive cannot know which operation owns the value, when it settles, or whether editing should remain available. |
| Disable, make readonly, or make inert while busy | Rejected as an implicit effect | `aria-busy` communicates an update; it does not define interaction suppression. The workflow must author any independent availability state. |
| Generate status text, a spinner, or a live region | Rejected for v1 | Useful waiting language, announcement policy, and placement are workflow concerns; generated content could change geometry or duplicate feedback. |
| Pending color as the only visual cue | Rejected | CEM pending state must remain distinguishable without hue. D5 supplies a thicker pending anchor while D0 supplies its color. |

## State and rendering algorithm

1. Treat the input host's `busy` attribute as the only v1 loading source.
2. When `busy` is absent, omit `data-state` and `aria-busy` from the interactive
   control and preserve the existing primitive output.
3. When `busy` is present with any value, patch the same interactive control to
   `data-state="loading"` and `aria-busy="true"`.
4. Do not create a `busy` or `loading` slice, request, timer, `AbortSignal`,
   lifecycle event, status node, live region, overlay, or generated payload.
5. The workflow sets `busy` before or with the pending operation and removes it
   after the current value or choice reaches the workflow's selected outcome.
6. Busy-on and busy-off rendering must retain control node identity, value,
   checked state, selection, focus, label relationships, and dimensions.

## Visual and state composition

The loading treatment reuses the input indicator's anchor/state stripe. D0
changes its color to `--cem-input-indicator-anchor-pending-color`; D5 changes
its width from `--cem-stroke-boundary` to `--cem-stroke-pending`. A thicker
box-shadow stripe does not participate in layout, so underline and outline
appearances preserve control dimensions. No component-local CSS value or
exception is required.

Anchor precedence remains
`disabled > invalid-hover > invalid > pending > readonly > hover > default`.
Disabled or invalid state therefore owns the complete anchor treatment even
while `aria-busy` continues to expose the simultaneous pending state. Focus and
selection remain independent stripes and can coexist with pending.

In forced colors, normal shadows are removed. An enabled, valid busy control
receives a full `CanvasText` outline using `--cem-stroke-pending`; keyboard
focus replaces it with the stronger focus outline. Disabled and invalid
fallbacks retain their higher-priority native or component treatments.

## Accessibility and interaction

- `aria-busy="true"` is applied to the native form control whose value or choice
  is pending; it is removed together with `data-state="loading"` on settlement.
- Accessible labels, descriptions, error relationships, required state, role,
  and native form ownership remain unchanged.
- Busy state neither moves focus nor removes a tab stop. Text controls remain
  editable and binary/select controls retain native activation unless a
  separate authored state says otherwise.
- The component adds no live announcement. A workflow that needs spoken or
  visible progress authors dedicated status content outside a busy subtree.
- Pending is not communicated by hue alone: the generated pending stroke
  changes anchor thickness, while `aria-busy` supplies the programmatic state.

## Executable acceptance

The implementation is complete only when a focused browser test proves:

- all seven hosts project presence-only `busy`, including `busy="false"`, to
  exact native loading markers;
- an ordinary control gains and loses those markers without replacing its
  native node or changing label, value, checked state, dimensions, focus,
  tab order, or editability;
- pending anchor color and width resolve from generated CEM tokens in both
  underline and outline appearances;
- invalid and disabled anchor precedence, plus independent focus and selection
  stripes, remain intact during busy state;
- forced colors retain a non-color pending outline and focus remains stronger;
- no busy/loading slice, lifecycle event, synthetic mutation event, resource
  request, generated status, live region, or inert subtree is introduced; and
- only `input:loading` moves from gap to browser-covered in the state matrix.
