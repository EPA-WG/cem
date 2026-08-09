# Layout Loading Contract

**Status:** Implemented Phase 4 contract. This contract is promoted by
[`docs/todo.md`](../../../docs/todo.md) and is covered by the focused browser
test in [`states.browser.spec.ts`](../src/lib/states.browser.spec.ts).

## Decision

`cem-surface[busy]` owns the Phase 4 layout-loading state. A surface is the
existing named boundary for a grouped workflow region, so it can identify the
whole layout whose children are being updated without adding state semantics to
generic formatting containers. The presence-only `busy` attribute says that an
application- or workflow-owned update is pending for that surface.

The surface projects state; it does not perform asynchronous work. While `busy`
is present, the existing named `<section>` gains exactly
`data-state="loading"` and `aria-busy="true"`. Its tone, default payload, child
placement, and stable section remain in place. Authors retain the last-known
layout during refresh, or provide visible loading text and layout-preserving
composition such as `cem-skeleton` and optional determinate `cem-progress`
during an initial load.

`cem-stack` and `cem-grid` remain formatting-only containers. They may arrange
content inside a busy surface, but they do not gain loading markers, infer state
from descendants, or propagate an ancestor's state.

## Alternatives considered

| Shape                                                                                      | Decision        | Reason                                                                                                                                                                                              |
| ------------------------------------------------------------------------------------------ | --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Explicit `cem-surface[busy]` with retained or authored default payload                     | Accepted        | The surface already provides the stable named workflow boundary and owns the category's settled empty state. It can mark the whole updating layout without changing the accepted primitive catalog. |
| `cem-stack[busy]`                                                                          | Rejected        | A stack is a generic single-axis formatter used throughout component internals. It has no accessible-name contract and cannot distinguish a workflow update from ordinary composition.              |
| `cem-grid[busy]`                                                                           | Rejected        | A grid controls child placement, not resource or workflow lifecycle. Making it busy would attach user-facing semantics to reusable visual geometry.                                                 |
| Infer loading from a busy descendant, skeleton, progress indicator, or unresolved resource | Rejected        | Descendant state and placeholder presence do not identify the owner, scope, timing, or outcome of the whole workflow update.                                                                        |
| Add `cem-loading-surface` or a named loading slot                                          | Rejected for v1 | A new primitive or alternate payload is unnecessary for the audited state and would either expand the 32-component MVP or expose two payloads before upgrade.                                       |
| Make the surface fetch resources or coordinate descendant requests                         | Rejected        | Resource identity, resolution, caching, cancellation, timing, and outcome selection belong to the application, loader, or workflow.                                                                 |
| Make the whole surface a status live region                                                | Rejected        | A surface may contain extensive content and controls. Announcing that subtree as a status would conflate a busy region with feedback and can produce excessive announcements.                       |

## Author API

Initial loading uses authored, progressively visible placeholders:

```html
<cem-surface label="Asset workspace" busy>
    <h2>Asset workspace</h2>
    <p>Loading filters and results…</p>
    <cem-stack gap="md">
        <cem-skeleton label="Asset filters"></cem-skeleton>
        <cem-skeleton label="Asset results"></cem-skeleton>
    </cem-stack>
</cem-surface>
```

A background refresh retains the useful layout:

```html
<cem-surface label="Profile workspace" busy>
    <h2>Profile</h2>
    <cem-grid columns="2" gap="lg">
        <cem-card label="Contact details">…</cem-card>
        <cem-card label="Preferences">…</cem-card>
    </cem-grid>
</cem-surface>
```

| Attribute or payload | Contract                                                                                                                                                                                           |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `label`              | Required non-empty accessible name for the workflow region. It names the stable layout boundary, not the pending operation. A visible heading SHOULD use matching language.                        |
| `busy`               | Presence marks a pending application- or workflow-owned update to the surface layout. Absence selects the ordinary or settled-empty state.                                                         |
| `empty`              | Presence marks the separately accepted settled-empty outcome only when `busy` is absent. While both host attributes are briefly present during a transition, `busy` has rendered-state precedence. |
| Initial-load payload | MUST contain concise visible loading text and SHOULD preserve expected geometry through authored layout, `cem-skeleton`, or meaningful determinate progress.                                       |
| Refresh payload      | SHOULD retain the last-known useful children and placement until the replacement layout is ready.                                                                                                  |
| `tone`               | Retains its existing visual contract and does not determine loading semantics.                                                                                                                     |

`busy` uses the package's WHATWG boolean-presence convention. `busy="false"`
is still present/true and is invalid authoring for a settled surface; remove the
attribute when the update settles. The contract uses the existing `busy`
vocabulary rather than `loading`, whose platform meaning already applies to
elements such as images and iframes.

## State and rendering algorithm

1. Treat the surface host's `busy` attribute as the only v1 source of layout
   loading state. Do not inspect child count, descendant state, skeletons,
   progress indicators, requests, image events, elapsed time, visibility, or CSS
   layout results.
2. When neither `busy` nor `empty` is present, render the existing named surface
   `<section>`, classes, tone, and default payload unchanged. Omit `data-state`
   and `aria-busy`.
3. When `empty` is present and `busy` is absent, preserve the accepted
   layout-empty behavior: render the same stable section and payload with only
   `data-state="empty"`.
4. When `busy` is present, render that same stable section, classes, tone,
   accessible label, and payload with exactly `data-state="loading"` and
   `aria-busy="true"`. `busy` has precedence if `empty` is also temporarily
   present, so the rendered section never exposes loading and empty together.
5. The workflow owner sets `busy` before, or in the same revision as, it begins
   changing the payload. It commits the final payload while the surface remains
   busy and removes `busy` only after the layout is ready.
6. The surface does not create `busy`, `loading`, or `empty` slices; bind a
   `slice-event`; fetch data; start a timer; accept an `AbortSignal`; dispatch a
   lifecycle event; select an outcome; or synthesize status, placeholder, empty,
   error, or recovery content.
7. Host-attribute and data-island updates may cause the existing declarative
   runtime to re-render. The section and surviving projected nodes must retain
   identity so focus, selection, dimensions, and child placement are not reset.
8. Busy state does not inherit. Nested stacks, grids, cards, lists, tables,
   controls, progress indicators, skeletons, and surfaces receive no loading
   marker unless an independently accepted API is explicitly authored on that
   component.

The existing host-attribute observer, CEM-ML conditional rendering, light-DOM
diffing, and default-slot projection are expected to satisfy this contract. If
the red browser fixture shows that the stable section, surviving descendants,
or child placement cannot be preserved without new rendering substrate, stop
and promote that substrate behavior as a separate decision.

## Dimensions and placeholder ownership

The surface always retains its named section and authored layout while busy.
That stable boundary prevents the workflow region itself from disappearing,
but the primitive cannot infer final rows, columns, responsive placement, media
ratios, text length, or control dimensions. It therefore does not add a fixed
height, minimum size, overlay, skeleton, or progress indicator.

For an initial load, the author owns placeholder quantity, arrangement, and
dimensions through ordinary `cem-stack`, `cem-grid`, `cem-skeleton`,
`cem-progress`, and theme-token composition. Visible loading text supplies the
human-readable pending state; a skeleton remains a visual placeholder rather
than an announcement.

For a refresh, retaining the last-known layout is preferred because it preserves
geometry and context. The workflow may disable individual controls whose
operations conflict with the update, but the surface does not disable or make
all descendants inert.

## Accessibility, interaction, and motion

- The surface's named section is the busy region. `aria-busy="true"` identifies
  that it is being modified and lets assistive technologies defer exposing its
  changes until the update is complete, as defined by the
  [WAI-ARIA `aria-busy` state](https://www.w3.org/TR/wai-aria-1.2/#aria-busy).
- Initial-loading text is visible ordinary content. The surface MUST NOT add
  `role="status"`, `role="alert"`, `aria-live`, or `aria-atomic` to itself.
- If a separately announced waiting or completion message is required, the
  workflow may author a dedicated non-interactive status node outside the busy
  subtree. Do not wrap the surface or its controls in that role. The
  [WCAG 2.2 status-message guidance](https://www.w3.org/WAI/WCAG22/Understanding/status-messages)
  applies when visible status text changes without moving focus.
- `aria-busy` is not disabled or inert semantics. Native controls retain their
  normal keyboard behavior and tab order unless the workflow explicitly
  disables an individual conflicting operation.
- Entering or leaving busy state MUST NOT move focus. A surviving focused
  descendant retains focus. If final payload replacement removes it, the
  workflow owns recovery because it knows the initiating control and valid
  destination.
- The surface branch introduces no animation or overlay. Skeleton and progress
  styling MUST continue to honor the package reduced-motion policy, and motion
  must not be the only loading cue.

## Transitions and outcome ownership

| Transition                   | Required owner behavior                                                                                                                                                                                               |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Ordinary or empty to loading | Set `busy` before or with payload updates. Because `busy` has precedence, an existing `empty` marker is replaced directly by loading; remove the obsolete `empty` host attribute after `busy` is established.         |
| Loading to content           | Commit the final payload while busy, ensure `empty` is absent, then remove `busy`. Both loading markers disappear together.                                                                                           |
| Loading to empty             | Commit the final empty payload and add `empty` while busy, then remove `busy`. The section changes directly from `data-state="loading"` to `data-state="empty"` without exposing both or flashing the ordinary state. |
| Loading to error             | Commit the appropriate feedback/error payload while busy, ensure `empty` is absent, then remove `busy`. A feedback primitive owns alert or status semantics.                                                          |
| Loading to cancellation      | Restore or retain the workflow-selected settled payload and state, then remove `busy`. The surface neither distinguishes nor announces cancellation.                                                                  |

Loading is pending; empty and error are settled outcomes. Persistently
authoring both `busy` and `empty` is invalid even though the deterministic busy
precedence makes ordered transitions and malformed markup safe. The rendered
section exposes at most one `data-state` value and never combines loading with
empty or error markers in v1.

## Ownership boundaries

- `layout:loading` covers the whole named workflow region. The existing
  `content:loading` contract remains the narrower `cem-card[busy]` boundary for
  independently replaceable card content. Authors SHOULD select the narrowest
  meaningful owner rather than redundantly marking both.
- `layout:empty` remains the settled `cem-surface[empty]` outcome. Loading-to-
  empty ordering is owned by the workflow, with `busy` precedence preventing an
  intermediate or simultaneous rendered state.
- `cem-stack` and `cem-grid` continue to own placement only. A `busy` attribute
  authored on either has no component semantics and must not produce markers.
- Feedback components own progress, statuses, alerts, and errors. They may be
  composed inside or alongside a busy surface but do not control its state.
- Action, input, collection, and media loading require their own component-
  specific interaction and structural contracts; the surface does not confer
  those states on descendants.
- Resource selection, fetching, caching, cancellation, retry, timing, routing,
  payload replacement, and outcome decisions remain application or workflow
  responsibilities.

## Executable acceptance

The implementation slice is complete only when a focused browser test proves:

- ordinary and settled-empty `cem-surface` output remains unchanged when
  `busy` is absent;
- `cem-surface[busy]` keeps the same named section and authored payload while
  exposing exactly `data-state="loading"` and `aria-busy="true"`;
- `busy` follows presence semantics, including `busy="false"` remaining true;
- authored initial loading text and layout-preserving placeholders remain
  meaningful progressive fallback and survive projection after upgrade;
- refresh payload, surface dimensions, child placement, stable section,
  surviving descendants, and focus survive busy-on and busy-off transitions;
- simultaneous host `busy` and `empty` renders only loading, and removing
  `busy` changes the same section directly to exact empty state;
- the surface creates no live region, inert subtree, slice, request, timer,
  synthesized content, or lifecycle event;
- `cem-stack[busy]`, `cem-grid[busy]`, and descendants inside a busy surface do
  not own, infer, or inherit layout loading; and
- only the `layout:loading` state-matrix row changes after the red fixture
  identifies the missing state reflection.

The browser test `marks explicit busy workflow surfaces without making
formatting containers loading owners` is the executable owner of these
requirements. It covers progressive fallback, ordinary/empty/busy surfaces,
retained refresh layout, presence-only initialization, exact state and ARIA
reflection, busy/empty precedence, stable rendered-node/dimension/placement/
focus behavior, ignored stack/grid candidates, descendant non-inheritance, and
the absence of automatic live, inert, slice, resource, and lifecycle-event
behavior.
