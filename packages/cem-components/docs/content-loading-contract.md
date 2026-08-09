# Content Loading Contract

**Status:** Accepted Phase 4 contract; implementation is pending. This decision
is promoted by [`docs/todo.md`](../../../docs/todo.md). No fixture or runtime
branch is part of this contract slice.

## Decision

`cem-card[busy]` owns the first Phase 4 content-loading state. A card is the
smallest existing primitive that consistently bounds replaceable workflow
content across the asset, profile, discussion, authentication, and settings
fixtures. The presence-only `busy` attribute says that the card's resource or
workflow owner is updating that content.

The card projects state; it does not perform asynchronous work. While `busy` is
present, the existing named `<section>` gains exactly
`data-state="loading"` and `aria-busy="true"`. Its header, body, and authored
payload remain in place. Authors retain last-known content during a background
refresh, or author a visible loading message and `cem-skeleton` placeholders
that approximate the final body during an initial load.

`cem-list`, `cem-table`, and `cem-media-preview` do not gain loading behavior in
this slice. They may participate inside a busy card, but they do not infer a
pending state from absent children, an image request, or a parent card.

## Alternatives considered

| Shape | Decision | Reason |
| --- | --- | --- |
| Explicit `cem-card[busy]` with retained or authored default payload | Accepted | The card already supplies a stable, named header/body boundary around the audited workflows without making every content container busy. |
| `cem-list[busy]` | Deferred | Passive and selectable list modes have distinct native output, and the passive mode already owns collection-local `No items` fallback. Loading must not be inferred from an absent option or item. |
| `cem-table[busy]` | Deferred | A table owns row semantics and a collection-local `No rows` fallback. A table-specific loading contract would need to define valid placeholder row structure separately. |
| `cem-media-preview[busy]` | Deferred | A media preview is too narrow for profile and discussion loading, and native image/resource loading is not evidence that the entire workflow content is pending. |
| A card that fetches resources or observes descendant requests | Rejected | Request identity, resolution, cancellation, timing, success, and failure belong to the application or workflow that owns the resource. |
| Component-generated message, skeleton count, or alternate loading slot | Rejected for v1 | The component cannot infer final geometry or useful loading language. A single authored payload preserves meaningful pre-upgrade content and avoids two simultaneously visible payloads. |
| Make the card a `status` live region | Rejected | `role="status"` implicitly creates a polite, atomic live region. Applying it to a card containing content and controls would announce too much and conflate a busy region with feedback. |

## Author API

Initial loading uses authored placeholders:

```html
<cem-card label="Assets" busy>
  <span slot="title">Assets</span>
  <p>Loading assets…</p>
  <cem-skeleton label="Asset rows"></cem-skeleton>
  <cem-skeleton label="Asset preview"></cem-skeleton>
</cem-card>
```

A background refresh keeps the useful payload in place:

```html
<cem-card label="Profile" busy>
  <span slot="title">Profile</span>
  <p>Grace Hopper</p>
  <a href="/profile/edit">Edit profile</a>
</cem-card>
```

| Attribute or payload | Contract |
| --- | --- |
| `label` | Required non-empty accessible name for the card region. It names the content boundary, not the operation; a visible title SHOULD use matching language. |
| `busy` | Presence marks the card content as pending an application- or workflow-owned update. Absence preserves the existing ordinary card. |
| Initial-load payload | MUST contain concise visible loading text and SHOULD include one or more `cem-skeleton` placeholders sized and arranged to approximate the expected final body. |
| Refresh payload | SHOULD retain the last-known usable content so the card does not collapse or lose context while its replacement is prepared. |

`busy` uses WHATWG boolean presence semantics. `busy="false"` still means
present/true and is invalid authoring for a settled card; remove the attribute
when the update settles. The contract deliberately uses the package's existing
`busy` vocabulary rather than `loading`, whose platform meaning already applies
to elements such as images and iframes.

## State and rendering algorithm

1. Treat the card host's `busy` attribute as the only v1 source of content
   loading state. Do not inspect child count, skeleton count, resource requests,
   image events, elapsed time, visibility, or CSS layout.
2. When `busy` is absent, render the existing named `<section>`,
   `.cem-card__header`, `.cem-card__body`, slots, and payload unchanged. Omit
   `data-state` and `aria-busy`.
3. When `busy` is present, render those same stable nodes and payload, and add
   exactly `data-state="loading"` and `aria-busy="true"` to the section. Do not
   synthesize, hide, reorder, or replace authored content.
4. The resource or workflow owner sets `busy` before, or in the same revision
   as, it begins changing the card payload. It commits the final payload while
   the card remains busy and removes `busy` only after that payload is ready.
   This follows WAI-ARIA's guidance to set `aria-busy` while changes are in
   progress and clear it after the update is complete.
5. The card does not create a `busy` or `loading` slice, bind a `slice-event`,
   fetch data, start a timer, accept an `AbortSignal`, dispatch `cem-loaded`,
   `cem-error`, or `cem-cancel`, or select an outcome. Those operations remain
   with the owner of the asynchronous work.
6. Host-attribute and data-island updates may cause the existing declarative
   runtime to re-render. The section, header, body, and surviving projected
   nodes must retain identity so focus, selection, and dimensions are not reset.
7. Busy state does not inherit. Nested cards and contained lists, tables, media
   previews, actions, inputs, progress indicators, and skeletons receive no
   loading state unless their own accepted APIs explicitly provide one.

The existing host-attribute observer, CEM-ML conditional rendering, light-DOM
diffing, and default/named-slot projection are expected to satisfy this
contract. If a red fixture shows that the stable section/header/body or a
surviving focused descendant cannot be preserved without new rendering
substrate, stop and promote that substrate as a separate decision.

## Dimensions and placeholder ownership

The card always retains its section, header, body, and title while busy. That
stable frame prevents a pending update from collapsing the content boundary.
The primitive does not know the final row count, media aspect ratio, text
length, or control layout, so it cannot guarantee body dimensions by itself.

For an initial load, the author owns placeholder quantity and dimensions through
ordinary composition and theme-token styles. `cem-skeleton` is the visual,
layout-preserving placeholder; it remains `aria-hidden` and its `label` is not a
substitute for the visible loading text. `cem-progress` is appropriate when a
workflow can expose meaningful process progress. Neither component starts work,
sets the card busy, or determines when it settles.

For a refresh, retaining the last-known payload is preferred to replacing it
with skeletons because it preserves both geometry and useful context. A
workflow may mark individual controls `disabled` when their operation is unsafe
during the update, but the card does not disable or make all descendants inert.

## Accessibility, interaction, and motion

- The card's named section is the busy region. `aria-busy="true"` exposes that
  the region is being modified and allows assistive technology to defer
  presenting its updates until the attribute is cleared, as defined by the
  [WAI-ARIA `aria-busy` state](https://www.w3.org/TR/wai-aria-1.2/#aria-busy).
- Initial-loading text is visible ordinary content, and `aria-busy` is the
  programmatic waiting-state property. The card MUST NOT add `role="status"`,
  `role="alert"`, `aria-live`, or `aria-atomic`; the package accessibility
  contract intentionally uses the busy transition without an extra live region.
- If an independently announced status message is required, the workflow may
  author a dedicated non-interactive `role="status"` node outside the busy
  subtree. Do not wrap the card or its actions in that role. The
  [WAI-ARIA status role](https://www.w3.org/TR/wai-aria-1.2/#status) is implicitly
  polite and atomic, and the
  [WCAG 2.2 status-message guidance](https://www.w3.org/WAI/WCAG22/Understanding/status-messages)
  applies to waiting-state text inserted without moving focus.
- `aria-busy` is not disabled or inert semantics. Native controls remain in
  their normal tab order and keep native keyboard behavior. The workflow
  disables only controls whose operations conflict with the pending update.
- Entering or leaving busy state MUST NOT move focus. A surviving focused
  descendant retains focus. If final payload replacement removes the focused
  node, the workflow owns recovery because it knows the initiating control and
  valid destination.
- The card branch introduces no animation. Skeleton styling MUST honor the
  package reduced-motion policy; a workflow must not rely on motion alone to
  communicate loading.

## Transitions and outcome ownership

| Transition | Required owner behavior |
| --- | --- |
| Settled to loading | Set `busy` before or with payload updates. Retain last-known content for refresh, or commit the authored initial-loading payload. |
| Loading to content | Commit final content while busy, then remove `busy`. Both rendered loading markers disappear together. |
| Loading to empty | Commit the settled empty payload and remove `busy` in the same revision. A nested list/table may expose its collection-local empty fallback, or the workflow may use the separately accepted `cem-surface[empty]` layout state. |
| Loading to error | Commit the appropriate feedback/error content and remove `busy` in the same revision. A feedback primitive owns alert or status semantics; the card neither synthesizes nor dispatches the error. |

Loading is pending, whereas empty and error are settled outcomes. A card MUST
NOT expose loading and an empty/error marker simultaneously in v1. Cancellation
returns to whichever settled payload the workflow selects and clears `busy`;
the card neither distinguishes nor announces cancellation.

## Ownership boundaries

- `content:loading` covers replaceable content inside a card. It does not close
  the deferred `layout:loading` state for a whole workflow region.
- Lists and tables retain collection content semantics, including their local
  empty fallbacks. A future contract may add structurally valid collection
  placeholders, but this card slice does not.
- Media preview remains a media presentation primitive. Image decoding,
  fallback, and resource errors do not automatically make an ancestor card
  busy.
- Action and input loading must define control-specific disabled, value, focus,
  and event behavior. Existing `cem-action[loading]` is not changed or generalized
  by this contract.
- `cem-progress`, `cem-skeleton`, `cem-alert`, and status content are feedback or
  presentation composed by the workflow; they do not own the card lifecycle.
- Resource selection, fetching, caching, cancellation, retry, timing, routing,
  payload replacement, and outcome decisions remain application or workflow
  responsibilities.

## Executable acceptance

The implementation slice is complete only when a focused browser test proves:

- ordinary `cem-card` output is byte-for-byte structurally unchanged and lacks
  both loading markers;
- `cem-card[busy]` keeps the same named section, header, body, title, and authored
  payload while exposing exactly `data-state="loading"` and
  `aria-busy="true"` on the section;
- `busy` follows presence semantics, including `busy="false"` remaining true;
- authored initial loading text and skeletons remain meaningful progressive
  fallback before upgrade and survive projection after upgrade;
- refresh payload, card frame, and surviving focused controls retain identity
  through busy-on and busy-off host-attribute transitions;
- the card creates no live region, inert subtree, slice, resource request,
  timer, or component event;
- list, table, and media-preview candidates do not infer or inherit loading;
- loading-to-content clears both markers only after final content is committed,
  and loading-to-empty leaves the empty semantics to the collection or workflow
  owner; and
- only the `content:loading` state-matrix audit row changes after the red fixture
  identifies the missing state reflection.
