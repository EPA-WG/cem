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
