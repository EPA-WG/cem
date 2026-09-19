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
completed browser data-table viewer or full XSLT 3.0 implementation. Native
output construction and the base viewer stylesheet are implemented; imported
presentation aspects and browser component bindings remain in [todo.md](todo.md).

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
| Literal result elements and attributes | Expanded names, output AVTs (including escaped braces), and unchanged declarative `slice`, `slice-event`, `slice-value` attributes. |
| Whitespace | Text/CDATA/entity chunks join before whitespace-only nodes are stripped; inherited `xml:space="preserve"` / `"default"` controls stripping. |
| `xsl:sequence`, `xsl:copy-of` | Select native nodes or values; retain them across calls, loops and recovery until the parent constructs its content. The bounded untyped copy profile supports `select` only. |
| `xsl:document`, `xsl:element`, `xsl:attribute` | Native document/element construction and attribute simple content, including attribute `select`. Element/attribute names and namespace URIs must be static in this profile. |
| `xsl:try` / `xsl:catch` | Contained constructors or `select`, ordered error-name matching and buffered rollback. |
| Static literal `style` | Result text is emitted only by the selected branch; CLI exports support linked, inline, omitted and CSS output. |

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

Unsupported instructions, attributes and unsupported syntax
fail compilation with stylesheet coordinates. The shared XPath evaluator
reports unsupported functions when evaluated; for example `concat()` is not
currently supported, while the standard `||` operator is. XPath evaluation errors
preserve stylesheet coordinates; generic dispatch and missing-required-parameter
failures retain generated CEMT frames. Both discard the whole partial result
through existing protected CEMT rendering. Cancellation and budget errors remain
uncatchable. No substitute output is manufactured.

Standard parsing, recovery and native output are documented below. Full namespace
node handling, schema-aware output, serialization controls, dynamic constructor
names/namespace AVTs and secondary result documents remain outside this profile.
Global variables/parameters, parameter constructors/types/tunnels, multiple
mode tokens, `#all`, `xsl:mode`, `apply-imports` and `next-match` are also outside
this bounded slice. Parameterized `element(name)`, `attribute(name)`,
`document-node(element(...))` and schema type patterns are rejected because
the current shared XPath kind-test model does not retain their typed arguments.
Literal `script` and dynamic literal `style` bodies remain rejected. Static literal
styles contain stylesheet text/CDATA/entities, without nested XSLT instructions.
Unlisted instruction attributes remain source-located rejections.

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
Select-based `xsl:try`/`xsl:catch` retains native results. A construction error is
caught only when its enclosing element/document is constructed inside the try;
returning a pending map or late attribute from a try does not construct the parent
there. The parent reports the error in its own recovery scope.

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

## Viewer selection and CSV parity gate

The 2026-09-19 viewer prerequisite probes in
[`xslt_viewer_selection_boundary.rs`](../packages/cem_ql/tests/xslt_viewer_selection_boundary.rs)
identified two shared contracts, now implemented after approval. The viewer
stylesheet remains open; these fixtures establish its prerequisites.

| Probe | Current result |
| --- | --- |
| CEMT `data:read` over XML/JSON/YAML/CSV | Row `.id` values survive fresh reads of identical source; changed source invalidates them. `.line` identifies the original source line. |
| XPath string imports followed by sorting | Native rows, owners, source maps and original line numbers survive sorting. Separate parses create distinct XDM documents and pointer-based identities. |
| `Q{urn:cem:source}node-key` / `line-number` | Shared native access now survives binary reload and native/WASM execution. CEM-QL counterparts return the same values for the same retained nodes. |
| CSV `label\nB\nA` | The one-argument XPath profile still returns three array rows. Explicit `map {'header':'present'}` returns two object rows with named fields, matching CEMT's reader. |

Persisting the current XPath native identity into a selection slice would lose
selection on the next parse. Using sorted row position would select a different
row after sorting. Source line numbers alone also cannot distinguish two rows
on the same line. Existing metadata must remain attached to native nodes; no
document-object projection or source serialization can repair this boundary.

Extension **approved and implemented on 2026-09-19**:

- Format-neutral provenance access to the retained CEM tree is exposed as
  `Q{urn:cem:source}node-key($node)` and `line-number($node)` in XPath, with
  matching native CEM-QL `data:node_key(node)` / `data:line_number(node)` functions.
  Each accepts zero or one native node; empty input returns the empty sequence,
  other types/cardinalities fail explicitly. The key is an opaque string; the
  line is a one-based integer, or empty when original location is unavailable.
- The source-key fingerprint is computed once at the shared tree/import boundary
  from source identity, content and import/projection profile, then combined
  with the canonical node ID. Identical input under the same profile retains
  keys across rerenders and native/WASM execution; changed content or profile
  invalidates them. Keys are versioned source-selection tokens, not XDM identity
  or persistent edit tracking. Independently parsed trees remain distinct for
  XPath `is` and document ordering. Existing CEMT `.id` values are unchanged.
  For trees without an imported-source fingerprint, return no source key.
- `import:parse-csv($text, map {'header': 'present'})` uses an explicit
  option owned by CEM-ML string import. It accepts `present` and `absent`, keeps
  the one-argument header-absent behavior, and rejects invalid option names,
  values or cardinalities with `Q{urn:cem:import}invalid-options`. A non-map
  options argument raises `XPTY0004`. The viewer will select `present`, matching
  the existing
  `data:read($text, 'csv')` contract. CEM-QL already supports explicit header
  selection through `data:read($text, 'text/csv;header=present')` / `absent`.
  CSV record/header mapping stays exclusively in import.
- Tests cover distinct keys for same-line nodes, sort/selection across repeated
  parsing, edit invalidation, projection options and binary reload. All four
  formats and both query languages retain native ownership and obey existing
  cancellation/resource limits. JSON projection now retains parser line ranges
  directly; CSV/YAML document roots retain source-map origins, so whole parsed
  documents can cross an XSLT variable boundary.

The key format is opaque and versioned (`cem-source:1:…`). The fingerprint uses
length-framed source identity, original input and mapping-profile fields with
BLAKE3. Standard string imports include the parsing call's source identity and
base URI; distinct call sites/profiles are not interchangeable selection scopes.
Byte and string CEM data-reader imports agree for the same bytes and profile.
Parser-only lifecycle/compatibility imports cannot invent the missing original
input fingerprint, and return empty keys. Their known original locations remain
available. New formats must supply provenance in import, not in query adapters.

Verification: 450 CEM-QL tests, 32 focused CEM-ML integration tests (including
the import-boundary audit), and 159 native/WASM compiled-bundle checks pass.
CEM-ML and CEM-QL Nx lint pass with existing warnings. The bundle checks
compare native/WASM compiled bytes and source keys, original lines, sorted
selection, edit invalidation and typed CSV option recovery for all four formats.

Native named-entry parameters already support the viewer's scalar controls.
This gate does not approve a new browser binding or claim browser interactions
are complete; those remain in XSLT-VIEW-DEMO/VERIFY. After viewer completion,
the user requested an audit and implementation of missing CEM-QL equivalents
for the XPath functionality added during this work (XPATH-CEMQL-PARITY-AUDIT).
After those gaps are implemented, the XPath functions demo will pair its
examples with equivalent CEM-QL samples and links to detailed function use
cases (XPATH-CEMQL-DEMO-PAIRS).

## Namespace-declaration parity

The original viewer probe used two rows that each declare `xmlns:p="urn:rows"`.
The CEMT viewer previously emitted a column headed
`@http://www.w3.org/2000/xmlns/|p`, alongside `@id`. Its reader intentionally
exposes source-oriented attribute nodes. Native XPath `@*` exposes only `id`;
the namespace declarations are correctly excluded from its attribute axis.
`cemt_namespace_declarations_are_not_attributes_in_the_xpath_data_model` verifies
both the actual CEMT output and the native XPath selection.

**Approved and implemented on 2026-09-19:** exclude XMLNS declarations from the
CEMT viewer's column list, row cells and tree attribute details. The XSLT viewer
uses semantic `@*`. Generic CEM-QL `.attributes` and native XPath semantics are
unchanged. Tests cover default and prefixed declarations, grouping by expanded
name and ordinary namespaced attributes whose local name happens to be `xmlns`.
The source editor still displays the original declarations.

## Native base viewer and compiler reductions

[`data-table-view.xslt`](../packages/cem-elements/demo/data-table-view.xslt) is a
working native/CLI base viewer with the named entrypoint `viewer`. Its declared
scalar parameters are `initial`, `source`, `format`, `column`, `direction`,
`mode` and `selected`. XML/JSON/CSV/YAML source strings enter CEM-ML import; all
navigation and presentation use retained CEM nodes. Presentation maps keep
native source nodes and calculated cells, rather than replacing the input tree.

[`xslt_data_view.rs`](../packages/cem_ql/tests/xslt_data_view.rs) compares semantic
rendered structure with CEMT for namespaces, later columns, nested collections,
empty/missing/null values and inert processing instructions. Comparison ignores
formatting whitespace, per-language opaque selection keys, and empty versus
absent select `value` attributes. Separate checks verify source keys/lines,
selection after sorting, invalidation on edits and recoverable parse errors.
Text/number sorting preserves stable ties and places missing, nonnumeric and
nonfinite numeric keys last, matching CEMT, including exponent notation.

The viewer exposed unnecessary compiler overhead under the existing 128-item
CEMT expression budget. Each XPath slot now receives only referenced outer
variables; a typed visitor respects inline-function and local binding scopes.
Ordinary sort keys with matching static contexts run in native population
focus as one typed XPath program, preserving per-key source locations and
separate array members. Cardinality, conversion and comparison remain bounded
native operations. Group-specific or distinct static contexts keep their
individual invocation path. No shared evaluator or budget limits changed.
The 50-row unused-binding regression and nine-row sorting parity case pass.

Verification: 457 CEM-QL tests, 13 CLI parity tests and 172 native/WASM bundle
checks pass. The CLI executes the actual stylesheet for all four formats;
WASM reloads its native-produced named-entry bundle for twelve source/sort/
selection cases and matches native compilation under default options. Native
and browser namespace fixtures pass. Full browser viewer parity, styling and
imported XSLT presentation aspects remain open.

## Browser state-binding decision

**Explicit scalar parameter mappings approved on 2026-09-19.**
`cem-elements` currently routes XSLT URLs through
`ensureLegacyConverted` and `convertLegacyTemplate`, then compiles the converted
CEMT source. It does not retain the typed XSLT bundle. The public WASM
`compileXsltBundle` / `retainXsltStylesheet` functions use default compiler
options; native-produced bundles can already declare an entrypoint and scalar
bindings and render through the tested import/render/dispose API.

The accepted contract is **explicit scalar parameter bindings** for a
strict XSLT component declaration. Reuse CEM-ML/CEM-QL expressions for state
selection, map them to declared XSLT parameters, and retain the compiled bundle
through the declaration/worker lifecycle. For this viewer the mapping is:

| XSLT parameter | Existing component input |
| --- | --- |
| `initial` | Trimmed joined payload source text |
| `source` | Source slice, falling back to `initial` |
| `format` | Declared format attribute |
| `column` | Column slice, default empty string |
| `direction` | Direction slice, default `ascending` |
| `mode` | Comparison slice, default `text` |
| `selected` | Selection slice, default empty string |

This keeps source text a scalar until CEM-ML imports it and keeps document
handles native. It requires explicit declaration metadata, WASM compiler-option
transport and worker retention/disposal; it must not silently infer parameter
bindings, use the legacy converter for strict XSLT, or expose document records.
The alternative native `datadom` document was not selected. Compiled-bundle
delivery, imports, provenance and scalar state selection are settled.

The native `xslt::component::XsltComponent` adapter now compiles each declared
CEM-QL `select` expression once and resolves parameter/entrypoint names in the
principal stylesheet's namespace context. Its control options carry
`entrypoint`, `parameters: [{name, select}]` and the explicit, content-hashed
stylesheet module closure. Expressions see only the declared host bindings.
Each mapping is independent; a parameter name is not another selector's local
variable. Zero items supply an empty sequence; an unmapped parameter keeps its
XSLT default. Multiple items, nodes, arrays, records and functions are rejected
before rendering. Native document properties may produce scalar values.

WASM exposes `retainXsltComponent`, `renderXsltComponent`,
`disposeXsltComponent` and `xsltStylesheetImports` for this adapter. The
stylesheet-import query returns only authored dependency specifiers from typed
XSLT authoring input. Documents enter through the existing CEM import/handle
channel. No external-format reader or JavaScript document projection was added.
The adapter currently requires an explicit native initial context for named
templates that evaluate XPath, as does the existing lowered bundle. Browser
declaration syntax, resolver preflight and worker integration remain unshipped
pending the focus choice below.

### Named-entry initial focus decision

The scalar mapping fixture exposed a separate existing limitation. Even a
named template that reads only `$text` fails without a source context item:
`cem.xslt.bundle_argument: XSLT context requires exactly one native item`.
The compiler emits sequence-focus XPath programs throughout the named body;
the existing bundle ABI requires a singleton item for those programs. Its
static `absent` focus form does not represent a named call whose focus may be
absent initially and become present inside an iteration.

The native test `characterizes_the_named_entry_absent_focus_gate` and its WASM
counterpart reproduce this without a synthetic document. Supplying an explicit
retained CEM document makes the same scalar mappings work. This characterizes
the current limitation; it is not a claim that the browser contract is complete.

**Recommended next change:** extend typed XSLT lowering and its bundle focus
contract to preserve an absent initial focus for named-template invocation.
This follows [XSLT 3.0 call-template invocation](https://www.w3.org/TR/xslt-30/#invoking-initial-template):
the initial context item is optional. Scalar-only expressions should succeed;
expressions reading missing focus should raise their standard dynamic errors
when accessed. Named calls must preserve absence, and iteration/application of
templates must establish ordinary native item/position/size focus. Use an
explicitly identified optional-focus ABI form, preserving existing strict
singleton/sequence bundle validation, ownership and limits. Verify native,
binary reload and WASM behavior, including typed recovery and empty iterations.
Do not invent an XML control document or route state through JSON records.

The alternative is to require a separately declared retained CEM source-document
binding for every browser XSLT declaration, in addition to scalar parameters.
This is a new initial-context contract, not a reconsideration of scalar state
selection. **Awaiting this choice** per the user's stop-at-decisions instruction
and the browser-runtime scope gate in [the viewer plan](xslt-data-table-parity.md#scope).

## Native output construction

The shared extension was approved on 2026-09-18. XSLT lowering selects explicit
`result-sequence`, `result-element`, `result-attribute` and `result-document`
instructions. Their typed portable IR is distinct from literal CEMT elements;
older readers reject the unknown instruction form. Ordinary CEMT interpolation,
late-attribute handling and component stylesheet declarations retain their
existing behavior, covered by the original compatibility probes.

Pending values remain distinct from constructed text through internal named
calls, XSLT imports, loops and buffered recovery. An enclosing native result
constructor consumes them using [complex-content construction](https://www.w3.org/TR/xslt-30/#constructing-complex-content):
arrays flatten recursively, atomic values use shared XPath string conversion,
adjacent atomics receive spaces, documents contribute children, empty text nodes
are removed and adjacent text nodes merge. An empty document still separates
atomic runs. Attribute simple content instead atomizes nodes and joins values,
after merging adjacent text. Maps/functions fail explicitly. XSLT supplies
standard names such as `XTDE0410` (late attribute), `XTDE0420` (attribute in a
document), `XTDE0450` (function/map in complex content), and `FOTY0013`
(unatomizable simple content). Duplicate attributes use the last value by
expanded name. Error URI, offset, line and column identify the constructing
stylesheet instruction; imported nodes retain their original source frames.

Copying uses the common CEM semantic node view: document, element, attribute,
text, comment and processing-instruction nodes. It never traverses an external
format AST, serializes an input tree or reparses output. XML, JSON, YAML and CSV
imports all use this same path. JSON-to-XML is an import projection into CEM
nodes, not an intermediate XML string or JavaScript document object.

Namespace fixup preserves expanded element/attribute names, assigns prefixes
when required and resets inherited default namespaces for unqualified children.
The bounded untyped profile preserves names rather than original prefix
spelling or unused in-scope namespace declarations; namespace-node constructors,
namespace-axis copying and schema-typed QName content are not supported. Native
render-plan names carry explicit lexical QName metadata and namespace URIs;
legacy CEMT namespace fields retain their prior contract. CLI CEM-tree export
and WASM render-plan serialization consume that metadata directly. WASM emits
attribute `namespaceUri` as explicit render protocol metadata. This checkpoint
verifies native/CLI/WASM output; browser viewer integration remains a later gate.

Native construction polls host cancellation and has cumulative limits of 1 MiB
of copied/constructed text, 100,000 construction work steps and depth 128.
Buffer production is charged before pending results accumulate. The existing
XPath/evaluator limits and 32-template-call limit remain in force. Rollback does
not replenish budgets, and catches cannot suppress limits or cancellation.
Pending values outside a result constructor fail explicitly; independent external
CEMT transform calls still return completed render plans.

Static literal result styles bypass component declaration extraction and remain
in their runtime branch. HTML serialization retains their raw text; the existing
CLI output boundary supplies linked CSS, inline and omit policies, CSS exports
and source maps. The restored CLI fixture is active, alongside rejection of
dynamic literal styles with no output files or sidecars.

Acceptance is covered by `native_result_construction.rs`, `xslt_output.rs`, the
ordinary CEMT compatibility probes, the CLI style export fixture and native/WASM
bundle checks. Verification totals are recorded with **XSLT-VIEW-OUTPUT** in
[todo.md](todo.md). Full viewer parity remains the next slice.

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

The CLI style-export fixture is active under XSLT-VIEW-OUTPUT. Unsupported
dynamic literal styles still fail before any output or sidecars are emitted.
