# CEM UI Kit Plan

The CEM UI Kit is the native Figma design library for CEM. It should mirror the generated token and semantic component
surface without making Figma the source of truth.

## Pages

| Page | Purpose |
| --- | --- |
| `00 Cover` | Library title, version, source links, status, and release notes. |
| `01 Tokens` | `CEM Tokens` variable collection, token demos, generator category demos, and validation notes. |
| `02 Foundations` | Color, type, spacing, shape, stroke, layering, and motion examples. |
| `03 Components` | Component sets and variants mapped to CEM tokens. |
| `04 Patterns` | Composite layouts using component sets: forms, lists, profile, assets, and messages. |
| `05 Site Demo` | End-to-end screen examples using library components. |
| `99 QA` | Smoke fixtures, contrast checks, mode checks, and visual parity references. |

## Token Mapping

| UI Surface | Figma Token Binding |
| --- | --- |
| Surface fill | `cem/palette/comfort`, `cem/palette/calm`, `cem/zebra/color/*` |
| Text fill | `cem/palette/*/text`, semantic action text variables |
| Action fill | `cem/action/*` and palette action endpoints |
| Spacing | `cem/gap/*`, `cem/inset/*`, `cem/layout/*/gap` |
| Control geometry | `cem/control/*`, `cem/icon/button/*`, `cem/coupling/*` |
| Corner radius | `cem/bend/*` |
| Stroke | `cem/stroke/*` |
| Typography | `cem/typography/*` size variables and derived text styles |
| Motion notes | `cem/duration/*` string variables and generated motion documentation |

## Component Mapping

| Figma Component | Source Component Plan | Required Variable Bindings |
| --- | --- | --- |
| App shell | `docs/component-mvp.md#component-list` | surface, text, gap, inset |
| Top bar | `docs/component-mvp.md#component-list` | surface, text, stroke, gap |
| Navigation item | `docs/component-mvp.md#component-list` | action state colors, gap, inset, bend |
| Text field | `docs/component-mvp.md#component-list` | surface, text, stroke, bend, gap, typography |
| Checkbox | `docs/component-mvp.md#component-list` | control size, stroke, selected color |
| Button | `docs/component-mvp.md#component-list` | action fill/text, control height, inset, bend |
| Card | `docs/component-mvp.md#component-list` | surface, stroke, bend, gap, inset |
| Data row | `docs/component-mvp.md#component-list` | zebra surface, text, gap, stroke |
| Badge | `docs/component-mvp.md#component-list` | status color, text, bend, inset |
| Message item | `docs/component-mvp.md#component-list` | surface, text, gap, bend |
| Alert | `docs/component-mvp.md#component-list` | semantic action/status color, stroke, gap |

## QA Expectations

- Each component set has at least default, hover, focus-visible, disabled, and invalid/selected variants where
  applicable.
- Each variant uses bound variables rather than copied hex or pixel literals when Figma supports the property.
- Each component is tested in `Light`, `Dark`, `Contrast Light`, `Contrast Dark`, and `Native` modes.
- Screens on `05 Site Demo` are composed from `03 Components` instances rather than detached shapes.
