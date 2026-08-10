# Action Active Contract

**Status:** Implemented Phase 4 contract. This contract is promoted by
[`docs/todo.md`](../../../docs/todo.md), implemented by the public component
stylesheet plus one focused Chromium fixture, and enforced by the style and
state-matrix verification targets.

## Decision

The native enabled `<button>` rendered by each Phase 4 action primitive owns
`action:active` through the browser-defined CSS `:active` pseudo-class:

- `cem-action > button:enabled:active`;
- `cem-icon-button > button:enabled:active`; and
- `cem-menu-item > button:enabled:active`.

The component stylesheet owns only those scoped selectors and their semantic
intent bindings. `@epa-wg/cem-theme` owns the generated active color values.
No runtime slice, host attribute, state class, event handler, or JavaScript
stylesheet side effect represents the transient held state.

Active changes only the enabled button's computed `background-color` and
`color`. It does not change geometry, content, semantics, focus treatment, or
serialized runtime state while the activating input remains held.

| Primitive         | Phase 4 intent | Active pair                                                                        |
| ----------------- | -------------- | ---------------------------------------------------------------------------------- |
| `cem-action`      | `primary`      | `--cem-action-primary-active-background`, `--cem-action-primary-active-text`       |
| `cem-icon-button` | `contextual`   | `--cem-action-contextual-active-background`, `--cem-action-contextual-active-text` |
| `cem-menu-item`   | `contextual`   | `--cem-action-contextual-active-background`, `--cem-action-contextual-active-text` |

The default and hover mappings remain those accepted by the
[`action:hover` contract](./action-hover-contract.md). `primary` remains the
only accepted Phase 4 intent for `cem-action`; icon and menu actions remain
contextual. Arbitrary `variant` strings do not create new theme intents.

## Pointer interaction and observation

The executable pointer path uses the browser provider's real click command
with a bounded delay between native pointer down and pointer up. The fixture
registers a one-shot `pointerdown` listener only to know when the input is held;
it does not dispatch an event or derive presentation from that event.

While the provider command is still pending, the fixture must prove that the
enabled native button:

- matches `:active`;
- resolves both painted colors from the matching active token pair;
- differs from the hover treatment underneath it;
- keeps the same button and host rectangles, HTML, host attributes, focus
  treatment, accessible name, role, and serialized runtime state; and
- has not emitted `click`, `input`, `change`, or a CEM lifecycle event.

After pointer up, the command emits its native click. The button remains
hovered, so its colors return to the matching hover pair. After unhover, they
return to the matching default pair. The rendered button remains the same
light-DOM control throughout; only the existing `pressed` slice on
`cem-action`/`cem-icon-button` or `selected` slice on `cem-menu-item` records
the release-time click.

Disabled buttons are not actionable and never receive the active pair. The
fixture may use the provider's force option solely to send a real coordinate
pointer sequence past its disabled actionability guard. It must not dispatch a
synthetic event. Native disabled semantics suppress `click`, while the
`:enabled:active` selector excludes the token treatment whether or not the user
agent transiently matches `:active` on a disabled element. The disabled button
must never become the focus owner; a real pointer press may still clear focus
from another control according to native browser behavior.

## Keyboard activation

One representative enabled `cem-action` supplies keyboard parity. With the
native button focused and the pointer away, the fixture holds the physical
Space key through the browser provider, observes the same active token pair
while keydown is retained, then releases Space and proves default restoration
plus the native click aftermath. Because the preceding pointer pass already set
the action's `pressed` slice to the string `"click"`, the repeated keyboard
activation is intentionally idempotent in serialized runtime data; the new
trusted click count proves the second release rather than requiring a different
slice value.

Space is the executable keyboard hold because native buttons activate it on
keyup and expose an observable depressed interval. Enter remains normal native
button activation and is already covered by the action click contract; this
slice does not manufacture a durable Enter state when a user agent completes
its activation during keydown.

The active fill must not remove, replace, or geometrically alter the existing
focus indicator. Pointer activation may overlap `:hover`; the later active rule
wins during the hold and release restores hover. Keyboard activation overlaps
focus without requiring hover.

## Contrast, accessibility, and forced colors

The component must consume the active background and text endpoints as a pair.
The focused browser fixture calculates their painted contrast and requires at
least 4.5:1 for the ordinary action text exercised in the light theme. It must
also prove that the active background differs from its underlying hover or
default background so the transient feedback is observable.

The static style gate pins the exact `var(--cem-...)` bindings. Chromium may
quantize a computed `color-mix()` and the canvas-based expected-color normalizer
one 8-bit channel step apart, so the browser comparison permits at most that
single-step rasterization boundary on each channel; it does not permit a
different token or color treatment.

Active adds no role, ARIA attribute, accessible state, live region, or focus
movement. The native button and its accessible name remain the semantic owner.
The release-time click continues through the existing declarative slice-event
path; no active-state runtime data is serialized.

The component rules must not set `forced-color-adjust`, appearance, outline,
shadow, border, opacity, or any other channel. Computed
`forced-color-adjust` remains `auto` before, during, and after activation so
user-agent/system colors may win in forced-colors mode. Theme token generation
continues to own forced-colors mappings; the component binding must not opt out.

## Stylesheet and token contract

Authors continue to load the side-effect-free public exports in order:

```css
@import '@epa-wg/cem-theme/styles.css';
@import '@epa-wg/cem-components/styles.css';
```

The active rules follow their corresponding hover rules so equal-specificity
`:active` treatment wins while both pseudo-classes match. Each rule must:

- start with the public custom-element tag;
- target its direct native button;
- include both `:enabled` and `:active`;
- bind only `background-color` and `color`; and
- reference the exact generated primary or contextual active tokens above.

Raw colors, fallback literals, local custom properties, geometry, motion, and
runtime styling are forbidden. The current generated token catalog contains
all four required active endpoints, so this contract requires no entry in the
[`component CSS exception queue`](./components-css-exceptions.md). If any
required treatment stops being expressible through those semantic endpoints,
implementation must stop and propose a queue entry before changing CSS.

## Alternatives considered

| Shape                                                       | Decision | Reason                                                                                                                                     |
| ----------------------------------------------------------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Native `button:enabled:active` plus generated active tokens | Accepted | It is the browser activation state, composes with hover/focus, and needs no duplicate runtime model.                                       |
| Reflected host attribute or `.is-active` class              | Rejected | A serialized or scripted flag can drift from the input device and replaces rather than observes the native pseudo-class.                   |
| Synthetic `pointerdown`/`pointerup` dispatch                | Rejected | Synthetic events do not create trusted browser activation or reliably match `:active`.                                                     |
| Transform, inset shadow, border, opacity, or animation      | Rejected | Existing paired color endpoints are sufficient; extra channels add untokenized geometry/motion and forced-colors risk.                     |
| Treat post-click `pressed`/`selected` data as `:active`     | Rejected | Those persistent runtime slices are click aftermath, while CSS active exists only during the held input interval.                          |
| Require an observable Enter hold                            | Rejected | Native Enter timing is user-agent-defined and may complete during keydown; Space provides a real, stable keyboard hold without simulation. |

## Executable acceptance

Implementation is owned by one focused browser test in
[`states.browser.spec.ts`](../src/lib/states.browser.spec.ts) named `applies
shared native active treatment during pointer and keyboard activation`.

The fixture covers all three enabled and disabled primitives with native
pointer down/hold/release evidence, plus representative Space hold/release
evidence. It must assert exact token bindings with the bounded painted-color
normalization above, contrast, pseudo-class overlap/restoration, geometry,
DOM/ARIA, focus, forced-color adjustment, runtime timing, disabled suppression,
and event timing.

The style verifier must accept exactly the three new active selectors and their
four generated token endpoints, reject any undeclared action selector or
property, and retain the global token/literal/scope checks. The state audit may
promote only `action:active` after the focused fixture, style gate, and aggregate
package verification are green.

## Stop conditions

Stop without styling or promoting the audit if:

- the Chromium provider cannot expose a real held pointer or Space state long
  enough to observe `:active`;
- the active treatment appears only through synthetic events or a test-only
  class/attribute;
- the generated primary/contextual active pairs are absent, collapse against
  their underlying state, or fail the accepted contrast threshold;
- activation changes geometry, focus ownership, DOM/ARIA, or release-time slice
  semantics; or
- any required CSS value lacks a CEM token and therefore needs an exception
  proposal.

## Implementation evidence

- The tests-first red run kept all 11 prior state tests green, observed the
  trusted held button matching `:active`, and failed only because its painted
  hover background did not equal the generated active background.
- `src/styles.css` adds only the accepted `:enabled:active` background/text
  rules. All declarations use the four generated primary/contextual active
  endpoints; no component CSS exception is required.
- The focused Chromium state target passes all 12 tests. The active fixture
  covers all three enabled and disabled action primitives, pointer and Space
  holds, contrast, overlap/restoration, geometry, DOM/ARIA, focus,
  forced-color adjustment, runtime timing, and native event targeting.
- The exact-selector style gate verifies 32 primitives and 371 generated theme
  tokens. The state audit promotes only `action:active`, yielding 27 covered,
  0 static-only, and 12 gaps with `input:hover` recommended next.
- The uncached aggregate component gate passes all 16 dependencies and 39 tests
  across five files.
