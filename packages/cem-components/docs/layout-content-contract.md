# Layout and content declaration contract

`cem-surface`, `cem-card`, `cem-text`, `cem-stack`, `cem-grid` and `cem-divider`
load from their canonical XHTML declarations with no native-surface capability.
They render light DOM, consume theme tokens in declaration-owned scoped CSS and
expose `part="root"`. Card additionally exposes `part="header"` and `part="body"`.
The component-owned structure remains stable during attribute changes. Native
controls projected into that structure retain edits and focus during updates;
reconnecting the same host retains the produced content and edits.

| Component | Attributes and defaults | Content and styling |
| --- | --- | --- |
| `cem-surface` | Optional `label`; `tone="default"`; presence `busy`, `empty` | Section with default slot. Comfort palette by default; calm, trust, enthusiasm, creativity, danger and conservative select theme palette pairs. Unsupported tones use comfort. D4 work-layer separation stays independent of tone. |
| `cem-card` | `label="Card"`; presence `busy` | Named section with title slot (label fallback) and default body slot. Comfort palette, D4 work-layer separation, reading body and UI header typography. |
| `cem-text` | `variant="body"`; `text=""` | Inline span. Default slot overrides text fallback. Body uses reading typography, caption uses UI, heading uses UI with heading weight, data uses data typography including tabular lining numerals. Reading uses the optical-sizing endpoint. Unsupported variants use body. Visual heading treatment does not add a heading role. |
| `cem-stack` | `gap="md"` | Vertical flex container with default slot. md/unsupported uses `--cem-layout-stack-gap`, sm uses `--cem-gap-related`, lg uses `--cem-gap-section`. |
| `cem-grid` | `columns="auto"`; `gap="md"` | Default slot in a grid, with the same gap contract as stack. Auto/unsupported uses auto-fit tracks with `--cem-layout-grid-min` as the minimum, capped to available width. Explicit 1–4 columns produce equal flexible tracks. |
| `cem-divider` | `orientation="horizontal"`; `spacing="group"`; presence `inset`, `decorative` | No content slot. Full semantics and geometry are defined in the [divider contract](divider-contract.md). |

Native `class`, `dir` and `hidden` apply to the host; hidden="until-found" keeps
browser-managed behavior. These declarations add no focus stops, custom events,
form value, live region, request lifecycle or local workflow state.

## Explicit loading and empty state

Presence includes empty and false-valued spellings. A busy card or surface sets
`data-state="loading"` and `aria-busy="true"` on its existing section. Removing
busy removes both attributes, except that a surface with empty still present
transitions to `data-state="empty"`. Empty carries no ARIA state. Removing empty
returns the surface to ordinary content. Busy takes precedence over empty.

State changes never replace the section or projected content, generate recovery
messages, disable descendants, make the subtree inert, or move focus. Loading
and recovery content remains authored by the application. Stacks, grids and
nested ordinary surfaces/cards do not infer or inherit these markers.

## Verification

Colocated stories transfer all prior card/surface/divider assertions, cover
attribute presence and live transitions in worker and fallback execution,
retain edits across reconnect, and check token geometry and responsive tracks.
`verify-layout` exercises all six property pages, five-theme galleries,
canonical source fetches and bundled declarations from source and isolated
package archives. `verify-divider-forced-colors` preserves the separator color,
stroke, inset and guarded separation-track assertions.


## CEM principle audit — 2026-10-12

The [canonical theme principles](../../cem-theme/src/lib/tokens/index.md#core-cem-principles)
govern the choices below. Token existence alone is not evidence of the correct
consumer meaning.

| Principle | Decision and evidence |
| --- | --- |
| Semantic intent first | D1 `--cem-layout-grid-min` owns minimum usable grid tracks; D6 reading measure remains a maximum for reading flows. D6 data text consumes tabular lining numerals. Surface/card separation consumes D4 `--cem-layer-work`, never an elevation value as z-index. |
| Bounded variation | Documented gap, tone, orientation, typography and column enums select existing endpoints. Auto layout is intrinsic and container-driven; explicit column counts remain authored choices. Unsupported values use documented defaults. |
| Accessibility by construction | Busy/empty/decorative states preserve native semantics, content and focus; passive components add no interaction owner. Divider D1/D2 guarded spacing and D5 strokes remain independent. Existing native control coupling/focus ownership is retained. |
| Cross-dimension harmony | D0 palette/text pairs, D1 gaps/insets/grid sizing, D3 container bend, D4 work separation, D5 boundary/divider and D6 role typography keep their source-defined purposes. No typography measure controls generic grid geometry. |
| Theme survivability | Five-mode inheritance plays exercise parent overrides on all six components. Surface/card forced colors remove shadow while retaining Canvas/CanvasText contours. The divider keeps its system separator color and geometry. |
| Public token ownership | The new required D1 endpoint lives in the `cem-dim-layout` Markdown table; the existing table-driven generator and manifest emit it. Components introduce no defaults or aliases that block inherited overrides. The only local variable is the private divider spacing calculation. |

Audit corrections: the earlier grid implementation had repurposed a reading
maximum as a layout minimum, and data text had omitted numeral features. The
new regression plays first reproduced both failures. Reading optical sizing
and D4 work-layer separation now also consume their theme endpoints. The
[CEM-CSS-004 decision](components-css-exceptions.md#cem-css-004--intrinsic-grid-track-minimum-adopted-into-theme)
records why the layout endpoint belongs in D1.

Generic surfaces, cards and inline text are not prose-layout containers; authors
still apply D6 maximum measure and paragraph rhythm to long-form reading content.
These browser checks establish the stated component/token contracts, not a
complete physical-device or assistive-technology acceptance run.
