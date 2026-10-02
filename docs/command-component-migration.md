# Icon-button and menu-item migration

2026-09-28. Both components now use per-component XHTML/CEM-ML declarations,
embedded scoped CSS, and colocated CSF Next stories. Their old registry entries
and global CSS rules have been removed. The migration inventory contains five
canonical components and 45 remaining legacy components.

## Attribute coverage

| Component | Explicitly implemented attributes |
| --- | --- |
| `cem-icon-button` | `image`, `size`, `direction`, `label`, `aria-label`, `href`, `type`, `name`, `value`, `form`, `formaction`, `formenctype`, `formmethod`, `formtarget`, `formnovalidate`, `variant`, `disabled`, `selected`, `selectable`, `pending`, `hidden`, `class` |
| `cem-menu-item` | `label`, `disabled`, `expanded`, `hidden`, `class` |

Icon-button's variant selects primary (default), explicit, contextual, alternate
or destructive action paint in both modes; native button/form attributes and
action bend classes are supported;
image selects the embedded icon. Present href switches to native link navigation.
Menu-item's default slot replaces its fallback label. Both are native command
buttons, with menu-item exposing the menuitem role. Disabled uses attribute
presence, including `disabled="false"`. Click event slices remain `pressed` and
`selected`, respectively. Icon-button supports container-owned persistent
selection and pending feedback through the action theme; clicks do not toggle
its selected attribute.

Each companion playground edits every component attribute, explicitly
inventories all supported attributes, supplies live examples, and shows the
canonical source. Shared `demo.css` makes fieldsets wrap and options stack
vertically. Published pages load the same shared stylesheet. Both declarations
also ship in the generated XHTML bundle with their source-base metadata.

## Preserved tests

All ten new stories pass. Attribute mutation/removal, projected content,
accessible names, native pointer/Enter/Space activation, event metadata,
disabled suppression and retained DOM identity are covered. The original hover
and active tests were moved into each component's stories, retaining geometry,
paint tokens, contrast, focus, held-state equality and post-release event checks.
State-matrix and parity evidence now refer to the colocated tests.

The remaining legacy state file reports **4 passing / 15 failing**. Its failed
test names are a subset of the earlier **3 passing / 18 failing** baseline.
The two action interaction tests now pass in their component stories; the mixed
state test retains its tabs assertions. These results do not claim that the
remaining legacy component suite is green.

## Radio feedback

The user chose radio-only selection feedback changes. Native checked dots now
convey selection without an additional CEM stripe. Checkbox and switch styling
is unchanged.

Chromium ignores `border-radius` on an input with native radio appearance. A
noninteractive `::before` on the input therefore paints circular focus, invalid
and pending feedback with `--cem-bend-circle`. The input retains native appearance
and checked semantics; its own rectangular shadow/outline is suppressed.
Forced colors paint the decorative ring with system colors. Five theme modes,
state transitions, geometry and native appearance are checked independently of
legacy attribute-binding failures.

## Verification

- Ten colocated component stories pass.
- Declarative architecture, style contract, state matrix and Material parity
  gates pass; package build and clean archive validation pass.
- All six playgrounds pass from source and isolated package archives, including
  complete new attribute inventories, responsive shared layouts and all five
  canonical bundle fragments.
- Native input feedback passes across ten controls, five theme modes and forced
  colors. Radio checks assert native appearance, decorative-only pointer handling,
  circular feedback and absence of a rectangular input shadow/outline.
- Component lint passes with existing warnings. The remaining legacy state
  failures are documented above.

## Next

Migrate `cem-field` and `cem-text-field` with the same explicit boolean-presence
and full-attribute coverage, then compare the remaining state failures again.
