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

The dominant source of fragility is not missing query syntax; it is the browser bridge's positional row shape.
`tokenTableRows()` currently emits only `td1`, `td2`, and so on. That requires every template to repeat a private column
map and caused the typography role table to read its three-column `tier` as if it were the fourth column.

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

Header aliases should use deterministic `snake_case` normalization (`Forced-colors value` becomes
`forced_colors_value`). Duplicate or empty normalized headers must produce a diagnostic and remain accessible through
`cells`/`tdN`; they must not silently overwrite one another. The portable row record and its provenance must be owned by
the Markdown/table projection contract, not only by browser JavaScript, so browser and SSR receive the same shape.

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

The CLI transform adapter can preflight resolver-backed imports, compile the module closure, and execute imported public
templates. The browser `renderCemMlTemplate()` path currently calls the raw `renderTemplateSource` WASM API, which
compiles one source string without a resolved import closure. Therefore a shared imported presentation module cannot be
adopted yet without creating browser/SSR divergence.

The right runtime addition is not new CEM-ML syntax. Runtime support should accept a resolver-produced CEMT module
closure or a compiled artifact whose identity includes:

- the root template URL and content hash;
- every resolved imported module URL and content hash;
- the active scope/import-map policy stamp;
- the selected entrypoint and parameter contract; and
- the CEM-ML/CEM-QL compatibility versions.

Browser resolution must use the same scope-aware module URL service as `cem-module-url`; it must not add generator-local
`fetch()` logic. SSR must consume the same closure/artifact. Once that boundary exists, a package-local
`cem-token-docs.cemt` module can own the repeated standard tables, preview kinds, CSS property blocks, and mode override
blocks.

## Reusable module boundary

The first shared module should stay small and data-driven:

| Public template | Responsibility |
|-----------------|----------------|
| `token-table` | Standard Token/Value/Tier/Description projection with an optional known preview kind |
| `token-row` | One named-field row; retained as a private helper unless consumers need it |
| `css-properties` | Ordered `token: value;` text for a row stream |
| `mode-overrides` | One selector plus one named value column from an override table |

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

## Recommended order

1. Keep the current source-completeness gate for all ten generators.
2. Add portable header-derived row fields and provenance, retaining `tdN` compatibility.
3. Refactor templates from `row.tdN` to named fields; use mixed AVTs and local named templates where they materially
   reduce specialized repetition.
4. Expose resolver-preflighted CEMT module closures/artifacts through browser runtime support using the existing scoped
   module resolver.
5. Create and adopt `cem-token-docs.cemt`; require identical browser and SSR render plans in its acceptance tests.
6. Re-measure source lines, loop count, positional accesses, rendered DOM, generated CSS bytes, diagnostics, and source
   maps. Reduction is accepted only when output and provenance remain equivalent.

No new core CEM-QL function or CEM-ML structural construct is required for steps 1–5. The only current platform gaps are
the portable named-field table projection and browser access to the already-designed resolver-backed CEMT module graph.
