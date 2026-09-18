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
completed data-table viewer or full XSLT 3.0 implementation. Grouping, sorting,
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
| `xsl:apply-templates` | Explicit/default child selection with original sequence position/size. One named mode, `#default`, and invocation-only `#current`. |
| Node match patterns | Root, child/attribute paths, descendant separators, namespace-aware names, bare kind tests, targeted processing instructions, predicates and unions. Unlisted pattern syntax is rejected. |
| Rule ordering | Import precedence, exact decimal priority, then declaration order. Union branches retain their individual default priorities. |
| Built-in rules | Elements/documents recurse in the current mode and forward supplied parameters; text/attribute nodes emit their string value; other nodes emit nothing. Atomic/map/array dispatch is outside this profile. |
| `xsl:import` / `xsl:include` | Explicit closed source graph; includes share precedence and later imports override earlier imports. Named calls resolve the winning declaration across the closure. |
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
currently supported, while the standard `||` operator is. XPath evaluation errors
preserve stylesheet coordinates; generic dispatch and missing-required-parameter
failures retain generated CEMT frames. Both discard the whole partial result
through existing protected CEMT rendering. Cancellation and budget errors remain
uncatchable. No substitute output is manufactured.

Grouping, sorting, standard parsing functions, dynamic output, stylesheet
sidecars and full namespace/output handling belong to the following fixtures.
Global variables/parameters, parameter constructors/types/tunnels, multiple
mode tokens, `#all`, `xsl:mode`, `apply-imports` and `next-match` are also outside
this bounded slice. Parameterized `element(name)`, `attribute(name)`,
`document-node(element(...))` and schema type patterns are rejected because
the current shared XPath kind-test model does not retain their typed arguments.
`script` and `style` literal result elements are rejected until that output profile is
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
