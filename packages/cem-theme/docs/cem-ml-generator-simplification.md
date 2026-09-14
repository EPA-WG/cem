# CEM-ML/CEM-QL Generator Simplification Analysis

## Scope

The ten CEM theme CSS generators are both executable token documentation and CSS producers. Their current source is
correctly Markdown-driven, but the CEM-ML still contains substantial structural repetition:

- 1,389 lines across the ten `cem-*.html` generator pages;
- 137 `cem:for-each` nodes;
- 497 positional `row.tdN` references;
- 51 copies of the basic `token: value;` emission; and
- 42 copies of the standard token-name table cell.

The color and typography generators need specialized presentation. Most other tables repeat a small set of table-row,
preview, and CSS-declaration patterns.

This analysis distinguishes improvements already expressible with the language from gaps that require a portable
runtime or data-shape addition. Theme-specific convenience must not become a core CEM-ML construct.

## Implementation result

Implemented 2026-09-12. All ten generators retain their Markdown-driven presentation and CSS protocols while sharing a
package-local CEMT module:

| Measure | Before | After |
|---------|-------:|------:|
| Generator source lines | 1,389 | 1,302 |
| `cem:for-each` nodes | 137 | 76 |
| Positional `row.tdN` accesses | 497 | 0 |
| Shared table calls | 0 | 18 |
| Shared CSS-property calls | 0 | 43 |

The generated CSS remains 58,527 bytes across the ten generator outputs. All ten manifest checks, all 479 token-coverage
checks, and the source-completeness/browser-capture verifier pass. Color matrices, typography previews, mode overrides,
and other page-specific projections remain local where their data shapes genuinely differ.

## Existing constructs to use first

### Sequence construction

CEM-QL sequence construction already concatenates streams in source order:

```cem
{cem:for-each
    @select="(datadom.slices.gaps,datadom.slices.insets)"
    @as=row |
    ...
}
```

This is the correct operation for combined presentation tables. `seq:union` is not a substitute because union has set
semantics and may remove duplicate rows. The dimension, layering, stroke, and typography presentations now use the
sequence constructor instead of adjacent identical loops.

### Mixed attribute-value templates

Mixed AVTs already cover most uses of `str:concat`:

```cem
{td @style="background-color: var({row.token}); min-width: 3rem" | }
```

They should replace `str:concat((...))` when the result is ordinary attribute text. Keep `str:concat` for joining a
runtime sequence or when a separator is required. A new variadic concatenation or formatting function would duplicate
existing syntax.

### Local named templates

The raw CEM-QL renderer already supports same-module `{template}` and `{call}` declarations with typed parameters.
Large specialized pages can therefore define a local `zebra-cell`, `standard-token-row`, or `typography-sample`
template and call it repeatedly. This is useful for repetition within one generator and requires no language addition.

Local templates do not solve cross-generator reuse. Copying the same named template into ten files only moves the
duplication.

### CEM-QL block bindings

`{ let name = value; expression }` is already available inside expressions. It is appropriate for repeated intermediate
values within one complex AVT, particularly the action and zebra formulas. It is not a structural replacement for a
reusable render template.

## Highest-value data-shape improvement

The dominant source of fragility was not missing query syntax; it was the browser bridge's positional row shape.
`tokenTableProjection()` now emits named fields and diagnostics, while `tokenTableRows()` remains as its row-only
compatibility facade. This removes each template's private column map and prevents a three-column table such as
typography roles from reading `tier` as if it were a fourth column.

The bridge should add stable, header-derived names while preserving `tdN` during migration:

```text
{
  token: "--cem-control-height",
  value: "2.5rem",
  description: "Generic control height",
  tier: "recommended",
  cells: ["--cem-control-height", "2.5rem", "Generic control height", "recommended"],
  source_table: "cem-controls-geometry",
  source_row: 1,
  td1: "--cem-control-height",
  td2: "2.5rem",
  td3: "Generic control height",
  td4: "recommended"
}
```

Header aliases use deterministic `snake_case` normalization (`Forced-colors value` becomes
`forced_colors_value`). Duplicate, empty, or reserved normalized headers produce diagnostics and remain accessible
through `cells`/`tdN`; they never silently overwrite one another. `cells`, `source_table`, `source_row`, and `tdN` names
are reserved. The Markdown schema package now owns this portable contract, so browser and SSR inputs have one defined
shape rather than a theme-specific JavaScript convention.

This change improves clarity more than adding a `record:get()` helper. `row.value` states the contract; dynamic numeric
lookup merely hides another positional index.

## Highest-value reuse improvement

CEM-native template modules already define the required authoring surface:

```cem
{module @version="1.0.0" |
    {import @as=docs @src="./cem-token-docs.cemt"}
    {body |
        {call
            @from=docs
            @template=token-table
            @with:rows={datadom.slices.geometry}
            @with:preview=width
        }
        {call
            @from=docs
            @template=css-properties
            @with:rows={datadom.slices.geometry}
        }
    }
}
```

The CLI transform adapter already preflighted resolver-backed imports. Browser runtime support now exposes
`preflightCemMlTemplateModules()` and `retainCemMlTemplateModuleClosure()`, and `renderCemMlTemplate()` accepts a
`moduleLoader`. The host resolves each static import through the existing scope-aware module URL service, performs I/O,
and supplies an immutable closure to the Rust/WASM compiler. The compiler performs no I/O, rechecks hashes and runtime
compatibility, namespaces imported templates by resolved URI, preserves nested module calls, and rejects calls to
non-public entrypoints.

The closure identity includes:

- the root template URL and content hash;
- every resolved imported module URL and content hash;
- the active scope/import-map policy stamp;
- the root-body entrypoint and sorted parameter contract; and
- the CEM-ML/CEM-QL compatibility versions.

`cem-css-generator.js` uses the page root plus the template's local module map to build that resolution context. The
default loader fetches only after resolution, so no generator-local URL algorithm exists. A checked-in root/module pair
is rendered by both the native unit fixture and the browser Storybook fixture; nested imports, hash drift, and private
entrypoints have separate native coverage.

## Reusable module boundary

The first shared module should stay small and data-driven:

| Public template | Responsibility |
|-----------------|----------------|
| `standard-token-table` | Standard Token/Value/Tier/Description projection |
| `width-token-table` | Standard projection with the common width swatch |
| `preview-token-table` | Standard projection with a known preview kind |
| `token-table` | Configurable table implementation used by the public wrappers |
| `css-properties` | Ordered `token: value;` text for a row stream |

`token-row` is a private module helper. A generic `mode-overrides` template was deliberately not added: current override
tables use different named columns and selector structures, so a generic version would require dynamic field access or
more parameters than the local loops it replaced.

Preview kinds should initially be a closed module-owned set such as `none`, `width`, `height`, `stroke`, `radius`,
`shadow`, `duration`, and `easing`. Color matrices and typography's semantic sample dispatch remain specialized until a
second real consumer demonstrates a generic abstraction.

CEMT calls currently pass data parameters but not caller render fragments. A higher-order block/template parameter
would make an entirely generic table shell possible, but it should not be added for this migration. A closed preview-kind
parameter covers the repeated theme cases while keeping call targets static and preserving compile-time validation.

## Functions and constructs not justified now

- Do not add a core `cem:token-table` element. Token documentation is an application of CEM-ML, not language syntax.
- Do not add variadic `str:concat` or `str:format`; mixed AVTs and the existing stream-joining `str:concat` cover the
  actual cases.
- Do not use `seq:union` to concatenate table sources; it may alter multiplicity.
- Do not add regex solely for token suffix dispatch. Existing `str:contains`, `str:starts_with`, and `str:ends_with`
  express the current closed naming rules.
- Do not add `css:var()` merely to save `var(...)` characters. Consider a CSS stdlib only when it returns or consumes a
  typed CSS value/AST and provides correctness beyond string assembly.
- Do not expose dynamic template invocation yet. Static `{call @template}` and `{call @from @template}` keep module
  graphs auditable and cacheable.

## Completed order

1. Kept the source-completeness gate for all ten generators.
2. Added portable header-derived row fields and provenance while retaining `tdN` compatibility.
3. Refactored every generator expression from `row.tdN` to named fields.
4. Exposed resolver-preflighted CEMT module closures through browser runtime support and the scoped module resolver.
5. Created and adopted `cem-token-docs.cemt`, with one shared native/browser module fixture.
6. Re-measured the source and reran rendered-DOM, CSS manifest/bytes, diagnostics, coverage, and protocol verification.

No new core CEM-QL function or CEM-ML structural construct was required. The two platform gaps were data/runtime
boundaries—the portable named-field table projection and browser access to the existing resolver-backed CEMT module
graph—and both are now implemented.
