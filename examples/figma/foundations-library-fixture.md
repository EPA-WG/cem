# Figma Foundations Composite Review Fixture

This fixture governs the layering and motion sections of `02 Foundations` when
native Figma variable import cannot represent their canonical composite token
types. It is a review contract, not evidence that the planned canvas assets
already exist.

## Representation cases

| Canonical family | Required Figma representation | Review rule |
| --- | --- | --- |
| Six recess/elevation shadow recipes | Derived Effect Styles | Build each named `CEM/Layering/*` style from the canonical DTCG shadow value; never copy a CSS shadow string. |
| `cem/elevation/0` | Explicit no-effect specimen | Preserve the canonical `none` meaning without inventing a transparent shadow or an empty DTCG shadow array. |
| Five `cem/layer/*` endpoints | Semantic aliases on rung specimens | Annotate the owning rung; do not duplicate Effect Styles with copied effect values. |
| Eight `cem/easing/*` curves | Derived motion specimens | Display the canonical curve name and intent; do not create unsupported native variables or copy CSS timing-function strings. |

## Review procedure

1. Run `yarn nx run @epa-wg/cem-theme:verify:figma-foundations` and use
   `examples/figma/foundations-library.json` as the only layering/motion
   checklist.
2. Use the generated report under `packages/cem-theme/dist/reports/` to obtain
   composite values. The inventory intentionally contains names, ownership,
   locators, and review evidence only.
3. Create the six named Effect Styles exactly once. Represent Base as no effect,
   and annotate semantic endpoints on their owning rung instead of duplicating
   styles.
4. Build all eight motion specimens from their canonical cubic-Bézier values and
   document intended entrance, dismissal, emphasis, uniform, and compatibility
   usage.
5. Review every layering and motion entry in `Light`, `Dark`, `Contrast Light`,
   `Contrast Dark`, and `Native`. Tone and contour remain required fallbacks
   where shadows are unavailable or suppressed.
6. A `planned` entry must retain a null revision. Only a live canvas review may
   promote it to `reviewed` and record a Figma node URL or stable revision.

## Deliberate rejection cases

The review fails if the inventory contains raw shadow/color/dimension/curve
values, an Effect Style duplicates a semantic alias, Base invents a transparent
shadow, a motion curve becomes an unsupported native variable, an entry is
missing or reordered, a locator drifts from `02 Foundations`, a planned entry
claims a revision, or a reviewed entry has no live Figma evidence.
