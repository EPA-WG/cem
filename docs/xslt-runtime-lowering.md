# Typed XSLT runtime lowering

`cem_ql::xslt::compiler::compile_xslt_bundle(source, source_uri)` compiles a
bounded XSLT 3.0 stylesheet into the [portable XSLT bundle](xslt-bundle.md).
It returns binary bytes, their content hash, the stylesheet source hash and
inspectable generated CEMT. Compilation does not accept runtime documents.
The same bundle evaluates changed retained CEM documents on every render.

This is the XSLT-VIEW-LOWER foundation, not the completed data-table viewer or
full XSLT 3.0 support. The existing transform/CLI adapter still uses its older
conversion path; replacing that path requires the template dispatch, parameter
and import contracts in XSLT-VIEW-MATCH. There is no automatic fallback from
this compiler to that path. Browser component URL loading and the viewer
remain later integration fixtures in [todo.md](todo.md).

## Adapter migration decision (pending)

The XSLT-VIEW-MATCH audit found a public compatibility boundary before adapter
replacement. The existing `XsltParityTransformTemplateAdapter` and its CLI
fixtures accept XSLT 1.0 and legacy namespace shortcuts. This compiler accepts
only the documented namespace-correct XSLT 3.0 profile. Replacing the registered
adapter therefore changes accepted source, even for a stylesheet containing
only literal output.

The native characterization fixture
[`xslt_adapter_migration_boundary.rs`](../packages/cem_ml_transform_cem_ql/tests/xslt_adapter_migration_boundary.rs)
compares both public compile APIs. It checks namespace-correct version 1.0,
version 3.0 with an undeclared `xsl` prefix, and a namespace-correct version 3.0
positive case. Existing consumer evidence includes the adapter's
`xslt_parity_adapter_*` unit tests and
[`xslt_parity_transform.rs`](../packages/cem_ml_cli/tests/xslt_parity_transform.rs).
Those consumers also exercise named entrypoints, implicit parameter shortcuts,
EXSLT syntax and stylesheet output; their migration must preserve the intended
results through supported standard declarations or explicit unsupported errors.

The user must choose between these public migration contracts before replacing
the adapter:

1. **Recommended: retire the legacy execution adapter.** Route execution through
   the typed compiler and migrate affected consumers/tests to namespace-correct
   XSLT 3.0 with explicit declarations. Diagnose unsupported legacy authoring;
   do not change versions or inject namespaces silently. This removes accepted
   legacy syntax from that execution route. Separate legacy conversion tools
   are outside this decision.
2. **Keep an explicit compatibility route.** Retain legacy execution behind an
   independently selected adapter/runtime route and expose typed execution
   separately. Keep the typed viewer on the new route with no fallback. This
   preserves compatibility while retaining two execution implementations and
   requires documenting their explicit selection.

Adding standards-compliant XSLT 1.0 compatibility to the typed runtime is a
third, larger scope if requested. It cannot be implemented by changing the
version check: [XSLT 3.0 §3.9](https://www.w3.org/TR/xslt-30/#backwards) defines
version-dependent instruction and XPath behavior. The current legacy converter
is not evidence of that conformance.

The audit confirms existing generic CEMT named calls, mode-based match rules,
native parameter streams and recursion limits as foundations for lowering.
XSLT pattern semantics, import precedence and parameter rules still need their
own compiler work and tests. No shared capability expansion has been identified
or approved by this audit. Runtime code and adapter registration remain
unchanged while this choice is pending; XSLT-VIEW-MATCH remains open.

## Supported authoring profile

The stylesheet must declare version `3.0` and contain exactly one
`xsl:template match="/"`. Its initial context is the native `document` host
binding, with position and size both one.

| Construct | Runtime behavior |
| --- | --- |
| `xsl:for-each select` | Native item sequence, one-based position and sequence size; nested loops restore the outer focus. |
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

Unsupported instructions, attributes, dynamic AVTs and unsupported syntax
fail compilation with stylesheet coordinates. The shared XPath evaluator
reports unsupported functions when evaluated; for example `concat()` is not
currently supported, while the standard `||` operator is. Runtime errors
preserve stylesheet diagnostics and discard the whole partial result through
existing protected CEMT rendering. Cancellation and budget errors remain
uncatchable. No substitute output is manufactured.

Named/matched template dispatch, parameters, imports, grouping, sorting,
standard parsing functions, dynamic output, stylesheet sidecars and full
namespace/output handling belong to the following fixtures. `script` and
`style` literal result elements are rejected until that output profile is
implemented. `xml:space` and other unlisted instruction attributes are also
rejected rather than silently ignored.

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

Compiler limits are 128 KiB of source, 8,192 XML authoring events, 64 nested
source levels, 128 XPath programs and a source URI short enough to append the
generated CEMT identity within the bundle's 4 KiB identifier limit. Existing
XPath, CEMT, bundle and operation limits also apply.
