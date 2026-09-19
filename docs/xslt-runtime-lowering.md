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
completed data-table viewer or full XSLT 3.0 implementation. Output construction
and browser component URL loading remain in [todo.md](todo.md).

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

Standard parsing and recovery are documented below. Dynamic output, stylesheet
sidecars and full namespace/output handling belong to the output fixture.
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
can swallow budget/cancellation failures. Parsing and recovery are described below.

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

## Standard parsing and recovery

The user approved the shared parsing and typed-error extension on 2026-09-18.
The standard functions execute inside native XPath programs, including branches,
predicates and inline functions. The compiler preserves authored expressions;
parsing occurs only when their calls are evaluated.

| Function | Bounded native contract |
| --- | --- |
| `fn:parse-xml($text)` | Empty sequence stays empty. Otherwise import an untyped XML document with namespace validation, normalized text, native ownership and source maps. Malformed input raises `err:FODC0006`. |
| `fn:json-to-xml($text [, $options])` | Import directly into the standard functions-namespace CEM node tree. Preserve null/empty values, member order and numeric spelling. Malformed JSON raises `err:FOJS0001`; duplicate rejection raises `err:FOJS0003`. |
| `fn:base-uri` / `fn:document-uri` | New parsed documents use the calling program's source URI as their static base and have no document URI. Import resolves inherited `xml:base` metadata. Provenance remains a separate source identity. |
| `import:parse-csv($text)` / `import:parse-yaml($text)` | With `xmlns:import="urn:cem:import"`, use the common generic-data CEM vocabulary and retained native owner. Malformed input raises `Q{urn:cem:import}invalid-source`. |

The result and option contracts follow
[`fn:parse-xml`](https://www.w3.org/TR/xpath-functions-31/#func-parse-xml) and
[`fn:json-to-xml`](https://www.w3.org/TR/xpath-functions-31/#func-json-to-xml)
within this profile. JSON options support `escape` and duplicate
`retain`/`use-first`/`reject`; unknown option keys are ignored. `validate` defaults
to false; true raises `FOJS0004`. Invalid options raise `FOJS0005`. `liberal=true`
accepts the same strict grammar: no additional deviations are provided. Custom
fallback callbacks remain an explicit unsupported capability. BOMs, surrogate
escapes, replacement/escape behavior and number lexemes resolve in import.
XML declaration encoding is metadata for an already decoded string.

[`import_string`](../packages/cem_ml/src/import/strings.rs) returns a retained
CEM tree or a typed failure distinguishing malformed input, duplicate keys,
resource limits, unsupported capabilities and internal failures. Existing
`import_data`, byte/HTTP loaders and reader reports keep their public contracts.
All parsers, format decoding and AST projections stay at this import boundary;
XPath sees only native CEM nodes. No JSON document records or serialized AST
handoff are introduced. The default JSON reader retains its stricter existing
input profile; the standard string profile is selected explicitly.

String import keeps the 32 KiB input, 64-level and 4096-value/event caps.
XPath also charges input text/work and checks host cancellation before and after
import. XML DTD processing, custom JSON fallback, XML fragments and resource
fetch functions remain unsupported; no parser performs external I/O.
`fn:parse-json` is outside the approved node-tree route.

### Catch clauses

`xsl:try` with a contained sequence constructor lowers to buffered CEMT recovery.
Ordered `xsl:catch` clauses match expanded QNames, EQNames, namespace/local-name
wildcards or `*`. The first match runs; unmatched errors and failures inside a
catch propagate outward. The protected output and local bindings roll back.
Catch bodies retain outer focus, position/size and group context, including when
the failed expression ran inside a called template.

The six standard catch variables are lexically scoped: `$err:code` is a native
`xs:QName`; description/module are optional strings; line/column are optional
integers; `$err:value` is empty for the supported failures. QName string display
and instance tests preserve its type. Existing CEM diagnostic identifiers and
stylesheet source frames remain available, alongside structured error identity
and parser diagnostic metadata. Names are never extracted from error prose.
Parsing, supported cast failures and absent group-context failures carry their
standard error names; other CEM failures retain implementation error names.

The [XSLT recovery contract](https://www.w3.org/TR/xslt-30/#try-catch) permits
buffering with `rollback-output="no"`; this runtime always buffers. Cancellation,
import limits, evaluator budgets and unavailable capabilities remain uncatchable.
Select-based `xsl:try`/`xsl:catch` result construction is explicitly rejected
until the output-profile task; it is not approximated by stringification.

### Verification

Native importer and XPath tests cover decoded strings, namespace/character
errors, BOMs, duplicate policies, escapes, large numeric lexemes, retained
ownership, URI metadata, lazy invocation, typed errors, resource limits and
cancellation. The original prerequisite probes now assert parsing successes
and standard error identity while preserving the legacy reader error contract.
XSLT tests reload portable bundles and cover changed inputs, ordered and nested
catches, named-template propagation, QName variables, group focus, lexical scope,
rollback, CSV/YAML imports and unsupported forms. The import-boundary source
guard includes the new XPath parsing module. The native/WASM bundle gate adds
changed XML/JSON inputs and parse-error recovery through both loading routes.

Verification on 2026-09-18: 419 CEM-QL tests, 101 adapter tests, 297 CEM-ML
integration tests, 118 native/WASM bundle checks, 17 XPath artifact checks and
125 function-companion checks pass. The broad CEM-ML library run passed 2,027
tests; its existing debugger pause/deadline timing test failed under load and
passed in isolation. The integration audit also exposed a stale adapter source
marker from the earlier strict-XSLT migration; updating that marker restored
the original zero-serialization check. CEM-ML, CEM-QL and adapter Nx lint
targets pass with existing warnings.

## Output construction: scope decision pending

XSLT-VIEW-OUTPUT reached a shared renderer boundary on 2026-09-18. The
[scope rule](xslt-data-table-parity.md#scope) requires approval before adding
CEMT/CEM-QL capabilities. Five native probes in
[`xslt_output_prerequisites.rs`](../packages/cem_ql/tests/xslt_output_prerequisites.rs)
characterize the boundary; this checkpoint changes no production behavior.
All 424 CEM-QL tests pass, including the five new probes, with none ignored.
Nx lint passes with existing warnings. No WASM rebuild is needed for this
tests-and-documentation checkpoint.

| Existing path | Observed behavior and implication |
| --- | --- |
| XPath native nodes inserted through a CEMT expression | The owner and source survive, but insertion emits only the node's string value. XML/JSON subtree structure is lost from output. |
| Atomic expression results | Adjacent values become text immediately: `(1, 2)` produces `12`. Their atomic identity is unavailable when the parent is constructed. |
| CEMT element/attribute constructors | An attribute emitted after child text is still attached to the element. The original sequence order is not retained for XSLT validation. |
| Static style output | `{element @name=style}` produces a result node without extracting a component stylesheet. Ordinary `{style}` keeps its existing declaration semantics. This distinction is reusable. |
| XSLT output authoring | AVTs, `xml:space`, result styles and select-based try/catch remain source-located rejections pending lowering. |

[XSLT complex-content construction](https://www.w3.org/TR/xslt-30/#constructing-complex-content)
keeps items until the parent is constructed: adjacent atomic values need space
separation, while adjacent text nodes merge without it. Native subtrees must
remain nodes. Attribute ordering must raise `XTDE0410` where required.
These rules also apply to values returned through
[select-based try/catch](https://www.w3.org/TR/xslt-30/#try-catch).
String interpolation or a generated recursive copy template alone cannot
preserve all these distinctions across calls, loops and recovery boundaries.
Serializing source nodes and parsing the markup back is prohibited by the
[import rule](cem-data-import-principle.md).

### Proposed shared change

Add an explicit native result-construction capability to shared CEMT rendering,
selected by XSLT lowering. Keep ordinary CEMT interpolation and component-style
extraction unchanged. The capability must:

1. Carry native items and constructed nodes until the enclosing result
   element/document consumes them, including across template calls, loops and
   buffered try/catch. Keep atomic values distinct from constructed text.
2. Copy retained CEM/XPath nodes through their common semantic view, preserving
   expanded names, node kinds and source provenance. Never inspect external
   XML/JSON/YAML/CSV parser ASTs, manufacture document records, or reparse output.
3. Expose explicit construction policy and typed failures so the XSLT layer can
   enforce array flattening, atomic spacing, text merging, attribute order and
   duplicate rules, and namespace fixup within its documented bounded profile.
   Unsupported function/map items must fail explicitly. XSLT owns standard
   error names; existing CEMT behavior must not change implicitly.
4. Retain the existing limits, cancellation and buffered rollback guarantees.
   Portable compilation/loading must carry the new capability explicitly and
   reject unsupported forms instead of silently treating them as text.

The proposed extension is a reusable result path. It adds no format reader or
table-specific renderer. All external parsing remains in `cem-ml` import.
Without approval, select-based try/catch stays deferred; ordinary literal HTML,
AVTs, whitespace and static-style work can be considered separately.

### Acceptance after approval

- Lower select-based try/catch and the corresponding sequence-producing
  instructions through native results. Verify XML/JSON subtrees, empty nodes,
  documents, attributes, comments/PIs and source retention after bundle reload.
- Test atomic versus text adjacency across instructions, named calls, loops and
  catches; recursive array flattening; attribute order/duplicate handling;
  namespace behavior; unsupported items; rollback; limits and cancellation.
- Implement output AVTs, whitespace preservation and existing slice/event
  attributes without browser-local interpretation of source documents.
- Keep declaration styles unchanged, lower static result styles as output
  nodes, and restore the CLI link/inline/omit/CSS export fixture through the
  existing output boundary. Do not extract inactive result branches as styles.
- Run native and CLI acceptance first, followed by portable native/WASM checks.
  Replace the current XSLT rejection probes with positive acceptance cases;
  retain the ordinary CEMT behavior probes as compatibility checks.

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
