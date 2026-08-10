# Action Hover Contract

**Status:** Implemented Phase 4 contract. This contract is promoted by
[`docs/todo.md`](../../../docs/todo.md), implemented by the public component
stylesheet and focused browser fixture, and uses the verified theme/component
stylesheet exports pinned by the
[`stylesheet publication contract`](./stylesheet-publication-contract.md).

## Decision

The native enabled `<button>` rendered by each Phase 4 action primitive owns
`action:hover` through the CSS `:hover` pseudo-class:

- `cem-action > button:enabled:hover`;
- `cem-icon-button > button:enabled:hover`; and
- `cem-menu-item > button:enabled:hover`.

The selectors and their component-to-token bindings belong to a static,
author-imported stylesheet published by `@epa-wg/cem-components`. The semantic
default and hover values belong to `@epa-wg/cem-theme`. The JavaScript entry
point remains style-side-effect free: authors load the theme CSS once, then
explicitly import the component stylesheet.

Hover changes only the enabled button's computed `background-color` and `color`
from the matching default action-token pair to its matching hover pair. It does
not change layout, content, semantics, focus, activation, or serialized runtime
state.

| Primitive         | Phase 4 action intent | Default pair                                                                         | Hover pair                                                                       |
| ----------------- | --------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- |
| `cem-action`      | `primary`             | `--cem-action-primary-default-background`, `--cem-action-primary-default-text`       | `--cem-action-primary-hover-background`, `--cem-action-primary-hover-text`       |
| `cem-icon-button` | `contextual`          | `--cem-action-contextual-default-background`, `--cem-action-contextual-default-text` | `--cem-action-contextual-hover-background`, `--cem-action-contextual-hover-text` |
| `cem-menu-item`   | `contextual`          | `--cem-action-contextual-default-background`, `--cem-action-contextual-default-text` | `--cem-action-contextual-hover-background`, `--cem-action-contextual-hover-text` |

`primary` is the only Phase 4 color intent currently accepted for
`cem-action`. The existing `cem-icon-button` `quiet` variant describes its
compact visual treatment; it does not create a sixth theme action intent.
Icon buttons and menu commands are contextual actions, matching the theme's
documented toolbar/menu use of the contextual token family. Additional action
intents or variant mappings require their own documented API decision and are
not inferred from an arbitrary `variant` string.

## Alternatives considered

| Shape                                                                                                    | Decision                            | Reason                                                                                                                                                                                                                                                                                               |
| -------------------------------------------------------------------------------------------------------- | ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Native enabled-button `:hover`, bound in package-owned static CSS to theme state tokens                  | Accepted                            | It is the browser-defined pointer-designation state, covers all three native-button primitives, requires no runtime state, and keeps token values independently themeable.                                                                                                                           |
| JavaScript `pointerenter`/`pointerleave` handlers plus a reflected host attribute or private state class | Rejected                            | Hover is ephemeral presentation, not serializable component state. Event handlers add touch, teardown, rerender, and synthetic-event behavior without improving native semantics.                                                                                                                    |
| Host-only `cem-action:hover` selectors                                                                   | Rejected                            | A host also matches when any rendered descendant is designated. Targeting the actual enabled forwarding button makes the interactive and disabled boundaries explicit.                                                                                                                               |
| Put component selectors in `@epa-wg/cem-theme`                                                           | Rejected                            | The theme owns semantic values and modes, not knowledge of a consumer package's DOM structure.                                                                                                                                                                                                       |
| Leave hover entirely to application CSS                                                                  | Rejected                            | `action:hover` is an accepted package state-matrix requirement and needs one executable implementation owner. Authors may override tokens, but the package supplies the baseline binding.                                                                                                            |
| Gate the selector with `hover`, `any-hover`, or `pointer` media features                                 | Rejected for this visual-only state | The CSS hover media feature describes the primary pointing device, while `:hover` can still match through another device. The treatment neither reveals content nor enables functionality, so direct `:hover` preserves optional pointing-device feedback without making the layout depend on hover. |
| Add border, padding, font, shadow, transform, opacity, generated content, or animation on hover          | Rejected                            | Those channels can change geometry, obscure the token-paired text treatment, or create motion. This slice needs only the existing background/text action-state endpoints.                                                                                                                            |

## Stylesheet and selector contract

The implementation MUST publish an explicit component stylesheet export that
authors can load after the generated theme CSS. Its stable public shape is:

```css
@import '@epa-wg/cem-theme/styles.css';
@import '@epa-wg/cem-components/styles.css';
```

Authors may substitute another existing generated CEM theme CSS entry; this
contract does not rename those artifacts. The component package MUST expose
`./styles.css` through its package exports and published `dist` files. Importing
the JavaScript module MUST NOT inject, adopt, or automatically import that
stylesheet.

The canonical source, cacheable copy target, `dist/styles.css` output,
package-export mapping, release path, and npm-pack evidence are defined by the
[`component stylesheet publication contract`](./stylesheet-publication-contract.md).
That contract is implemented and verified; the hover bindings are the first
behavioral rules in its canonical source. The theme package also exposes and
verifies `@epa-wg/cem-theme/styles.css` for export-aware bundlers.

Rules MUST be scoped through the public custom-element tag and then target its
direct native button. A global `.cem-action`, `.cem-icon-button`, or
`.cem-menu-item` selector is not a component ownership boundary, even though
the current light-DOM renderer retains those inner classes for rendered-output
compatibility.

The enabled baseline consumes the matching default background/text pair. The
enabled hover selector consumes the matching hover pair. The implementation
MUST use `:enabled:hover`, not bare `:hover`, so a native disabled button may
still be pointer-designated without receiving an enabled-action treatment.
Disabled colors remain owned by the separate `action:disabled` contract and
native disabled behavior in this slice.

Theme custom properties are the only authored color values. The component CSS
MUST NOT copy generated `color-mix()` formulas, palette values, or literal
colors. If theme CSS is absent, normal CSS custom-property invalidation and the
user agent's button presentation provide fallback; the component runtime does
not synthesize a theme.

## Interaction and input-modality behavior

Selectors Level 4 defines `:hover` as applying while a pointing device
designates an element without necessarily activating it. The native button and
browser hit testing therefore own entry, exit, nested-label/icon designation,
and simultaneous user-action pseudo-classes. See the
[Selectors Level 4 hover definition](https://www.w3.org/TR/selectors-4/#the-hover-pseudo).

The stylesheet does not use `pointerenter`, `pointerleave`, `mouseover`,
`mouseout`, mouse capture, timers, or synthetic events. Hovering MUST NOT:

- dispatch or suppress `click` or a custom event;
- create or mutate a CEM slice, data-island value, host attribute, class, ARIA
  attribute, role, accessible name, disabled state, or tab order;
- focus or blur a control;
- make hidden content visible, add a tooltip, or alter menu expansion;
- move, resize, transform, or replace the host or its button; or
- become required to discover, understand, focus, or activate the command.

Media Queries Level 4 notes that the `hover` feature describes the primary
pointing device, that `:hover` may still match when `hover: none`, and that
layouts must remain fully usable without hovering. This contract therefore
uses the pseudo-class directly and keeps its effect purely visual. See the
[Media Queries Level 4 hover capability](https://www.w3.org/TR/mediaqueries-4/#hover).

On hardware or user-agent modes where `:hover` never matches, the enabled
default treatment remains complete and the action remains operable. A touch
user agent that transiently or persistently matches `:hover` may show the hover
colors, but no content, layout, or behavior changes. The contract does not use
`pointer: fine` as a proxy for input type.

## Geometry, focus, accessibility, and forced colors

The hover rule changes only background and text color. Before, during, and
after hover, each button retains the same bounding rectangle, host placement,
rendered DOM, visible label or icon, and accessible name. The rule does not set
border width, outline, padding, margin, dimensions, font metrics, transform,
box shadow, display, visibility, opacity, or generated content.

User-action pseudo-classes can overlap. Hovering a focused action MUST leave
the same native control focused and MUST NOT remove or replace its
`focus-visible` outline. Pointer down may also match `:active`; that separately
audited state may override the fill while active and then return to hover when
released. This contract does not define the future active-state selector.

The paired background/text theme endpoints own readable state colors across
theme modes. Component CSS MUST consume both endpoints together and MUST NOT
keep default text against a hover background unless those tokens resolve to the
same value. In forced-colors mode, the rule MUST NOT opt out of user-agent color
adjustment with `forced-color-adjust: none`.

Hover provides no accessible state and adds no ARIA. A disabled native button
retains its disabled semantics and never receives the enabled hover pair.
Keyboard and switch-access users receive the independently owned focus and
activation states; nothing is available only through hover.

## Runtime and ownership boundaries

- `@epa-wg/cem-components` owns the scoped selectors, intent bindings, public
  stylesheet export, and browser acceptance fixture.
- `@epa-wg/cem-theme` owns the generated default/hover token values, mode
  variation, and token contrast pairing. This slice does not change theme
  formulas or add tokens.
- `cem-elements` and the CEM component declarations continue to own rendering,
  native-button forwarding, click slices, host observation, and light-DOM
  identity. They gain no hover logic.
- Applications own when and where they load theme/component CSS and may theme
  by overriding semantic endpoints. They MUST NOT need to recreate the baseline
  hover selectors.
- `action:active` is implemented by the companion
  [`action active contract`](./action-active-contract.md). `action:disabled`,
  `action:loading`, navigation hover, input hover, content hover, selected rows,
  tooltips, menus, and richer action variants remain separate state or
  component contracts.

## Executable acceptance

Implementation is owned by one focused browser test in
[`states.browser.spec.ts`](../src/lib/states.browser.spec.ts). The fixture loads
generated theme CSS followed by the author-imported component stylesheet, then
uses the browser runner's real pointer hover/unhover interaction rather than
dispatching synthetic pointer events or adding a test-only state class.

The fixture proves:

- enabled `cem-action`, `cem-icon-button`, and `cem-menu-item` render the same
  native buttons and accessible names before interaction;
- each enabled button begins with its accepted default token treatment, changes
  to the matching hover background/text treatment while the pointer designates
  it, and returns exactly to its prior computed treatment after unhover;
- the hover background is observably different from the default background for
  each primitive under the generated test theme;
- button and host rectangles and placement are identical before, during, and
  after hover;
- button `outerHTML`, host attributes/classes, type, role, ARIA, accessible name,
  focus owner, and data-island/slice snapshot do not change;
- hovering a focused enabled action preserves its visible focus indicator;
- pointer hover over disabled examples does not apply the enabled hover pair or
  change their computed treatment, semantics, focusability, or runtime state;
- hover alone dispatches no click, slice, or custom event and does not activate
  or expand any primitive;
- the style-contract verifier confirms that the package stylesheet uses only
  generated CEM theme tokens, contains no color or geometry literals, and is
  scoped by component element selectors; and
- only the `action:hover` state-matrix row changes from `gap` to `covered`, with
  the exact browser test name and assertions recorded after the fixture passes.

The focused test is named `applies shared native hover treatment without
changing action geometry or semantics`. The previously discovered component
and theme stylesheet export boundaries are resolved by their package-owned
verification targets. Real pointer hover works in the existing Chromium
harness without simulating hover or injecting runtime styles.

## Implementation evidence

- `src/styles.css` contains only the accepted component-scoped default and
  `:enabled:hover` selectors, paired with eight generated `--cem-action-*`
  tokens. No component CSS exception is required.
- The focused browser target passes all 12 state tests and proves treatment,
  restoration, focus, geometry, DOM/ARIA, disabled, runtime, and event
  invariants for the three action primitives.
- `@epa-wg/cem-theme:verify-package` proves the public `./styles.css` export and
  dry-run npm inclusion of `dist/lib/css/cem-combined.css`; the component style
  gate validates exact generated-token mappings and selector scope.
- The state-matrix audit now covers both `action:hover` and the separately owned
  `action:active`, yielding 27 covered, 0 static-only, and 12 gap rows with
  `input:hover` recommended next.
- The uncached aggregate gate passes 16 dependencies and all 39 package tests
  across five files, including the 12-case focused state suite.
