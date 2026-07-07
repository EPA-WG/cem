# CEMT Encoding Proposal

Status: promoted for the core CEMT output producer contract; remaining content
is temporary implementation backlog and worked examples.

This note records the detailed CEMT encoding design, worked examples, and
promotion backlog. The canonical documentation entry points for the active
CEMT output producer contract are:

- [`../schema-packages/cem-transform/v1/README.md`](../schema-packages/cem-transform/v1/README.md)
- [`../schema-packages/README.md`](../schema-packages/README.md)

## Problem

CEMT can transform semantic data into target artifacts, but output serializers
need context-aware encoding that should not be hand-written in template text.
Examples include JSON string escaping, XML attribute escaping, HTML text-state
escaping, CSV field quoting, CSS identifier escaping, YAML scalar style
selection, and CEM binary chunk framing.

Encoding here means syntax/context encoding. It is separate from byte character
encoding such as UTF-8 or UTF-16 and separate from transport content encoding
such as gzip.

## Base Principle

CEMT is the primary transformation mechanism for schema-owned output from the
canonical CEM AST and typed projection artifacts into registered destination
content identities. Once source bytes have been parsed, normalized, validated,
and represented as CEM AST, DOM, events, or another typed projection, the
content-family conversion work belongs in CEMT unless a native producer is
explicitly registered as a paired fallback or fast path.

The output pipeline is layered:

```text
CEM AST / typed projection subject
  -> CEMT transformation
    -> target semantic tree, syntax tree, token stream, or chunk plan
      -> CEMT formatting
        -> formatted CEM output tree or token tree
          -> CEMT semantic color projection
            -> colored CEM output tree or styled token tree
              -> final writer
                -> text, bytes, sealed chunks, or transport-ready artifact
```

Formatting is a CEMT-owned transformation stage. The primary output of a
formatter must be a typed CEM output tree, syntax tree, token tree, or chunk plan
that carries destination identity, ordering, whitespace, line-ending,
canonicalization, style-role, and source-map decisions as structured data.
Formatting must not be hidden inside a final writer or host-side string filter.

Coloring is also a CEMT-owned transformation stage. When the formatted artifact
is a CEM tree, semantic color projection must transform that CEM tree in place or
produce a new colored CEM tree with equivalent structure plus color metadata.
The color stage must not emit final writer text as its primary result for CEM
tree pipelines.

The writer is the last phase. It materializes an already transformed, formatted,
and colored artifact into text, bytes, or chunks. A writer may enforce the
selected low-level syntax and byte boundary rules, but it must not choose the
schema semantics, destination content identity, formatting policy, or color
policy. Those choices are CEMT output-producer decisions validated through
registry metadata. A writer for a CEM tree must receive a formatted tree with
formatter metadata and a colored tree with color metadata; otherwise it must
report a diagnostic rather than silently choosing formatting or color policy.

## Core Decision

CEMT is the primary output producer for schema-owned exports. Output production
includes transformation, encoding, formatting, terminal/HTML color output,
source-map span creation, and final artifact identity. Content-type-specific
encoders, formatters, colorizers, writer primitives, and small transformation
helpers are part of the CEMT stack, not an external post-processing layer.

```text
typed subject
  -> schema-owned CEMT output producer
    -> content-type-specific transform / encode / format / color helpers
      -> formatted and optionally colored CEM output tree, token tree, or chunk plan
        -> final writer
          -> destination content type and schema
```

Native output producers remain necessary for performance, bootstrap, and clarity
for some content types. They are paired implementations, not replacements for
the CEMT contract. Every native producer should have a matching CEMT producer or
a planned CEMT producer, and shared fixtures must cross-check native output
against CEMT output. Differences must be explicit diagnostics or documented
lossiness/canonicalization choices.

The default architecture is therefore:

```text
package.cem serializer edge
  -> CEMT producer (primary)
  -> native producer (paired fallback or fast path)
  -> parity fixtures compare CEMT and native output
```

Rust fallback is allowed when a syntax profile is not yet expressible in CEMT,
when binary framing is required, or when performance requires a native writer.

## CEMT Stack Capabilities

The CEMT output stack should provide:

- encoder functions for context-specific escaping and binary framing;
- formatter functions for indentation, line endings, ordering, wrapping, scalar
  style, namespace declaration placement, and canonical output;
- color functions for semantic style roles, terminal ANSI/SGR output, HTML
  color output, no-color fallbacks, and accessibility-aware palettes;
- writer primitives for tokens, byte streams, sealed chunks, and source-map
  spans;
- schema helpers for target syntax rules, void/empty element policy, raw-text
  modes, namespace repair, identifier validity, and field/header policy;
- diagnostics for unsupported category, unsafe raw output, context mismatch,
  charset mismatch, unsupported color capability, lossy output, and
  native/CEMT parity mismatch.

These capabilities are called from CEMT templates and declared by schema package
metadata. They are not opaque host-side string filters.

## Immediate TODO: CEMT Language Features

### Needed For Formatter And Coloring

- Extend template `call` rendering across imported modules. Runtime body
  `call(name, { ... })` expressions now bind named arguments against target
  function params, apply defaults, validate required/type contracts, and reject
  unknown arguments. Declaration-site `{call @template ... @with:* ...}` now
  validates local targets, import aliases, required/default/type-compatible
  arguments, and reports source-map-aware diagnostics.
- Extend immutable local `let` bindings for computed layout, style, profile,
  namespace, source-map, and node-shape decisions. Literal `@value` and
  expression-backed `@expr`/`@expression` lets now lower into module options;
  remaining work is broader expression coverage, source-map diagnostics, and
  formatter/coloring showcases.
- Add `match` dispatch over CEM tree node kind, name, attribute presence,
  formatter role, color role, and layout mode.
- Extend deterministic sequence operations over children, attributes, slots,
  formatter nodes, color nodes, token streams, and chunk plans. `map(...)`,
  `fold(...)`, and the array accumulator helper `append(...)` now execute in
  CEMT bodies; remaining work is richer helpers for object patching, filtering,
  sorting, flattening, and diagnostics accumulation.
- Extend scoped traversal stack support for ancestor path, indentation depth,
  namespace scope, inherited layout, source-map frames, semantic style role, and
  active color capability. `withStack(...)`, `stackPush(...)`, `stackPop(...)`,
  `stackTop(...)`, `stackDepth(...)`, and `stackPath(...)` now provide bounded
  immutable stack values for CEMT bodies; remaining work is parser/declaration
  diagnostics, source-map policy integration, and formatter/coloring showcases.
- Extend deferred queue support for post-order edits, wrapper insertions, color
  mutations, diagnostics, namespace repairs, and writer-boundary checks that
  must run after child traversal is complete. `defer(...)`, `queuePush(...)`,
  `queueShift(...)`, `queuePeek(...)`, `queueLength(...)`, and
  `drainQueue(...)` now provide bounded immutable FIFO queues for CEMT bodies;
  remaining work is richer typed edit helpers, source-map policy integration,
  and formatter/coloring showcases.
- Extend accumulator helpers beyond array `append(...)` for `formatNodes`,
  `colorNodes`, diagnostics, namespace declarations, output spans, and
  writer-boundary metadata. `appendFormatNode(...)`, `appendColorNode(...)`,
  `appendDiagnostic(...)`, `appendNamespace(...)`, `appendOutputSpan(...)`, and
  `appendWriterBoundary(...)` now append typed metadata arrays on immutable
  object accumulators with role/shape validation. Metadata helpers now default
  missing item source maps from the accumulator CEM tree and stamp the active
  formatter/colorizer transform frame; remaining work is formatter/coloring
  showcases.
- Extend tree patch operations for replacing nodes, wrapping nodes,
  prepending/appending formatter or color nodes, and applying queued edits to a
  formatted CEM tree. Shallow object `merge(...)` and path-based `set(...)` now
  provide immutable object/tree patching in CEMT bodies. `replaceNode(...)`,
  `appendNode(...)`, `prependNode(...)`, `wrapNode(...)`, and
  `applyEdits(...)` now provide node replacement, child-array insertion,
  wrapper insertion, and queued edit replay. The built-in `cem.format-tree` and
  `cem.color-tree` CEMT declarations now use these helpers around their direct
  runtime operations. Queued edits now reject ambiguous value fields, malformed
  paths, null appended/prepended nodes, and non-object wrappers. Wrapper
  insertion now defaults missing wrapper source maps from the wrapped node and
  stamps the active CEMT transform frame; remaining work is formatter/coloring
  showcases.
- Extend first-class diagnostics emitted from formatter/color templates.
  `diagnostic(...)` and `diagnostics(...)` now construct writer-ready diagnostic
  values for unsupported layout, inaccessible color, unsafe raw content,
  missing metadata, and writer-boundary mismatch; remaining work is source-map
  policy integration, diagnostics accumulation helpers, and showcases.
- Document each formatter/coloring feature in the CEMT docs, add Rust unit and
  integration tests for parser/lowering/runtime behavior, and add Storybook
  showcases that demonstrate formatted tree output, colored tree output, and
  final writer output as separate visible stages.

### Sufficient Generic Programming Language Surface

- Typed `param` declarations with required/default/nullable semantics for
  modules, templates, and output functions.
- Named template/function `call` with explicit arguments, lexical parameter
  scope, import-qualified targets, recursion limits, and unknown-call
  diagnostics.
- Immutable `let` bindings and expression evaluation over JSON-compatible CEMT
  values: null, boolean, number, string, array, object, node, token, chunk, and
  diagnostic.
- Conditional control flow through `if` and structural `match`, with exhaustive
  or default branches for deterministic output profiles.
- Sequence primitives: `map`, `filter`, `fold`, `flatMap`, stable `sortBy`,
  `first`, `last`, `isEmpty`, `length`, and `join`.
- Structured update primitives for immutable object/array/node patching instead
  of arbitrary mutation.
- Scoped stack, FIFO queue, and accumulator abstractions with deterministic
  operations and bounded resource limits.
- Error handling through typed diagnostics and explicit fallback branches rather
  than unchecked exceptions.
- Module/import visibility rules, package-qualified names, deterministic
  registry lookup, and parity hooks for native paired implementations.
- Resource limits for recursion depth, queue length, stack depth, output size,
  and traversal budget so templates stay safe in canonical producer paths.
- Document the generic language surface as a compact CEMT language reference,
  add parser/lowering/runtime tests for each feature, add negative diagnostics
  fixtures, and add Storybook showcases for representative formatter, coloring,
  and generic transformation templates.

## Function Call Declaration

Proposed expression-level function:

```text
encode(subject, target, options?) -> encoded-artifact
```

Minimum logical signature:

```text
encode(
  subject: any,
  target: {
    contentType: media-type,
    schema: uri,
    category: encoding-category,
    context?: encoding-context
  },
  options?: {
    mode?: "canonical" | "preserve" | "pretty" | "fragment",
    encoder?: qualified-name,
    formatter?: qualified-name,
    colorizer?: qualified-name,
    profile?: string,
    charset?: "utf-8" | "utf-16" | "utf-16be" | "utf-16le" | "us-ascii" | "other",
    lineEnding?: "lf" | "crlf" | "preserve",
    quote?: "auto" | "single" | "double" | "none",
    indent?: string,
    namespacePolicy?: "preserve" | "repair" | "canonical",
    sourceMap?: "preserve" | "generated" | "none"
  }
)
```

Expected CEMT expression form:

```cemt
{$ encode(
    $node.data,
    {
      contentType: "text/html",
      schema: "https://cem.dev/ns/data/html/1",
      category: "html-text"
    }
) }
```

The return value is not a plain string. It is an encoded artifact carrying:

- produced kind: `text`, `bytes`, `tokens`, or `chunks`;
- target content type and schema URL;
- encoding category and context;
- charset or binary framing identity;
- source-map policy and generated spans;
- a flag that prevents accidental second-pass encoding.

Template insertion must reject an encoded artifact when its target identity or
context is incompatible with the surrounding output context.

## Runtime Function Call Expressions

CEMT output-function bodies can call other CEMT-authored output functions with
named arguments:

```cemt
{$ call(acme.format-node, {
    subject: $subject,
    depth: 0,
    role: "syntax.name"
}) }
```

Runtime calls are lexically scoped. The callee receives the caller bindings plus
arguments bound by declared parameter name. Parameter declarations provide the
argument contract: required params must be supplied, default values are applied
when an argument is omitted, nullable and type metadata are enforced, and
unknown arguments are rejected. Argument expressions are evaluated before the
callee body runs, so unresolved values fail the call rather than silently
falling back to host code.

Calls are deterministic and bounded. Recursive calls are allowed for formatter
and coloring traversal helpers, but execution stops at the configured recursion
limit.

Declaration-site template calls use `{call @template="..." @with:*="..."}`.
Local targets are checked against declared template params: required arguments
must be present unless a default exists, `with:*` values are validated against
the target type, and unknown arguments are rejected. Diagnostics carry the call
node source map and byte offset. Imported calls currently validate the import
alias only; imported target params are validated once module loading is wired
into the call graph.

## Local Let Bindings

CEMT modules and entrypoints can declare immutable local values with either a
typed literal value or an expression over the current lexical bindings:

```cemt
{let @name="layout" @type="string" @value="block"}
{let @name="title" @type="string" @expr="$input.title"}
```

Module-level lets are applied before entrypoint-level lets, and both are
evaluated in declaration order. Expression lets can reference earlier bindings
from the same render context, including prior lets. The resolved value must
satisfy the declared type and nullable contract before it is inserted into the
binding scope; unresolved expressions or type mismatches produce fatal template
diagnostics rather than falling through to writer behavior.

## Sequence Fold And Accumulators

CEMT body expressions can use deterministic `map(...)` and `fold(...)` over JSON
arrays. `map(collection, body)` evaluates `body` once for each item with `$item`
and `$index` in scope. `fold(collection, initial, step)` also provides `$acc`
and `$accumulator`, updating the accumulator with the resolved `step` value on
each iteration:

```cemt
{$ fold($subject.children, [], append($acc, {
  kind: "child",
  slot: $index,
  name: $item.name
})) }
```

`append(array, value)` returns a new array with `value` appended. It is the first
bounded accumulator helper for formatter/coloring templates that need to collect
formatted children, color nodes, writer chunks, or diagnostics before returning
a CEM tree value.

## Typed Metadata Accumulators

CEMT formatter and color templates can collect transformation metadata on object
accumulators without manually expanding `set(... append(...))` chains. Each
helper returns a new object, creates the target metadata array when it is absent,
and appends to the existing array when present:

```cemt
{$ fold($subject.children, { formatNodes: [] }, appendFormatNode($acc, {
  kind: "format-decision",
  name: match($item.kind, { element: $item.name, text: $item.value, default: "unknown" }),
  slot: $index,
  formatterRole: "formatter.child"
})) }
```

The typed helpers are:

- `appendFormatNode(accumulator, node)` for `formatNodes`
- `appendColorNode(accumulator, node)` for `colorNodes`
- `appendDiagnostic(accumulator, diagnostic)` for `diagnostics`
- `appendNamespace(accumulator, declaration)` for `namespaceDeclarations`
- `appendOutputSpan(accumulator, span)` for `outputSpans`
- `appendWriterBoundary(accumulator, metadata)` for `writerBoundaries`

Metadata helpers validate their item shape before appending:

- `appendFormatNode(...)` requires object items with `kind` and
  `formatterRole`.
- `appendColorNode(...)` requires object items with `kind` and
  `colorizerRole`.
- `appendDiagnostic(...)` normalizes the diagnostic object the same way as
  `diagnostic(...)`: it requires `code` and `message`, defaults severity to
  `info`, and accepts optional source/location fields.
- `appendNamespace(...)` requires a namespace `uri` and accepts an optional
  `prefix`.
- `appendOutputSpan(...)` requires `kind`, `start`, and `end`; `end` must be
  greater than or equal to `start`.
- `appendWriterBoundary(...)` requires `kind` and `stage`.

These helpers keep formatter metadata, coloring metadata, diagnostics,
namespace repairs, output spans, and writer-boundary checks in the CEM tree
value before the writer phase. When an appended metadata item omits
`sourceMap`, the helper derives one from the nearest source-mapped CEM tree
value in the accumulator and appends the current formatter/colorizer transform
frame. Explicit `sourceMap` values are preserved.

## Scoped Traversal Stacks

CEMT body expressions can carry traversal context as bounded immutable stack
values. Stack helpers are data helpers, so formatter and color templates can pass
the active stack through normal `call(...)` parameters while the writer remains
only the final serialization phase.

`withStack(name, frame, body)` pushes `frame` onto the named stack binding,
evaluates `body` with that scoped value, and then returns the body result without
mutating the caller binding. Missing named stacks start as empty arrays; existing
stack bindings must be arrays. The stack depth is bounded.

```cemt
{$ withStack(ancestors, {
  name: $subject.name,
  slot: $slot,
  layout: "block"
}, {
  name: $subject.name,
  depth: stackDepth($ancestors),
  ancestorNames: stackPath($ancestors, name),
  currentFrame: stackTop($ancestors),
  children: map($subject.children, call(acme.format-node, {
    subject: $item,
    slot: $index,
    ancestors: $ancestors
  }))
}) }
```

`stackPush(stack, frame)` and `stackPop(stack)` return new stack arrays for
explicit parameter passing. `stackTop(stack)` returns the last frame or `null`
for an empty stack. `stackDepth(stack)` returns the current depth.
`stackPath(stack, path)` projects a dotted object/array path from each frame,
preserving missing frame values as `null`. Formatter and color templates use
these frames for ancestor paths, indentation, namespace scope, inherited layout,
source-map frames, semantic style role, and active color capability.

## Deferred FIFO Queues

CEMT body expressions can collect deferred work as bounded immutable FIFO queues
and drain that work after child traversal. This supports post-order formatter
edits, wrapper insertions, color mutations, diagnostics, namespace repairs, and
writer-boundary checks while keeping those decisions in CEMT rather than the
writer phase.

`defer(queue, item)` appends a deferred work item to the back of a queue.
`queuePush(queue, item)` is the same generic FIFO append operation. Queue values
are JSON arrays and are length-bounded. `queuePeek(queue)` returns the first item
or `null`, `queueShift(queue)` returns `{ item, queue }`, and
`queueLength(queue)` returns the current queue length.

`drainQueue(queue, initial, step)` processes the queue in FIFO order. Each
iteration evaluates `step` with `$item`, `$index`, `$acc`/`$accumulator`, and
`$queue` for the remaining queued items. The returned value becomes the next
accumulator:

```cemt
{$ applyEdits(
  $formattedTree,
  fold($subject.children, [], defer($acc, {
    kind: "set",
    path: $item.colorPath,
    value: $item.colorRole
  }))
) }
```

Formatter and color templates can use this to first produce formatted children
and collect work items, then apply queued changes to the CEM tree before the
writer receives it.

## Immutable Tree Patches

CEMT body expressions can patch JSON-compatible CEM tree values without mutating
the source binding. `merge(object, patch)` returns a shallow object merge, while
`set(value, path, replacement)` returns a new value with a dotted object/array
path replaced:

```cemt
{$ set(
  merge($subject, {
    coloredBy: "acme.color-tree",
    colorNodes: [{ kind: "colorizer", name: "acme.color-tree" }]
  }),
  "nodes.0.style.colorRole",
  "syntax.name"
) }
```

Object fields in a `set(...)` path are created as needed. Array segments must be
numeric and in bounds; appending remains explicit through `append(...)`.

Node-specific helpers cover the formatter and coloring edits that should remain
in CEMT before the writer phase:

- `replaceNode(tree, path, node)` replaces an existing node path.
- `appendNode(tree, path, node)` appends a node to the array at `path`, creating
  a missing object field as an array.
- `prependNode(tree, path, node)` prepends a node to the array at `path`.
- `wrapNode(tree, path, wrapper)` replaces the node at `path` with `wrapper` and
  places the original node in `wrapper.children`.
- `applyEdits(tree, edits)` replays a queue/array of edit objects in order.

Queued edit objects use `kind` or `op`, a string `path`, and a value field. The
supported edit kinds are `set`, `replace`, `append`, `prepend`, and `wrap`.
`set`, `replace`, `append`, and `prepend` read `value`, `node`, or
`replacement`; `wrap` reads `wrapper`, `value`, or `node`:

Each edit must provide exactly one value field. Patch paths cannot contain empty
segments. `append` and `prepend` reject `null` nodes, and `wrap` requires an
object wrapper before the edit is applied.

When `wrapNode(...)` or an `applyEdits(...)` wrap edit creates a wrapper without
`sourceMap`, CEMT derives the wrapper map from the wrapped node and appends the
active transform frame. Formatter bodies therefore produce formatted wrapper
metadata, coloring bodies produce colored wrapper metadata, and explicit wrapper
`sourceMap` values are preserved.

```cemt
{$ applyEdits($formattedTree, [
  { kind: "set", path: "children.0.style.colorRole", value: "syntax.name" },
  { kind: "wrap", path: "children.1", wrapper: {
      kind: "element",
      name: "span",
      colorRole: "syntax.text"
  }},
  { kind: "append", path: "children", node: {
      kind: "color-node",
      name: "trailing-color-metadata",
      colorizerRole: "colorizer.after"
  }}
]) }
```

This lets the formatter build the formatted CEM tree, lets the coloring
transformation change that CEM tree, and leaves final serialization to the
writer after formatting and coloring metadata has already been materialized.

## First-Class Diagnostics

CEMT formatter and color templates can construct diagnostics as data instead of
throwing host-side errors or leaving diagnostics to the writer phase.
`diagnostic({...})` accepts a JSON-compatible object with `code`, `message`, an
optional `severity`, and optional location fields such as `uri`, `node`, `line`,
`column`, `byteOffset`, and `sourceMap`. Missing severity defaults to `info`.
The positional form `diagnostic(code, severity, message)` is available for
compact cases:

```cemt
{$ diagnostics([
  diagnostic({
    code: "cem.format.unsupported_layout",
    severity: "warning",
    message: "inline layout is not supported by this formatter profile",
    node: $subject.name
  }),
  diagnostic("cem.color.inaccessible", "error", "palette contrast failed")
]) }
```

`diagnostics(value)` wraps either one diagnostic object or an array of diagnostic
objects into the `{ diagnostics: [...] }` shape expected by diagnostics writer
artifacts. This keeps formatter and coloring transformations responsible for
their own diagnostics while the writer remains the final serialization phase for
already formatted, colored, and annotated CEM values.

## Encoding, Formatting, And Color Function Declarations

Schema packages and shared modules should be able to declare named encoding,
formatting, and color output functions. Declaration metadata makes helpers
discoverable by the registry and validatable before execution.

Proposed CEMT declaration shape:

```cemt
{encoding-function
    @name="html.text"
    @category="html-text"
    @subject="string"
    @produces="text"
    @content-type="text/html"
    @schema="https://cem.dev/ns/data/html/1"
    @canonical=true
    @streamable=true |
    {param @name="subject" @type="string" @required=true}
    {param @name="mode" @type="string" @default="canonical"}
}
```

Formatter declarations use the same pattern but produce typed formatting
decisions, formatted CEM output trees, or formatted token streams:

```cemt
{format-function
    @name="json.pretty"
    @category="json-document"
    @subject="object"
    @produces="tokens"
    @content-type="application/json"
    @schema="https://cem.dev/ns/data/json/1"
    @canonical=false
    @streamable=true |
    {param @name="subject" @type="object" @required=true}
    {param @name="indent" @type="string" @default="  "}
}
```

Color declarations use the same pattern but produce target-specific styled
output from semantic style roles:

```cemt
{color-function
    @name="terminal.diagnostic"
    @category="terminal-color"
    @subject="tokens"
    @produces="text"
    @content-type="text/plain"
    @schema="https://cem.dev/ns/data/text/terminal/1"
    @canonical=false
    @streamable=true |
    {param @name="subject" @type="array" @required=true}
    {param @name="palette" @type="string" @default="diagnostic"}
    {param @name="capability" @type="string" @default="auto"}
}
```

## Custom Encoding And Formatting Functions

CEMT should ship standard encoders, formatters, and color functions, but schema
packages and application packages must also be able to define custom functions.
Custom functions are how a package captures domain syntax rules, organization
style rules, proprietary wire formats, experimental AI context profiles, or
specialized canonicalization policies without forking the CEMT runtime.

Custom functions use the same declaration families as built-ins:
`encoding-function`, `format-function`, and `color-function`. The difference is
ownership and implementation source, not artifact semantics. A custom function
must still declare its subject type, output kind, content type, schema, category,
streamability, canonicality, params, and diagnostics. It must return the same
typed encoded artifact or formatting/color result that a standard function
returns; it must not return an untagged string that bypasses context checks.

Custom function names should be package-qualified. Standard CEM names are
reserved, and a custom declaration must not shadow a standard function unless
the import site explicitly aliases it. Registry lookup should therefore resolve
functions by `(owner package, name, contentType, schema, category, subject type,
profile)` rather than by short name alone.

Proposed custom declaration attributes:

- `@name`: package-qualified function name, such as
  `acme.markdown.callout-block`;
- `@visibility`: `public`, `package`, or `private`;
- `@implementation`: `cemt`, `native`, or `external`;
- `@profile`: optional named profile selected by `encode` options or package
  metadata;
- `@extends`: optional standard or custom function that this function wraps or
  refines;
- `@capability`: optional required host capability for native or external
  functions;
- `@deterministic`: whether the result is stable for the same input and options;
- `@trusted`: whether the function is allowed to emit raw fragments for a
  schema-gated context;
- `@fallback`: optional fallback function name when the preferred implementation
  is unavailable.

Example CEMT-authored custom encoder:

```cemt
{encoding-function
    @name="acme.markdown.callout-block"
    @visibility="public"
    @implementation="cemt"
    @category="markdown-callout"
    @subject="object"
    @produces="tokens"
    @content-type="text/markdown"
    @schema="https://acme.test/ns/docs/markdown/1"
    @canonical=false
    @streamable=true
    @deterministic=true
    @extends="markdown-document" |
    {param @name="subject" @type="object" @required=true}
    {param @name="marker" @type="string" @default="NOTE"}
    {body |
        {$ write.token("blockquote-marker", "> ") }
        {$ encode($marker, {
            contentType: "text/markdown",
            schema: "https://cem.dev/ns/data/markdown/1",
            category: "markdown-text"
        }) }
        {$ write.token("text", ": ") }
        {$ encode($subject.message, {
            contentType: "text/markdown",
            schema: "https://cem.dev/ns/data/markdown/1",
            category: "markdown-text"
        }) }
    }
}
```

Example native-backed custom formatter:

```cemt
{format-function
    @name="acme.json.stable-api"
    @visibility="package"
    @implementation="native"
    @category="json-document"
    @subject="object"
    @produces="tokens"
    @content-type="application/json"
    @schema="https://acme.test/ns/api/json/1"
    @canonical=true
    @streamable=false
    @deterministic=true
    @capability="acme.native.JsonStableApiFormatter"
    @fallback="json.pretty" |
    {param @name="subject" @type="object" @required=true}
    {param @name="fieldOrder" @type="array" @required=false}
}
```

`encode` should be able to select custom functions in three ways:

- by explicit function/profile option when the caller knows the desired helper;
- by schema package metadata on a serializer edge;
- by registry resolution from content type, schema, category, context, subject
  type, and profile.

This implies extending the logical options shape with optional function selectors:

```text
options?: {
  encoder?: qualified-name,
  formatter?: qualified-name,
  colorizer?: qualified-name,
  profile?: string,
  ...
}
```

Custom function validation rules:

- The declared output identity must be compatible with the surrounding template
  output context before insertion.
- A custom function may call standard or imported custom functions, but every
  nested `encode` result must preserve its own identity and double-encoding
  guard.
- CEMT-authored functions may use writer primitives directly; native or external
  functions must declare capability, determinism, streamability, and fallback
  behavior.
- Raw output requires both a raw category and a trusted/schema-gated function.
- Public functions are versioned as package API. Breaking signature, output
  identity, canonicalization, or safety-policy changes require a package
  version boundary.
- Registry diagnostics must report ambiguous custom function resolution,
  missing capability, unavailable fallback, unsafe raw emission, non-determinism
  in a canonical profile, incompatible subject type, and incompatible produced
  kind.

For a schema-owned serializer, `package.cem` references the serializer template
edge, and the CEMT module declares or imports the encoders it uses:

```cem
{converter
    @id="ast-to-html"
    @implementation="cemt"
    @template="converters/ast-to-html.cemt"
    @template-content-type="application/vnd.cem.transform+cem"
    @template-schema="https://cem.dev/ns/transform/cem/1"
    @template-entrypoint="main"
    @streamable=true
    @lossiness="syntax-normalized"
    @readiness="planned"
    @rust-symbol="HtmlAstExportConverter"
    @fallback-reason="HTML writer primitives are not fully available in CEMT yet" |
    {from @content-type="application/vnd.cem.ast+cem-bin" @schema="https://cem.dev/ns/projection/ast/1"}
    {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
}
```

## Subjects

The subject is the typed value to be encoded. It must be unencoded semantic
data, not a string that already contains target syntax unless the category is
explicitly `raw` and the caller accepts the policy risk.

Common subjects:

- scalar values: string, boolean, integer, number, null;
- names: local names, qualified names, namespace URIs, identifiers;
- structured values: arrays, maps, JSON values, YAML nodes, CSV rows;
- semantic nodes: CEM AST nodes, CEM DOM nodes, XML nodes, HTML nodes;
- token streams: already normalized parser or transform events;
- binary chunks: sealed CEM projection chunks;
- attributes and slots: name/value pairs with target-context metadata;
- fragments: document fragments whose output context is not a full document.

## Produced Values

Encoding produces a typed artifact. The produced artifact should be one of:

- `text`: UTF-8 text by default, with optional charset metadata;
- `bytes`: fully encoded byte sequence;
- `tokens`: target syntax or styled token stream for later writer composition;
- `chunks`: framed binary or streaming output chunks;
- `diagnostics`: non-output result when encoding is impossible or unsafe.

Produced artifacts must carry enough identity to validate downstream use:

- content type;
- schema URL;
- encoding category;
- formatter profile;
- color profile and capability;
- fragment/document mode;
- source-map spans;
- canonicalization mode.

## AI-Facing Context Output

AI-facing output should be a projection over the semantic CEM AST, DOM,
event-stream, schema registry, and token metadata, not a replacement for the
canonical projections. The canonical AST remains the lossless source of truth;
AI context output is a task-shaped view used for retrieval, tool calls,
summaries, and lazy expansion.

The goal is to minimize irrelevant context while preserving enough structure for
an AI consumer to act precisely. Smaller bytes are useful only when the format is
also easy for the target model or tool to interpret. A compact integer token
stream may be ideal for transport or indexing, but a model-facing surface usually
also needs a declared legend, stable names, source ranges, and expansion
references back to the canonical projection.

Useful AI-facing categories include:

- `ai-context-pack`: a bounded JSON or CEM object containing task-relevant
  nodes, summaries, diagnostics, schema identities, and source references;
- `ai-entity-graph`: named entities such as components, tokens, attributes,
  slots, converter edges, schemas, imports, and their relationships;
- `ai-semantic-tokens`: compact classified tokens with a declared legend,
  offsets/ranges, and optional source-map spans;
- `ai-context-fragment`: a subtree or event slice with neighboring context,
  source excerpt, and lazy expansion links;
- `ai-embedding-record`: normalized chunks and relationships for vector or graph
  indexes, with stable IDs back to AST/DOM/event projection nodes.

AI context is exposed through encoding functions, not a separate generic
formatter family. The projection step applies task profiles and budgets to
produce a task-shaped AI projection object. The encoder receives that object,
validates that its projection kind matches the requested category, and serializes
it as an encoded JSON text artifact. Existing JSON formatter controls such as
`pretty`, `lineEnding`, and ordering options still apply through `encode`
options.

AI context encoders should support:

- budgets for nodes, tokens, characters, depth, diagnostics, and source excerpts;
- stable IDs and source ranges for exact edits and follow-up expansion;
- profile names such as `summary`, `navigation`, `refactor`, `token-authoring`,
  `diagnostic`, and `embedding`;
- lossiness metadata that distinguishes omitted detail from normalized detail;
- lazy expansion references to the canonical AST/DOM/event projection;
- host/tool metadata such as audience, priority, and cache identity when the
  encoded artifact is served through an agent protocol.

Example declaration:

```cemt
{encoding-function
    @name="ai.context-pack"
    @category="ai-context-pack"
    @subject="object"
    @produces="text"
    @content-type="application/vnd.cem.ai-context+json"
    @schema="https://cem.dev/ns/projection/ai-context/1"
    @canonical=true
    @streamable=true |
    {param @name="subject" @type="object" @required=true}
}
```

Example use:

```cemt
{$ encode($projection,
  {
    contentType: "application/vnd.cem.ai-context+json",
    schema: "https://cem.dev/ns/projection/ai-context/1",
    category: "ai-context-pack"
  },
  {
    encoder: "ai.context-pack",
    pretty: true
  }) }
```

An AI-facing profile can be faster and more efficient for consumers when it
precomputes the entities and relationships the task needs, avoids full-tree
context dumps, and lets tools fetch exact subtrees on demand. It should not be
the only AST export, and it should be evaluated against representative agent
tasks because over-compressed or unfamiliar formats can cost more reasoning
tokens than they save.

## Terminal And HTML Color Output

Color output is part of CEMT output production, not a terminal-only afterthought.
CEMT should represent color semantically first, then encode it for a target
surface.

Common subjects:

- diagnostic spans and severities;
- syntax-highlight token streams;
- source excerpts with ranges;
- diff hunks and change categories;
- trace, planner, benchmark, and validation report records;
- schema element/attribute names and content-type identities.

Semantic style roles should be stable across targets:

- `diagnostic.error`, `diagnostic.warning`, `diagnostic.info`,
  `diagnostic.fatal`;
- `source.line-number`, `source.gutter`, `source.highlight`,
  `source.secondary-highlight`;
- `syntax.keyword`, `syntax.name`, `syntax.attribute`, `syntax.string`,
  `syntax.number`, `syntax.comment`, `syntax.raw`;
- `diff.add`, `diff.remove`, `diff.context`;
- `status.success`, `status.pending`, `status.muted`.

Terminal color output targets ANSI/SGR-capable text streams. The encoder must
support:

- capability modes: `none`, `ansi-16`, `ansi-256`, `truecolor`, and `auto`;
- environment policy such as no-color and forced-color;
- reset discipline so style does not leak past the produced artifact;
- optional hyperlinks only when terminal capability allows them;
- plain-text fallback that preserves meaning through labels and layout.

HTML color output targets `text/html` document or fragment output. The encoder
must support:

- class-based output for stable artifacts;
- optional inline style output only when explicitly requested;
- CSS custom-property palettes for themeable output;
- accessible contrast policy and non-color cues for diagnostics/diffs;
- escaped text content and attribute values using the same HTML encoders as
  ordinary HTML output;
- fragment-safe output that does not assume a full document wrapper.

Terminal and HTML color output share semantic style roles, but they are separate
encoding categories because their escaping, reset, accessibility, and artifact
identity rules differ.

## Native Pairing And Parity

Native producers are part of the design for performance and implementation
clarity. They should be used when:

- a content type needs a mature low-level writer before CEMT primitives exist;
- binary chunk framing needs native memory control;
- a serializer is performance-sensitive enough to justify a native fast path;
- a native writer makes edge cases clearer and can serve as an executable oracle
  for the CEMT implementation.

Native producers must be paired with CEMT producers:

- same source identity and target identity;
- same fixtures and expected diagnostics;
- same canonicalization/lossiness contract;
- comparison mode declared in package metadata: byte-exact, token-equivalent,
  parse-equivalent, or diagnostic-equivalent;
- drift reported as a parity diagnostic before a native fast path is promoted.

## Encoding Categories By Content Type Family

| Family | Content types | Encoding subject | Category examples | Produced value |
| --- | --- | --- | --- | --- |
| CEM-ML syntax | `application/cem`, CEM vendor `+cem` types | CEM AST node, name, attribute, text, directive, comment | `cem-document`, `cem-fragment`, `cem-name`, `cem-attribute-value`, `cem-text`, `cem-string-literal` | CEM text tokens or UTF-8 text |
| CEMT source | `application/vnd.cem.transform+cem` | CEMT module, template, expression text, call metadata | `cemt-module`, `cemt-template`, `cemt-expression`, `cemt-attribute-value` | CEMT source text |
| XML family | `application/xml`, `text/xml`, `application/xhtml+xml`, `image/svg+xml`, `application/mathml+xml`, `application/xslt+xml`, `application/relax-ng+xml` | XML node, QName, namespace binding, text, attribute value, comment, PI, CDATA | `xml-document`, `xml-element`, `xml-text`, `xml-attribute-value`, `xml-qname`, `xml-namespace`, `xml-comment`, `xml-cdata` | XML text tokens or bytes |
| HTML | `text/html` | HTML DOM node, text, attribute value, URL-ish attribute, raw-text/RCDATA text, foreign SVG/MathML node | `html-document`, `html-fragment`, `html-text`, `html-attribute-value`, `html-raw-text`, `html-rcdata`, `html-comment`, `html-foreign-content` | HTML text tokens or bytes |
| JSON family | `application/json`, `application/schema+json`, CEM projection `+json` debug views | JSON value, string, number, object member name, array/object | `json-document`, `json-value`, `json-string`, `json-member-name`, `json-number` | Canonical or pretty JSON text |
| YAML | `application/yaml`, `application/x-yaml`, `text/yaml`, `text/x-yaml` | YAML stream, document, scalar, sequence, mapping, tag, anchor | `yaml-stream`, `yaml-document`, `yaml-scalar`, `yaml-plain-scalar`, `yaml-quoted-scalar`, `yaml-block-scalar` | YAML text |
| CSV | `text/csv` | table, header, row, field | `csv-table`, `csv-record`, `csv-field`, `csv-header` | CSV text with configured delimiter, quote, and line ending |
| Markdown | `text/markdown` | Markdown document, inline text, code, link destination, table cell, embedded HTML policy marker | `markdown-document`, `markdown-text`, `markdown-code-span`, `markdown-fence`, `markdown-link-destination`, `markdown-table-cell` | Markdown text |
| CSS | `text/css` | stylesheet, rule, selector, declaration, identifier, string, URL token, custom property value | `css-stylesheet`, `css-identifier`, `css-string`, `css-url`, `css-declaration`, `css-selector` | CSS text |
| Terminal color text | `text/plain` with terminal color profile, future terminal-specific content type | diagnostic spans, source excerpts, syntax tokens, diff hunks, report records | `terminal-color`, `terminal-diagnostic`, `terminal-source`, `terminal-diff`, `terminal-syntax` | Plain text or ANSI/SGR text |
| HTML color output | `text/html` | diagnostic spans, source excerpts, syntax tokens, diff hunks, report records | `html-color-fragment`, `html-diagnostic`, `html-source`, `html-diff`, `html-syntax` | HTML fragment or document |
| CEM-QL | `application/vnd.cem.query+cem-ql`, `text/cem-ql` | query module, selector, string literal, identifier, parameter reference | `cem-ql-module`, `cem-ql-selector`, `cem-ql-string`, `cem-ql-identifier` | CEM-QL text |
| RELAX NG compact | `application/relax-ng-compact-syntax` | grammar, pattern, name class, literal | `rnc-document`, `rnc-pattern`, `rnc-name`, `rnc-literal` | RNC text |
| AI context projections | `application/vnd.cem.ai-context+json`, future `application/vnd.cem.ai-context+cem-bin` | CEM AST/DOM/event projection nodes, schema registry records, token metadata, diagnostics, converter edges | `ai-context-pack`, `ai-entity-graph`, `ai-semantic-tokens`, `ai-context-fragment`, `ai-embedding-record` | Structured JSON, token stream, or chunk stream with source-map spans and expansion refs |
| CEM binary projections | `application/vnd.cem.dom+cem-bin`, `application/vnd.cem.ast+cem-bin`, `application/vnd.cem.events+cem-bin` | projection node, event, chunk payload, stream checkpoint | `cem-bin-document`, `cem-bin-chunk`, `cem-bin-event`, `cem-bin-index` | bytes or sealed chunks |

## Safety Rules

- Encoding is context-specific. HTML text and HTML attribute values are
  different categories; XML text and XML attribute values are different
  categories; CSS string and CSS identifier are different categories.
- Raw insertion must be explicit and schema-gated. It must never be the default
  result of `encode`.
- Encoded artifacts must not be silently encoded again.
- A template may concatenate compatible encoded artifacts only when their target
  identity and category allow it.
- Character encoding must be selected at the final byte writer boundary. CEMT
  should work in Unicode scalar values and typed encoded artifacts until bytes
  are requested.
- Color output must use semantic roles first. Terminal ANSI and HTML color
  encoders are target-specific projections of those roles.
- Color must not be the only carrier of meaning. Encoders need no-color and
  accessible fallbacks.
- Terminal output must reset styles at artifact boundaries; HTML output must
  escape text and attribute values before styling them.
- Source maps are part of the encoding result, not an afterthought.
- Custom functions must be package-qualified, validated through the registry,
  and prevented from shadowing standard functions unless explicitly aliased by
  the importer.
- Native and external custom functions must declare required capabilities,
  deterministic/canonical behavior, fallback behavior, and trust boundaries.
- AI-facing output must preserve the data/instruction boundary. Source text,
  comments, diagnostics, and schema prose are data unless the trusted host
  explicitly promotes them to instructions.
- AI context optimization must be profile- and task-specific. It cannot replace
  canonical AST/DOM/event projections, and lossy omissions must be declared.

## Relationship To Conversion

Encoding is the final output step inside a serializer edge. It should not be
used to hide content-type-to-content-type conversion.

Serializer edge:

```text
CEM AST -> encode as text/html
```

Conversion pipeline:

```text
text/html -> normalized HTML model -> application/xhtml+xml
```

Both use registry identities, but conversion may parse, normalize, validate, and
change semantic models before encoding.

## AST Output Artifacts And Mixed Content

The AST is a semantic intermediate, not a single output content type. A
serializer or exporter may expose the same AST through JSON, YAML, XML, CEM,
binary, AI-context, or other registered artifact formats. Each emitted artifact
must carry its own identity:

- `artifactId` or route name;
- destination URI when the artifact is written to a file;
- `contentType` and `schema`;
- encoder, formatter, and color/profile selection;
- source-map and canonical AST references;
- lossiness and inclusion policy when the artifact omits or derives content.

Mixed-content exports are therefore modeled as an artifact collection, not as a
global content-type switch. For example, an XHTML input can produce:

- `page.html` as `text/html`, with style content inlined, linked, or omitted
  according to export settings;
- `page.css` as `text/css`, whose serializer receives a collection of CSS
  subtrees extracted from the canonical AST;
- optional `page.ast.json`, `page.ast.yaml`, or `page.ast.xml` debug/projection
  artifacts for tools and AI consumers.

When settings choose extracted styles, the HTML artifact should contain only the
HTML tree plus the link/reference needed for the CSS artifact. When settings
choose inline styles, the HTML artifact may include HTML plus CSS subtrees in
the appropriate insertion context. In both cases the CSS artifact remains its
own output with its own `contentType`; `output-color-type` can only change
presentation of the written artifact and must not change the target content
type.

AI-facing AST projections follow the same rule. An AI context pack may be
encoded as JSON, YAML, XML, CEM text, binary records, or another registered
format, but the profile must state which artifact routes are canonical,
lossless, lossy, expandable, or budget-truncated. Compact semantic-token or
entity-graph artifacts are optimized views over the AST; they do not replace
canonical AST/DOM/event projections.

## Promotion Checklist

- Add CEMT schema vocabulary for `encoding-function` and `format-function`
  declarations, plus `color-function` declarations. Include custom function
  ownership, visibility, implementation, profile, extension, capability,
  deterministic, trusted/raw, and fallback metadata.
- Add package manifest metadata for serializer edges that name CEMT producers,
  encoder/formatter/color profiles or explicit custom function selectors,
  native paired producers, and parity mode.
- Add shared writer primitive API and CEMT bindings for encoders, formatters,
  color output, token output, byte output, chunk output, and source-map spans.
- Add diagnostics for unknown encoder, context mismatch, unsafe raw insertion,
  unsupported charset, double-encoding, unknown formatter, unsupported terminal
  color capability, inaccessible HTML palette, ambiguous custom function
  resolution, missing custom function capability, unavailable fallback,
  non-determinism in a canonical profile, incompatible produced kind, and
  CEMT/native parity mismatch.
- Add an AI context projection schema or profile that declares context-pack,
  entity-graph, semantic-token, fragment, and embedding-record shapes, including
  budgets, source ranges, expansion refs, and lossiness metadata.
- Add diagnostics for unsafe AI data/instruction mixing, unsupported AI context
  profile, missing expansion target, and budget-driven omission.
- Add multi-artifact export metadata for AST projections, including per-artifact
  content type/schema, route/destination, extracted subtree selectors, and
  inclusion policy for mixed outputs such as XHTML plus CSS.
- Add examples for CEM, XML, HTML, terminal color text, HTML color output, JSON,
  CSV, CSS, AI context projection output, and CEM binary projection output.
- Add parity tests comparing CEMT producers with native paired producers.
- Add task fixtures or evals for AI context profiles so compact forms are
  accepted only when they improve retrieval, edit precision, or token budget.
