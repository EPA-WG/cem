# Typed XSLT runtime lowering

`cem_ql::xslt::compiler::compile_xslt_bundle(source, source_uri)` compiles a
bounded XSLT 3.0 stylesheet into the [portable XSLT bundle](xslt-bundle.md).
It returns binary bytes, their content hash, the stylesheet source hash and
inspectable generated CEMT. Compilation does not accept runtime documents.
The same bundle evaluates changed retained CEM documents on every render.

The transform/CLI adapter now executes this typed bundle path. Legacy execution
is retired, as selected by the user in XSLT-VIEW-MATCH. The public adapter type
and registration ID remain stable, but source must declare the XSLT namespace
and version `3.0`. Standard `application/xslt+xml` and `text/xsl` media types,
and the historical custom-element-XSLT aliases, all select this strict profile.
There is no fallback, version replacement, namespace injection, or XPath token
rewriting. Standalone legacy conversion commands remain separate authoring tools.

Consumers declare template parameters explicitly and replace legacy EXSLT
shortcuts with supported standard expressions over retained input documents.
The migration fixture verifies that both public compilation APIs reject version
1.0 and unbound `xsl` prefixes. This is a bounded runtime profile, not the
completed data-table viewer or full XSLT 3.0 implementation. Standard parsing,
output and browser component URL loading remain in [todo.md](todo.md).

## Supported authoring profile

The stylesheet must declare the XSLT namespace and version `3.0`. The native
`document` host binding supplies its initial context. Default entry selection
applies templates in the default mode; an explicit named entrypoint receives
that context with position and size both one.

| Construct | Runtime behavior |
| --- | --- |
| Named templates and `xsl:call-template` | Recursive calls, expanded names, restored caller focus, explicit `xsl:param` / select-based `xsl:with-param`. |
| Parameters | Omitted values evaluate their defaults in callee focus; supplied empty sequences stay empty. Required and duplicate parameters are diagnosed. Caller-local variables do not leak into the callee. |
| `xsl:apply-templates` | Explicit/default child selection, optional sorting and resulting sequence position/size. One named mode, `#default`, and invocation-only `#current`. |
| Node match patterns | Root, child/attribute paths, descendant separators, namespace-aware names, bare kind tests, targeted processing instructions, predicates and unions. Unlisted pattern syntax is rejected. |
| Rule ordering | Import precedence, exact decimal priority, then declaration order. Union branches retain their individual default priorities. |
| Built-in rules | Elements/documents recurse in the current mode and forward supplied parameters; text/attribute nodes emit their string value; other nodes emit nothing. Atomic/map/array dispatch is outside this profile. |
| `xsl:import` / `xsl:include` | Explicit closed source graph; includes share precedence and later imports override earlier imports. Named calls resolve the winning declaration across the closure. |
| `xsl:for-each select` | Native item sequence, one-based position and sequence size; nested loops restore the outer focus. |
| `xsl:for-each-group select group-by` | Non-composite, Unicode codepoint grouping with population focus, multiple/empty keys, first-seen groups and native members. |
| `xsl:sort` | Stable multiple keys on loops, template application and groups; dynamic direction, text/number conversion, stability and codepoint collation controls. |
| `current-group()` / `current-grouping-key()` | XSLT dynamic context across non-streaming template calls, restored nested groups, lazy absent-context errors and absent state inside invoked function bodies. |
| Local `xsl:variable name select` | Expanded names and lexical scope; the value is evaluated before the new binding exists. Native node owners, map values and array member sequences are preserved. |
| `xsl:if test`, `xsl:choose/when/otherwise` | XPath effective boolean values; only the chosen branch runs. |
| `xsl:value-of select` | Simple content construction, with a literal `separator` or the default space. Empty text nodes are removed, adjacent text nodes merge, and atomized string values are joined. |
| `xsl:text` | Literal text, including whitespace, entity references and CEMT delimiter characters. |
| Literal HTML elements and attributes | Plain result elements and static attributes, including escaped literal AVT braces. Stylesheet text/CDATA/entity chunks join before whitespace-only text is stripped. |

Paths, predicates, variables, sequences, map/array operations and other XPath
syntax remain typed XPath programs evaluated by the existing shared native
XPath engine. The compiler does not rewrite authored XPath tokens into
CEM-QL. It adds typed boolean/simple-content wrappers and emits CEMT control
flow that calls bundle-local XPath capabilities. The simple-content wrapper is
a compiler-owned standard XPath expression; its generated nodes are anchored
to the original selection, and authored nodes retain their precise ranges.
Compiler-local variables cannot capture authored variable references.

These behaviors follow the bounded portions of the
[XSLT 3.0 specification](https://www.w3.org/TR/xslt-30/), including simple content
construction (§5.7.2). A stylesheet recognizing version 3.0 is not a claim that
all standard instructions or functions are available.

Unsupported instructions, attributes, output AVTs and unsupported syntax
fail compilation with stylesheet coordinates. The shared XPath evaluator
reports unsupported functions when evaluated; for example `concat()` is not
currently supported, while the standard `||` operator is. XPath evaluation errors
preserve stylesheet coordinates; generic dispatch and missing-required-parameter
failures retain generated CEMT frames. Both discard the whole partial result
through existing protected CEMT rendering. Cancellation and budget errors remain
uncatchable. No substitute output is manufactured.

Standard parsing functions, dynamic output, stylesheet
sidecars and full namespace/output handling belong to the following fixtures.
Global variables/parameters, parameter constructors/types/tunnels, multiple
mode tokens, `#all`, `xsl:mode`, `apply-imports` and `next-match` are also outside
this bounded slice. Parameterized `element(name)`, `attribute(name)`,
`document-node(element(...))` and schema type patterns are rejected because
the current shared XPath kind-test model does not retain their typed arguments.
`script` and `style` literal result elements are rejected until that output profile is
implemented. `xml:space` and other unlisted instruction attributes are also
rejected rather than silently ignored.

## Grouping

The shared XPath group-context extension was approved on 2026-09-18.
`xsl:for-each-group` currently supports non-composite `group-by`, with the default
Unicode codepoint collation or its explicit literal URI. Composite, adjacent,
starting/ending-pattern grouping and dynamic grouping collations
are rejected explicitly. This is the bounded viewer-required grouping slice.

The compiler evaluates the population and each item's authored key expression
once, with the item's population position and size. Keys are atomized and
untyped atomic values become strings. Typed XPath macros use native arrays and
first-seen `distinct-values` representatives, assigning each key to the first
matching representative. This follows XSLT's sequential rule even when numeric
promotion is non-transitive. NaNs group together and incomparable values stay
separate. Multiple keys can place an item in several groups; repeated keys
never duplicate that population position in one group. Empty keys omit the
item. Repeated occurrences of the same node in the population remain distinct
occurrences. CEM item identity and XPath map `op:same-key` do not replace this
comparison contract. See [XSLT 3.0 §14.1 and §14.5](https://www.w3.org/TR/xslt-30/#grouping).

Each group body receives its first member as context item, its group position,
and the total group count. The shared `XPathXsltGroupContext` carries native
members and keys, separately from ordinary XPath focus. Non-streaming template
calls preserve this state; nested grouping restores outer state. Invoked
function bodies have absent group state, including inline functions created
inside a group. Explicit lexical variables can retain group values normally.
Absent `current-group()` / `current-grouping-key()` access raises
`XTDE1061` / `XTDE1071` at the expression's original source location. An
unevaluated branch raises nothing. The compiler rejects these functions in
match patterns with `XTSE1060` / `XTSE1070`, including unreachable branches.
See [XSLT 3.0 §14.2](https://www.w3.org/TR/xslt-30/#func-current-group).

Grouping runs through compiler-owned typed XPath and generic CEMT loops.
Authored XPath nodes retain their namespaces and source ranges; macro variables
cannot capture authored names. Native maps, array members and node owners
survive grouping. There is no grouping algorithm or table projection in the
Rust renderer. Repeated-row detection and first-seen heading unions are authored
in the [native stylesheet fixture](../packages/cem_ql/tests/xslt_grouping.rs).
The same grouping bundle consumes XML, JSON, YAML and CSV after shared import.

This materialized implementation uses pairwise scans and existing XPath
item/text/work limits, plus the caller's cancellation scope. Budget failures
remain uncatchable and discard partial output. Named function references,
streaming and the other unimplemented grouping forms remain outside this
profile. Sorting the resulting groups is described below.

## Sorting

`xsl:for-each`, `xsl:apply-templates` and `xsl:for-each-group` accept multiple
`xsl:sort` keys. The bounded profile supports a `select` expression or the
default `.` key, Unicode codepoint comparison, and dynamic `order`, `stable`,
`data-type` and `collation` AVTs. Without `data-type`, keys keep their native
atomic types, except that untyped values and URIs compare as strings. Supported
comparison families are strings, booleans and numbers. Explicit `data-type`
supports `text` and `number`; other types are diagnosed as unsupported.
Locale/case tailoring, other collations, sequence-constructor keys and
`xsl:perform-sort` remain outside this profile.

Controls evaluate once in the containing instruction's outer focus. Each key
evaluates once in the original population focus; group keys receive the first
member, original group position/count and that group's native members/key.
Processing then uses sorted positions. Sorting groups leaves their members in
population order. Sorting a template selection keeps the caller's group state
and evaluates supplied template parameters in caller focus.

Compiler-owned typed XPath programs build native array records, normalize each
key column to its common numeric comparison type, and apply stable `fn:sort`
passes from the least significant key to the most significant. Reversing both
the input and output of a descending pass preserves ties. `stable="no"` also
returns stable order, one of the orders the standard permits. Authored XPath
remains separate from the compiler's fixed namespace context and local names.
The shared call-depth correction below was approved separately. No shared
sorter, host ABI, document representation or renderer was changed.

Empty keys sort before ordinary keys ascending; NaNs compare equal and precede
other numbers. Text/number conversion uses standard `string()`/`number()`
semantics, including empty values. Mixed float/decimal/double columns use one
common promoted type, including comparisons between two values originally of
the same narrower type. These rules follow
[XSLT 3.0 §13](https://www.w3.org/TR/xslt-30/#sorting), rather than CEM's
`seq:sorted` missing/invalid-last policy.

The stylesheet defines any missing/invalid-last behavior explicitly, for example:

```xml
<xsl:sort select="not(@n castable as xs:decimal)"/>
<xsl:sort select="if (@n castable as xs:decimal) then xs:decimal(@n) else 0"
          order="{$direction}"/>
```

Here the first key separates invalid values; the second key's fallback merely
ties those values. It does not treat invalid data as a valid zero. The native
[sorting fixture](../packages/cem_ql/tests/xslt_sorting.rs) covers both directions,
secondary keys, ownership, original/sorted focus, groups and template calls,
numeric promotion, NaN/infinities, native containers, import formats and limits.
The same fixtures run through the WASM bundle gate.

Static checks reject sort placement, nonempty select-based keys (`XTSE1015`),
`stable` on a later key (`XTSE1017`), and invalid literal order/stability values
(`XTSE0020`). Runtime checks diagnose multi-item keys (`XTTE1020`), incompatible
key families (`XTDE1030`), invalid dynamic order/stability (`XTDE0030`) and
unrecognized collations (`XTDE1035`). Authored expression failures retain exact
stylesheet ranges. Compiler control checks use the existing `report:raise`
capability: their messages identify the stylesheet location, while their source
frames identify generated CEMT. Both paths discard partial output. Native item,
text and work limits and host cancellation remain enforced; no error recovery
can swallow budget/cancellation failures. The next viewer task is XSLT-VIEW-DATA.

### Shared call-depth accounting

WASM sorting exposed a shared CEM-QL budget defect, whose correction the user
approved on 2026-09-18. `EvalCtx::enter_call` had charged `CallDepth` cumulatively,
so completed sequential calls exhausted a supposed nesting limit. The smaller
default WASM limit exposed the error before the native host did.

[`eval.rs`](../packages/cem_ql/src/eval.rs) now checks active nesting against
the existing depth limit; completed and recovered calls release depth normally.
`FunctionCalls` remains cumulative. The configured limits, uncatchable failures,
safe-point checks and bundle ABI are unchanged. Compiler lowering does not split
queries or increase limits to evade accounting.

The four native regressions in
[`call_budgets.rs`](../packages/cem_ql/tests/call_budgets.rs) cover explicit
one/four-worker policies, sequential named/native/lambda calls, completed and
excessive recursion, recovered failures and total-call exhaustion. They run
independently of sorting and host hardware.

## Native ownership and loading

Only stylesheet **authoring** source is parsed by this compiler. Runtime XML,
JSON, YAML, CSV and future external formats resolve through `cem-ml` import
into retained CEM trees. There are no format-specific evaluation branches,
JavaScript document records or serialized AST handoffs. The same XPath
`/*/*` and compiled loop fixture runs over all four current import formats.
See the [external data import rule](cem-data-import-principle.md).

WASM exposes two source entry points:

- `compileXsltBundle(source, sourceUri)` returns portable binary bytes without
  retaining a handle. Native and WASM compilation produce identical bytes.
- `retainXsltStylesheet(source, sourceUri)` compiles and imports those bytes
  through the ordinary bounded bundle host, returning its existing control
  metadata with hashes and a bundle handle.

Both throw explicit JSON **diagnostic metadata** on compile errors, including
source locations. Compiled programs remain binary. Source-loaded and imported
bundles use the same `renderXsltBundle` and `disposeXsltBundle` lifecycle and
retained-document bindings. Disposing a bundle never disposes a caller-owned
input document; stale bundle/document handles fail explicitly.

Native `compile_xslt_bundle_with_options` accepts an expanded named entrypoint,
expanded parameter names mapped to host binding identifiers, and preflighted
`XsltModuleSource` edges. `resolve_xslt_names` resolves host names in the root
stylesheet namespace context. Bindings cannot occupy the compiler's `xslt_`
namespace or `document`. The compiler validates hashes, exact parent/href
edges, reachability, cycles and source ownership without I/O. Import/include
authorization applies only to those source edges; it does not authorize
`document()`, `result-document`, or runtime URI access.

The transform engine discovers stylesheet dependencies at this authoring
boundary and resolves them with its existing policy-controlled resolver. The
bundle retains every source hash and original XPath owner. CEMT module
aliases/visibility and reserved include semantics do not substitute for XSLT
linking. CLI parameters are explicit scalar controls (null supplies the empty
sequence); document records and secondary input bindings are rejected. Native
CEM arenas use the existing retained-tree constructor without serialization.
The adapter preserves the host operation-control scope during rendering.

WASM source entry points currently use default compiler options. Native-built
bundles containing named entries, parameter bindings and module closures load
through the existing explicit WASM bundle API; loading needs no source resolver.

Compiler limits are 128 KiB across the source closure, 64 sources, 128 dependency
edges, 8,192 authoring events per source, 64 nested source levels, 32 dependency
levels, 1,024 include/import expansions, 128 linked declarations and 128 XPath
programs, with at most 250 external parameter bindings. Source URIs leave room for the generated CEMT identity within the
bundle's 4 KiB identifier limit. Existing XPath, bundle and operation limits
also apply. Native CEMT dispatch supplies its fixed 32-call recursion guard;
custom adapter recursion limits are explicitly rejected in this profile.

The CLI style-export success fixture remains explicitly ignored under
XSLT-VIEW-OUTPUT, alongside an active test proving rejection with no output or
sidecars. Restore its link/inline/omit/CSS assertions when that profile is
implemented; do not silently drop stylesheet styles.
