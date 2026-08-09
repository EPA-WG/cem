# Layout Empty Contract

**Status:** Accepted Phase 4 design; implementation pending. This contract is
promoted by [`docs/todo.md`](../../../docs/todo.md) and governs the next
`layout:empty` implementation slice.

## Decision

`cem-surface[empty]` owns the Phase 4 layout empty state. A surface represents a
named workflow region, so it can give an empty result the context, visible
guidance, and recovery path required by the state matrix. The presence-only
`empty` attribute explicitly says that the region has reached a settled state
with no workflow content to show.

The component does not infer emptiness by counting child nodes. In empty mode,
authors project the complete visible empty-state experience through the existing
default payload: a heading or other clear message, concise guidance, and a
native or CEM action/link that provides the next path. The rendered `<section>`
keeps its normal accessible name and gains `data-state="empty"`; it does not
become a live region or move focus.

`cem-stack` and `cem-grid` remain formatting-only containers. Their empty output
is valid during composition and must not create landmarks, announcements, empty
messages, state inference, or recovery behavior.

## Alternatives considered

| Shape | Decision | Reason |
| --- | --- | --- |
| Explicit `cem-surface[empty]` with authored default payload | Accepted | The surface already represents a named workflow region, preserves progressive fallback, and can contain contextual guidance plus a real next action without expanding the primitive catalog. |
| Empty `cem-stack` owner | Rejected | A stack is a generic single-axis formatter used throughout component internals and workflows. Zero children is not enough evidence of a user-facing empty state. |
| Empty `cem-grid` owner | Rejected | A grid controls visual placement, not collection or workflow semantics. Empty grids are often transient or intentional and have no accessible name contract. |
| Infer state from child count or CSS `:empty` | Rejected | Whitespace, comments, inert templates, conditionally rendered nodes, hidden content, and unresolved resources make structural emptiness different from settled semantic emptiness. |
| Add `cem-empty-state` | Deferred | A new public primitive is unnecessary for the audited layout state and would expand the accepted 32-component MVP before a reusable cross-category contract exists. |
| Add named empty/content slots | Rejected for v1 | Two alternate authored payloads can both remain visible before upgrade and add a second content-switching contract. A single default payload keeps the fallback meaningful. |

## Author API

```html
<cem-surface label="Asset results" empty>
  <h2>No assets yet</h2>
  <p>Upload an asset to begin building this collection.</p>
  <a href="/assets/new">Upload an asset</a>
</cem-surface>
```

| Attribute or payload | Contract |
| --- | --- |
| `label` | Required non-empty accessible name describing the workflow region. A visible heading SHOULD use matching language so the visual and programmatic context agree. |
| `empty` | Presence marks a settled empty workflow state. Absence preserves the existing ordinary surface. It is author/data-source owned, not inferred by `cem-surface`. |
| Default payload while empty | MUST contain visible human-readable empty guidance and MUST expose a meaningful next path as a native link/button or an appropriate CEM action. It is rendered unchanged rather than synthesized by the primitive. |
| `tone` | Retains its existing visual contract and does not determine empty semantics. |

`empty` uses WHATWG boolean presence semantics. `empty="false"` still means
present/true and is an invalid authoring attempt to express a non-empty state.
Authors remove the attribute when the region is no longer empty.

## State and rendering algorithm

1. Treat the `empty` attribute as the only v1 source of layout-empty state. Do
   not inspect payload node count, text, visibility, descendant collection
   length, loading resources, or CSS layout results.
2. When `empty` is absent, render the existing named surface `<section>` and
   default payload unchanged. Do not add `data-state`, wrapper content, fallback
   text, or controls.
3. When `empty` is present, render the same stable `<section>`, classes, tone,
   accessible label, and default payload, and add exactly
   `data-state="empty"` to that section. Do not hide or replace authored payload.
4. The workflow or data owner changes the `empty` attribute and corresponding
   payload together. `cem-surface` does not create an `empty` slice, bind a
   `slice-event`, dispatch a state event, fetch data, or decide that a request
   returned zero results.
5. Host-attribute and data-island updates may cause the existing declarative
   runtime to re-render. The section and surviving projected nodes must remain
   structurally stable so focus and selection are not reset.
6. Empty is a settled result, not a pending state. A workflow MUST clear loading
   or busy state before or in the same revision that it presents the empty
   payload. Layout `empty` and layout `loading` are not simultaneous v1 states.
7. Empty state does not inherit through layout descendants. An empty surface does
   not mark nested stacks, grids, lists, tables, or surfaces empty. Independently
   meaningful nested regions require their own explicit state and unique label.

The existing host-attribute observer, CEM-ML conditional rendering, light-DOM
diffing, and default-slot projection are expected to satisfy this contract. If
the red browser fixture shows that stable output or focus requires a new
substrate behavior, stop and promote that behavior as a separate decision.

## Accessibility and interaction

- The named `<section>` provides workflow context. The
  [WAI-ARIA `region` contract](https://www.w3.org/TR/wai-aria-1.2/#region)
  requires a brief accessible name and recommends limiting regions to important
  sections; this is why the semantic surface owns the state and generic stacks
  or grids do not.
- Empty guidance is ordinary visible document content. `data-state="empty"` is
  a styling and test hook, not an accessibility role or replacement for the
  message.
- The surface MUST NOT add `role="status"`, `role="alert"`, `aria-live`,
  `aria-atomic`, or focusability automatically. Initial empty content does not
  need an announcement merely because it is empty.
- If an asynchronous user action makes the visible empty message a status
  update, the workflow author may place `role="status"` on a dedicated,
  non-interactive message node. Do not put the whole surface or its recovery
  control in the live region. This follows the
  [WCAG 2.2 status-message guidance](https://www.w3.org/WAI/WCAG22/Understanding/status-messages),
  which applies when visible status text is added without moving focus and warns
  against unnecessarily chatty live regions.
- The recovery link or control keeps its native role, accessible name, keyboard
  behavior, and normal tab position. Do not add roving `tabindex` or composite
  roles.
- A transition to or from empty MUST NOT move focus automatically. The workflow
  that removes a focused descendant owns any necessary focus recovery because
  only that workflow knows the initiating control and valid destination.

## Ownership boundaries

- `content:empty` remains owned by collection content such as `cem-list` and
  `cem-table`, which already provide collection-local `No items` and `No rows`
  fallbacks. A surface empty state describes the broader workflow result and
  supplies contextual recovery; it must not duplicate a nested collection's
  announcement.
- `layout:loading` remains a separate future decision for pending workflow
  content and `aria-busy` timing. Empty means loading has completed with no
  result.
- Feedback components own statuses, alerts, progress, and errors. An empty
  surface does not become feedback merely because its payload may include a
  dedicated status message.
- Resource loading, search/filter logic, permissions, routing, and recovery
  side effects remain application or workflow responsibilities.
- This slice does not add `cem-empty-state`, new slots, generated IDs, hidden
  alternate payloads, automatic announcements, or component-specific event
  handlers.

## Executable acceptance

The implementation slice is complete only when a focused browser test proves:

- ordinary `cem-surface` output remains unchanged and lacks an empty marker;
- `cem-surface[empty]` keeps the same named stable section and exposes exactly
  `data-state="empty"`;
- authored visible guidance and a native next-action path survive light-DOM
  projection and remain available before and after upgrade;
- presence-only initialization and host-attribute transitions update the marker
  without synthesizing content, creating a slice/event, or losing focus from a
  surviving recovery control;
- empty `cem-stack` and `cem-grid` remain unlabelled generic layout containers
  without empty markers or fallback UI;
- the surface does not gain live-region, alert, composite, or focusable
  semantics; and
- only the `layout:empty` audit row changes after the red fixture identifies the
  missing state reflection.
