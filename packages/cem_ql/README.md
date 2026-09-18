# cem-ql

`cem-ql` is the Rust compiler and evaluator for the CEM query language used by
schema behaviors, CEMT templates, validation, component bindings, and explicit
standalone query execution.

## Public boundary

The crate owns CEM-QL lexing, parsing, name resolution, type checking, IR,
standard-library modules, evaluation budgets, compiled artifacts, templates,
transport, and the WebAssembly boundary. It consumes CEM-ML data and projection
owners without moving general parser or lifecycle semantics out of `cem-ml`.

The public expression identity is
`application/vnd.cem.query-expression+cem-ql` with schema
`https://cem.dev/ns/query/cem-ql/1#expression`. CEM-native transform rendering
is integrated through the separate
[`cem-ml-transform-cem-ql`](../cem_ml_transform_cem_ql/README.md) adapter crate.

The explicit `cem_ql::xslt` host integrates XSLT-owned deployment bundles:
generated CEMT, independently encoded XPath programs, and their stylesheet
closure. Bundles have local capabilities and bounded retained handles, with
WASM `importXsltBundle`, `renderXsltBundle` and `disposeXsltBundle` entry
points. The [bundle contract](../../docs/xslt-bundle.md) defines source
ownership, focus/variable arguments and retained CEM document bindings.
The [typed stylesheet compiler](../../docs/xslt-runtime-lowering.md) supports
recursive named/matched templates, explicit parameters, modes, import/include
precedence, runtime loops, scoped variables, XPath conditionals and simple text
construction. The transform/CLI adapter uses this strict XSLT 3.0 path. WASM
`compileXsltBundle` / `retainXsltStylesheet` expose default compiler options;
native-built module closures load through the explicit bundle API. Grouping,
sorting, the full output profile and the complete viewer remain pending.

## String helpers

Tier A `str:` functions include literal `split(value, separator)`,
`trim`/`trim_start`/`trim_end`, `char_at`/`at`, and `index_of`/`last_index_of`.
They complement the existing case conversion, substring, containment,
replacement, and joining functions. Indices use Unicode codepoints; split
returns a sequence and preserves empty fields. For example, a whitespace word
count that retains repeated words and treats blank input as zero is:

```cem-ql
seq:count(seq:where(
    str:split(str:normalize_space(text), " "),
    fn(word) => word != ""
))
```

See the [string-function contracts](../../docs/cem-ql-stack-design-impl.md#112-cemstdlibstrings)
for signatures, edge cases, and differences from JavaScript.

## Native data import and presentation dispatch

Tier B `data:read(source, format, projection?)` accepts XML, CSV, YAML or JSON (short
names or their MIME types) through existing native parsers. Its report exposes
`error` and `root`; failures expose no partial root. The declarative equivalent
binds the same native owner without emitting a DOM element:

```cem-ml
{cem-data @name=document @select=source @type="{$format}"}
{apply-templates @select=document.root.children @mode=inspect}
```

The typed CEM AST exposes `kind`, `name`, `namespace`, `attributes`,
`children`, `descendants`, `value`, source-versioned `id`, and `line`.
Children/attributes are sequences; native source maps remain attached.
XML keeps expanded names and ordered text, comments, CDATA and processing
instructions. JSON/YAML/CSV use the `cem:generic-data` namespace with
`object`, `array`, `property @name`, and typed scalar elements
(`string`, `number`, `boolean`, `null`). CSV uses its first row as headings
and retains string-valued fields. This is not a JSON AST handoff.

The optional projection defaults to `"cem"`, preserving those existing shapes.
For JSON input, `"json-to-xml"` selects the native `cem-ml` projection using the
[standard JSON-to-XML structure](https://www.w3.org/TR/xpath-functions-31/#json-to-xml-mapping):
`map`, `array`, `string`, `number`, `boolean`, and `null` elements in
`http://www.w3.org/2005/xpath-functions`, with unnamespaced `key` attributes on
object member values. There are no intervening `property` nodes. Null array
entries remain elements and empty strings have no zero-length text node.

```cem-ml
{cem-data @name=document @select=source @type=json @projection=json-to-xml}
```

`projection` also accepts a whole attribute-value expression. Unsupported
projections or non-JSON input with `json-to-xml` return an error with no root.
The reader retains the JSON owner and source maps; projection participates in
node identity. No XML string is constructed or reparsed, and CEM-ML remains
the default structural presentation through the existing typed tree writer.

The query profile uses untyped nodes, retains duplicate keys and source order,
preserves number spelling, and replaces XML-invalid scalar characters with
U+FFFD (`escape=false`). The underlying native Rust API
`cem_ml::validation::json_xml::project_json_to_xml` additionally offers
`JsonXmlProjectionOptions` for escape marking, retain/use-first/reject duplicate
policies, and depth/value limits. This is a structural projection of the JSON
parser's accepted input, not a claim that the XSLT standard function is already
implemented. Schema-typed output, custom fallback functions, liberal JSON and
unpaired-surrogate input are not added by this projection.

Import limits: 32 KiB, depth 64, 4096 events/values. DTDs, unresolved XML
entities, YAML anchors/aliases/explicit tags and complex keys are rejected.
Native parser owners remain available; the query projection is not a lexical
round-trip export. Scalar YAML mapping keys become property-name strings.
Generic evaluator budgets and template recursion limits also apply.

Imported CEM document roots and semantic nodes from XML, JSON, YAML and CSV can
be passed to named XPath functions with `@type=any`. This includes both JSON projections.
The common XPath view preserves node identity, source owners and source maps;
import performs all format-specific decoding. XPath coalesces text-like CEM
nodes without changing the source-oriented `children`/`attributes` fields.
Source-only nodes omitted from the semantic view, such as XML declarations and
namespace attributes, are rejected at this binding boundary.
See the [external-data import principle](../../docs/cem-data-import-principle.md).

For compatibility, XML input also accepts `"xpath"`, which selects an opaque
XPath wrapper over the same semantic CEM tree:

```cem-ml
{cem-data @name=document @select=source @type=xml @projection=xpath}
{cem:for-each @as=item @select='native:call("demo.items", document.root)' |
    {p | {$item}}}
```

The query equivalent is `data:read(source, "xml", "xpath")`. The report retains
`error` and `root`; its root is an `XPathQueryItem` over the retained CEM tree,
which keeps the original XML parser owner, source maps and node identity. Pass it to an explicitly installed
XPath function library. This view does not expose the CEM-tree `children` /
`attributes` fields; XPath functions perform node selection. Other source
formats fail with an error report and no root. The default `"cem"` view and
`"json-to-xml"` projection keep their existing behavior.

`EvaluationContext.data_readers` and `TemplateData.data_readers` retain a bounded
LRU of 16 successful imports across all formats, keyed by exact source, format
and projection. Reusing or
cloning the cache preserves owners across evaluations/renders; changed source
gets a distinct owner. The existing per-input limits apply. Errors are not
cached. Clear or drop the cache to release its references; returned nodes keep
their owners alive independently. WASM retains this cache with each compiled or
imported template and releases it through `disposeTemplate`. Native reader
owners never cross JSON, and retention is not serialized in portable artifacts.

Two reusable Tier B collection operations preserve original items and native
identities:

- `seq:group_by(items, keyFn)`: first-occurrence groups with `key` and
  `items`; keys are empty or one atomic value.
- `seq:sorted(items, keyFn, direction?, mode?)`: stable sorting, default
  `ascending` and `text`; `descending` and finite `number` comparison are
  supported. Missing/nonnumeric keys stay last in either direction.

CEMT `template @match='predicate' @mode=inspect @priority=10` declares a
presentation rule. `apply-templates @select=items @mode=inspect` binds each
native item as `node`, forwards `@with:...` parameters, and runs the first
matching rule. Priority is a static signed integer (default 0); higher wins,
then local over imported, then later declaration. Modes are static on rules.
Unmatched values emit nothing. Parameter scopes are restored and recursion is
bounded. Imported match rules participate without copying their bodies into
the consuming module.

The [five-case demo](../cem-elements/demo/data-table.html) authors repeated-row
discovery, headings, cells, tree/table rendering and sort-key selection in
[CEMT](../cem-elements/demo/data-table-view.cemt). There is no Rust table-view
API. An [imported extension](../cem-elements/demo/data-table-aspects.cemt)
changes notes to a tree and an IP-filter record to a local preview form.
These are explicit lossy UI views, not the default typed CEM-tree writer.

## Native query capabilities

Tier B `native:call(identifier, ...arguments)` invokes only functions explicitly
supplied by the Rust host in a `NativeFunctionRegistry`. Identifiers are exact
strings, paired with an arity (0–254 arguments); duplicate registrations are
rejected. Missing capabilities or arities fail with
`cem.ql.native_function_unavailable`, not an empty result or a recoverable data
error. Import aliases for `cem:stdlib/native` work like other stdlib aliases.

```cem-ql
native:call("urn:my-host:select:v1", input, ())
```

Register an implementation of `NativeQueryFunction`, then supply the registry
through `EvaluationContext.native_functions`,
`StandaloneExpressionContext.native_functions`, or
`TemplateData.native_functions`. It is a runtime capability, not a data binding.
All default contexts have an empty registry, including the JSON/WASM boundary;
JSON input cannot install callbacks. Template/query artifacts contain the call
and its arguments only. Hosts must supply capabilities again after artifact
reload. No callback, source-format AST or executable implementation is serialized.

Each callback receives one native item sequence per argument, preserving empty
and multi-item arguments, node identity and source maps. Failed arguments stop
invocation; the evaluator retains argument diagnostics. `NativeQueryRequest`
provides the current item/query scope, call-site source map, module-resolution
capability, active operation control/scope and remaining result-item budget.
Native implementations must bound their own work/allocations and cooperatively
poll that control. The evaluator checks control before/after calls, charges
function-call and result-item budgets, and rejects failed/over-budget results.
Callbacks can use `request.raise(code, message)` for source-mapped domain errors
handled by query/CEMT recovery. Cancellation and budget failures remain
uncatchable. Named/imported templates and native template-call handlers retain
the registry, while data-DOM construction sees only ordinary bindings.

The hook contains no XPath, XSLT, parser or presentation behavior. A host can
retain a typed XPath AST in its callback and invoke the XPath layer directly.
See the [native call tests](tests/native_functions.rs) and
[XSLT-owned XPath integration fixture](../cem_ml_transform_cem_ql/tests/native_xpath_calls.rs).

### Named XPath functions and reusable matching

The opt-in Rust API `xpath::functions::CemtXPathFunctions::compile(source, uri)`
compiles public XPath-backed functions from an existing CEMT module. For example:

```cem-ml
@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module |
    {function @name=demo.accept @visibility=public @returns=boolean |
        {param @name=candidate @type=any @required=true}
        {param @name=minimum @type=integer @required=true}
        {body | {xpath @context=candidate @sequence-type="xs:boolean" |
            {variable @binding=minimum @local-name=minimum}
            {expression | exists(self::item[@qty >= $minimum])}
        }}
    }
}
```

The host calls `library.install(&mut registry, resolvers, policy)` with owned
`Arc<ResolverRegistry>` and `Arc<ResolverPolicy>`, then supplies that registry to
its query/template context. Installation is atomic on conflicts. No default
registry changes, CEM-QL syntax changes, or global functions are introduced.
Native invocation uses the exact declared name and positional parameter order:

```cem-ql
seq:where(items, fn(candidate) => native:call("demo.accept", candidate, 2))
```

The same predicate works in a CEMT rule:

```cem-ml
{template @mode=inspect @match='native:call("demo.accept", node, 2)' |
    {body | {b | {$node}}}
}
```

Matching returns an actual singleton `xs:boolean`; strings such as `"false"`,
nodes and multi-item results cannot pass this declared contract through
truthiness. Query selection stays in CEM-QL; rule priorities, modes, imports
and rendering stay in CEMT/XSLT. This predicate interface is not an XSLT
match-pattern compiler, and it does not implement regex `fn:matches`.
No new matching DSL is needed for these reusable predicates. If literal XSLT
patterns are exposed later, their compiler should stay in the XSLT layer and
offer candidate-to-boolean evaluation through the same explicit capability.

Use the existing triple-backtick rich-content fence around an XPath `expression`
that contains constructor braces. The compiler excludes the fence delimiters and
keeps the original body coordinates for diagnostics, including artifact reload.

The bounded native binding contract is:

- Required positional scalar parameters: `string`, `boolean`, `integer`,
  `number`; no coercion from strings to numbers. Decimal spelling is retained.
  `@nullable=true` permits an empty scalar sequence, not JSON null or an omitted
  argument. Defaults, optional parameters and object/array/JSON types fail closed.
- `any` accepts retained `XPathQueryItem` values and native imported CEM nodes.
  Imported CEM nodes adapt through the shared tree capability. Native hosts can
  also wrap an existing `XPathNativeNode` with `XPathQueryItem::from_node`.
  Returned maps and arrays stay opaque retained items, including nested and
  empty member sequences; CEM-QL does not flatten them or infer record fields.
  Returned nodes keep their retained tree, original source owner and source maps.
  Records and source strings do not become nodes by shape inference.
  The explicit XML `"xpath"` reader remains a compatibility wrapper.
- Both `@returns` and XPath `@sequence-type` are checked. The latter accepts
  `empty-sequence()`, `item()`, `node()`, `map(*)`, `array(*)` and the basic `xs:string`, `xs:boolean`,
  `xs:integer`, `xs:decimal`, `xs:float`, `xs:double`, `xs:anyURI`,
  `xs:untypedAtomic` types, with `?`, `*`, `+` occurrences. This is a closed host
  result contract, not full XPath schema typing; general function items
  cannot cross this boundary. Inline functions can execute inside XPath for
  operations such as `fn:sort`, retaining lexical values and native nodes;
  functions nested in returned maps/arrays are rejected as well.
- Compilation limits source and URI to 32 KiB each and XPath bodies to 64.
  Only public XPath functions are installed; imports require a future resolved
  companion closure. Calls share operation control and sequence/call budgets
  with CEM-QL. XPath defaults also bound each intermediate/final string or
  sequence to 1 MiB of UTF-8 atomic lexical bytes and each invocation to
  16,777,216 work units. Nested expressions share the work counter. These are
  lexical text/work bounds, not total heap accounting; native source owners remain
  retained and text extraction is bounded on atomization. Hosts can use
  `install_with_limits` to choose `XPathEvaluationLimits`; library source and
  portable companions cannot override the host. Unsupported capabilities,
  the XPath inline limits (32 calls and 32 active expression frames),
  cancellation and item/text/work limits remain uncatchable; failed calls
  return no partial sequence.

Compilation reloads each independently identified XPath artifact into a retained
typed program without source text/tokens; render-time calls do not reparse it.
Query/template artifact reload still requires explicit function installation.
The explicit companion APIs below provide binary/source loading in WASM.
The component loader supports an explicit external `xpath-functions` library
reference for declared scalars and explicit XPath reader nodes; see the
[browser contract](../cem-elements/README.md).
Direct QName call syntax, module-URL capability forwarding and the XSLT viewer
bundle remain separate work.
See the [named-function fixtures](tests/xpath_named_functions.rs) for executable
compile-once/render-many, filtering/matching, typing and isolation examples.

### XPath function companions

`CemtXPathFunctions::to_companion_bytes()` exports a versioned CEMT binding
manifest and opaque XPath artifacts. `from_companion_bytes(bytes,
expected_content_hash, expected_source_hash)` validates and reloads the library
without source parsing or installing callbacks. XPath programs retain their
own namespace and `application/vnd.cem.xpath-artifact+cem-bin` identity; the
container is `application/vnd.cem.cemt-xpath-functions+cem-bin`, version
`cemt-xpath-functions/1`. It is not the XSLT viewer bundle.

The length-delimited container uses an explicit JSON **control manifest** for
function names, parameter/variable bindings, source-map metadata and hashes.
Executable programs stay in opaque XPath-owned binary blocks. Runtime data
ASTs and XPath syntax trees never pass through JSON. Reload checks compiler
versions, source/host identity, exact function ownership, variable declarations,
types, source maps, hashes and framing. Trusted expected hashes provide integrity,
not publisher authentication.

Limits are 4 MiB per companion, 128 KiB of manifest metadata, 64 functions,
254 parameters/variable bindings per function, and 64 source-map frames/ranges.
Each enclosed program also obeys the XPath artifact limits. The explicit
`CemtXPathCompanions` host retains at most 64 companions / 16 MiB of encoded
companions. Handles are never reused. Disposal prevents future handle lookup;
it does not revoke registries already cloned by a native caller.

The combined WASM module exposes:

- `compileCemtXPathFunctions(source, sourceUri)`: produce companion bytes.
- `retainCemtXPathFunctions(source, sourceUri)`: compile, validate and retain;
  return JSON control metadata including `companionId` and compiler-generated
  `contentHash` / `sourceHash`. No program bytes enter that JSON response.
- `importCemtXPathFunctions(bytes, contentHash, sourceHash)`: validate binary
  bytes against trusted manifest hashes and return the same control metadata.
- `renderTemplateWithXPathFunctions(templateId, companionId, dataJson)`:
  render an existing template using only the selected companion's functions.
- `disposeCemtXPathFunctions(companionId)`: release the companion handle.

For example, an explicit host can retain once and render repeatedly:

```js
const { companionId } = JSON.parse(retainCemtXPathFunctions(functionSource, functionUri));
try {
    const plan = JSON.parse(renderTemplateWithXPathFunctions(
        templateId, companionId, JSON.stringify({ text: '🍇', quantity: 2 })
    ));
    // The host consumes the ordinary render plan.
} finally {
    disposeCemtXPathFunctions(companionId);
}
```

Template and companion lifecycles are independent. Importing a companion or
rendering with it never changes ordinary `renderTemplate` behavior. JSON render
data cannot install functions or synthesize native XPath nodes. An authored
`cem-data @projection=xpath` reader explicitly parses XML source within Rust and
reuses retained owners on subsequent renders. The WASM fixture covers scalars,
XML selection/matching, reader errors, namespace isolation and artifact reload.
The component loader retains an explicitly referenced external
library through its worker/fallback host. The [live demo](../cem-elements/demo/xpath-functions.html)
uses string functions, XML reader nodes and boolean CEMT predicates across changed slices.

Verification: [native companion fixtures](tests/xpath_function_companion.rs) and
`node tools/scripts/verify-xpath-function-companions.mjs` after the Nx WASM build.

## Error recovery

Tier B queries support `try { expression } catch (code, message) { expression }`
and `report:raise(code, message)`. Both raise arguments must be single strings;
the code must be nonempty. Catch bindings are lexical and visible only in the
handler. For example:

```cem-ql
try { report:raise("sample.invalid", "Check the source") }
catch (code, message) { {code: code, message: message} }
```

Successful results (including empty sequences and native node references) pass
through unchanged. Raised errors and runtime type errors stop the protected
evaluation; partial values and the handled failure's diagnostics are discarded.
Handler failures propagate to an enclosing catch. Independent diagnostic reports
remain reports: `report:emit`, even at fatal severity, does not raise an error.
`data:read` continues to return its existing report rather than raising.
Static compile errors, unsupported engine/policy capabilities, cancellation and
resource-limit failures are not caught; recovery never resets operation budgets.

CEMT provides scoped output recovery using the same failure channel:

```cem-ml
{try |
    {call @template=load-content}
    {catch @as=failure @test='failure.code == "sample.invalid"' |
        {p @role=alert | {$failure.message}}
    }
    {catch @as=failure | {p @role=alert | {$failure.code}}}
}
```

`try` has no attributes and requires one or more direct `catch` children after
its protected content. A catch binds `@as` (default `error`) to a native
source-mapped record with `code` and `message`. Its optional `@test` is a query;
the first matching handler runs. No match propagates the original failure;
predicate/handler failures go outward, never to sibling handlers. Dynamic
constructor and call errors also participate. Template recursion limits remain
uncatchable.

Protected nodes and constructed attributes are committed only on success.
Failures roll back that output and local bindings before recovery. Unhandled
recovery-region failures produce diagnostics and no output plan. Ordinary
templates outside recovery retain their previous diagnostic/partial-output
behavior. This is render-plan recovery, not rollback of external I/O or static
declaration setup.

The native CLI adapter resolves module calls during rendering through
`TemplateCallHandler`, so recovery spans imported calls while retaining typed
parameters and module recursion limits. The same core semantics work in WASM
and precompiled templates. XSLT syntax, standard error-name mapping and standard
parsing-function semantics remain separate compatibility-layer work.

## Verification

Use the cached Nx targets for the native and WASM surfaces:

```bash
yarn nx run cem_ql:lint
yarn nx run cem_ql:test
yarn nx run cem_ql:build:wasm
```

See the [CEM-QL acceptance criteria](../../docs/cem-ql-ac.md),
[stack design](../../docs/cem-ql-stack-design.md), and
[implementation design](../../docs/cem-ql-stack-design-impl.md) for the complete
language and runtime contract.
