# Typed prelude reference slots

Status: specification adopted 2026-10-09 following the explicit request recorded
in [archived checklist](archive/todo-snapshot-2026-10-10.md#5-integrate-schema-validation-and-construct-reuse).
The 1.1 parser, original lexical capture, native consumers, canonical writer and
versioned reload are implemented. Explicit 1.0 and broad major constraints retain
literal directive parsing. Verification and release gates are recorded there.

This contract extends document and opening block preludes with original native
value slots. It uses the existing [reference and scope contract](cem-ql-cem-ml-node-references-design.md)
and [recognition rules](cem-ml-syntax.md#schema-scoping-a). It is independent of the
already implemented containing-host and wrapping schema forms. It adds no
execution authority, automatic loading, datatype override or function registration.

## Admitted source forms

The initial typed sites are finite and predefined. A directive name, namespace
prefix, source field name or format/version identifier is always literal.

| Directive site | Literal form retained | New native value | Consumption contract |
| --- | --- | --- | --- |
| `@schema select` | Quoted selector text | One `{#...}` reference or `{$ ...}` expression | Exactly one original admitted schema declaration after bounded resolution |
| `@ns prefix =` | Literal namespace URI | One reference or expression | Exactly one original admitted namespace declaration/provider; the literal left side supplies the destination prefix |
| `@default` | Literal prefix alias or URI | One reference or expression | Same namespace admission; destination prefix is blank |
| `@schema src` and bare schema URI | Literal URI | None | Existing explicit loader handoff |
| `@doc` and other directives | Existing literal payload | None | Existing format/control semantics |

Examples below use the 1.1 grammar. Runtime bindings `schemaChoice` and `namespaceChoice` are
supplied by the caller, not declared or fetched by these directives.

```cem
@doc cem-ml 1.1
@schema select={#schemaChoice}
@ns ui = {#namespaceChoice}
@default ui

{ui:panel | Content}
```

The expression counterparts are `@schema select={$ schemaChoice}`,
`@ns ui = {$ namespaceChoice}` and `@default {$ namespaceChoice}`. An explicit
reference constructor creates a reference AST node; a general expression retains
its expression wrapper and must yield native nodes/references accepted by the
same consuming slot. Neither form accepts a URI string as a selected declaration.
Namespace selection reuses a completed original binding, including an empty URI
reset. A selected namespace declaration's own prefix does not rename the
consumer's literal destination prefix.

A quoted payload stays a literal payload. For example,
`@schema select="{#schemaChoice}"` stores selector text and can construct a
reference when the existing literal-query consumer runs; it contains no authored
reference AST node. Quoted namespace URI text containing braces stays URI text.
There is no interpolation inside literals and no string-to-node coercion.

## Recognition and grammar

Typed syntax follows the existing document directive placement rules. Inside a
block, only `@schema`, `@ns` and `@default` on their own opening physical lines,
preceded by spaces/tabs, can form directives. Existing whitespace/comment rules,
LF/CRLF/CR handling, nested body restoration and the first-substantive-item cutoff
remain in force. Same-line controls, escaped directive-looking text, unknown block
directives and controls after body content remain ordinary content.

Within a recognized directive, the native constructor must occupy the entire
admitted value. Horizontal whitespace may surround it. The field/prefix syntax
is parsed as directive syntax, not as an element header:

```text
schema-native  := "@schema" hspace+ "select" hspace* "=" hspace* native-value hspace*
ns-native      := "@ns" hspace+ prefix hspace* "=" hspace* native-value hspace*
default-native := "@default" hspace+ native-value hspace*
native-value   := reference-constructor | expression-constructor
```

`reference-constructor` starts with `{#`; `expression-constructor` starts with
`{$`. Their expression bodies use the existing CEM-QL lexical/delimiter rules,
including nested braces and strings. Bare `{expression}` shorthand is not admitted
in directive values. This keeps the new branch distinguishable from literal URI
and header text. No new positional native shorthand for `@schema` is introduced.

The first implementation is confined to one physical directive line. Nested
constructors are permitted within that line; physical newlines inside a native
value are errors, including inside quoted query strings. Use existing host or
schema-element forms for multiline expressions. An unterminated constructor is
an attributed syntax failure at that directive; bounded recovery stops at its
line boundary and never consumes the next sibling or enclosing block close as
part of a guessed value. Canonical output puts a block close on a separate line.

`src` and `select` remain exclusive; repeated fields are errors. Mixed literal
and native payload, multiple constructors, trailing non-whitespace, an empty
constructor and native values in unsupported fields are errors. A malformed
native branch never falls back to literal interpretation or inherited governance.
Comments inside the expression obey its lexer; there is no new trailing directive
comment syntax. Quoting remains the way to author a brace-looking literal value.

## Retained source representation

Typed slots are built during the primary token/event/AST pass. The existing
original directive wrapper (`@schema`, `@ns` or `@default`) retains its identity,
source frames and prelude form. Its typed value is an original owning child edge
to the existing reference node or expression wrapper. Native reference targets
remain non-owning runtime edges, never copied structural children.

The parser captures a typed slot record containing:

- The original directive node and value node IDs, valid only with their owner.
- A closed role: schema selector, named namespace value or default namespace value.
- The literal destination prefix when applicable, the directive extent/form,
  and the original field/prefix/value source spans.
- The lexical snapshot immediately preceding the declaration and its required
  format capability.

Literal prefix/header text and trivia may remain original text children for
source preservation; the captured role/value edge determines consumption. Existing
literal-only directives keep their current text representation. A typed directive
must not be sent through the legacy concatenated-text decoder.

Introduce structural token/event information for native directive boundaries and
slot roles as needed, retaining the legacy literal event path. Do not tokenize a
whole typed payload as `Directive { data: String }` and later reparse that string.
Do not manufacture an `Attribute` for `select`/`xmlns`, clone a second CEM tree,
reconstruct a reference from printed text or derive authority from `@`-prefixed
node names. A wrapper lacking original role/form/owner metadata cannot activate
a typed slot; a foreign lookalike element remains ordinary data.

Consumers can use a small owner-checked directive-value view alongside the
existing attribute-value view. Both feed the shared selection/admission APIs;
the view preserves whether its original source occurrence is an attribute or a
directive. Any generalization of `SchemaHostControl.attribute` or
`NativeNamespaceProperty` must retain that distinction and existing callers.

## Lexical dependencies and scope extent

The native value captures the environment before its own declaration. A newly
selected schema or namespace cannot govern its selecting expression retroactively.
Following source names capture the new declaration dependency from its source
position forward, while preceding names retain their prior bindings.

A typed `@ns` or `@default` initially records a pending namespace declaration.
Dependent expanded names remain unavailable until explicit namespace preparation
completes them. Do not install constructor text as a namespace URI, treat a pending
prefix as a literal URI, or use a shadowed earlier binding to fabricate completion.
The literal alias `@default ui` records a dependency on the preceding `ui`
declaration, including when that declaration is pending. Later rebinding of `ui`
does not change the original provider selected by that earlier alias.

A typed `@schema select` similarly records a pending schema source. At an entered
following transition, it governs later siblings and descendants in that original
parent scope. The directive itself and its selecting value keep enclosing
governance. Pending or invalid explicit overrides make dependent governance
unavailable; they cannot borrow the previous model. Closing a block restores the
enclosing lexical environment and consuming model. Namespace selection and schema
selection remain independent; one cannot substitute for the other.

Directive payload children belong to the control occurrence and are excluded
from ordinary application behavior candidates. Reusing a source subtree preserves
its original declarations and lexical dependencies while following the existing
consuming-placement rules. Enter only controls reached by the explicit lifecycle
scheduler; do not scan unrelated directives to infer inputs or grants.

## Selection, readiness and invocation

Parsing, importing, formatting and structural inspection execute no queries and
perform no I/O. At an explicitly entered lifecycle stage the caller supplies the
original retained source, captured slot metadata, current context, effective
policies, operation control and directed grants. A parsed slot alone authorizes
none of these capabilities.

The shared resolver processes reference constructors and any reference chains
returned by general expressions. Singleton cardinality is checked after bounded
resolution: complete zero/multiple results, including repeated identical nodes,
are invalid. Schema slots use existing schema target admission and compilation.
Namespace slots use original recorded namespace providers, including explicitly
completed native providers. A data element, scalar string or matching local name
cannot stand in for either contract.

| Outcome | Required behavior |
| --- | --- |
| No current context, unavailable names/provider, or unfinished dependency | Retain pending state and original handles; do not activate |
| Complete singleton with admitted target and ready compiled/provider dependencies | Publish the invocation-local following assignment |
| Empty/multiple/scalar/wrong-kind target, malformed syntax or invalid contract | Retain attributed invalid state; prevent activation |
| Denied crossing, cycle, exhausted work/depth or cancellation | Keep the specific incomplete/failure reason; warning/ignore disposition does not create readiness |

One preparation attempt shares finite work, depth and cancellation accounting
across directive sites, namespace/name dependencies, schema selection and nested
compilation. Destination policies can tighten limits and never replenish them.
Cycles use original owner/declaration/occurrence identities and consumer roles;
completed diamond reuse is allowed. Source text cannot supply larger limits.

Successful assignments are local to the current invocation. Fresh context,
publication, grant or owner generations require fresh preparation; stale or
cancelled work cannot publish. Retry uses the same retained source without saving
targets on authored nodes. Keep original binding providers distinct from selected
namespace declarations and destination prefixes. Restore assignments on exit or
failure, using existing lifecycle cleanup. If a consumer publishes a persistent
package snapshot, apply its existing coordinated ready-only publication gate;
failed replacement preserves the previous complete snapshot.

Diagnostics identify the original directive, precise value/field span and relevant
selected target/provider. Preserve resolver/query diagnostics and source frames,
including on partial failure. Inspection distinguishes literal selector text,
authored reference syntax, general expressions and unavailable metadata without
executing them or claiming completion.

## Compatibility, formatting and transport

The grammar is released in the minor CEM-ML format profile **1.1.0**;
`SUPPORTED_VERSION` is 1.1.0. Typed source documents must declare a constraint requiring at
least 1.1.0, such as `@doc cem-ml 1.1`; a broad `@doc cem-ml 1` does not declare the
required capability. Existing 1.0 documents retain their literal directive
interpretation, including previously accepted unquoted brace text.

Fragments require an explicit compatible enclosing/host format profile. Source
shape alone cannot opt a fragment into 1.1. Resolve the format capability before
choosing directive lexical mode, without building and reparsing an intermediate
AST. An unsupported required profile fails before schema activation; old 1.0
readers reject the 1.1 requirement instead of accepting a different namespace or
selector interpretation. Release the version increment only with parser,
consumer, writer and reload coverage together.

Canonical CEM-ML output uses the admitted native spelling, the original directive
role/prefix and one-line constructor. It does not quote a native slot into a
literal, unquote a literal into a slot, flatten the reference target graph or
mutate the source. A programmatically supplied multiline value cannot be emitted
as a one-line prelude without an explicit valid formatting strategy; otherwise
report unsupported output. Never silently rewrite it into a host attribute.

Retained binary and executable reload representations preserve the original
owner/child edges, slot roles, lexical dependencies, form metadata, spans and
required format capability. Extend/version the existing metadata boundary rather
than serializing an AST through JSON. Reject unknown required metadata versions
or missing typed-slot metadata; do not downgrade to a literal or activate an
unverified handle. Runtime contexts, grants, completed selections and active
assignments are not resumable source metadata. XML keeps its existing explicit
attribute-slot admission; this design adds no processing-instruction shorthand.

## Parser and import admission

Normal `CemTokenizer::from_source` admits typed slots after a leading
`@doc cem-ml 1.1` or `1.1.0`. `from_source_with_format_profile` admits a compatible
headerless fragment. The shared byte import offers
`import_bytes_with_lexical_scopes_and_profile` for the same explicit capability.
An authored leading header overrides host admission; broad 1/1.0 constraints
retain literal parsing. Late/nested headers cannot change lexical mode. Future
and prerelease requirements fail document admission.

The original `from_source_with_typed_prelude_preview` API remains available for
callers supplying tighter bounds; its control-aware counterpart accepts the
existing operation control and scope.

`TypedPreludePreview` permits tighter value byte and brace/comment nesting
limits, capped at 64 KiB and 128 levels. Recovery stays on the physical line and
preserves a following block close. Invalid native attempts retain an error child
and slot metadata; their namespace/schema effects remain unavailable. Parser
facts use the schema-owned `tokenizer-invalid-typed-prelude` diagnostic binding.

Structural `TypedDirective` tokens and `TypedPrelude` value events produce the
original owning child directly. `LexicallyScopedDocument::typed_prelude` checks
the original owner before returning the directive/value handles, role, prefix,
spans, preceding snapshot, following extent and required version. Pending
namespace declarations and aliases use those original directive IDs; a
`SchemaSource::PendingPrelude` prevents inherited schema fallback. No query is
executed. Debug native binary codec 5 retains passive source metadata version 1;
lexical reload metadata version 2 additionally preserves preceding snapshots and
pending dependency records. Older literal-only payloads remain readable. Missing
slot records, changed edges, mismatched identities, unknown versions and partial
pending dependencies fail validation. Runtime state is never serialized.

`formatter::format_with_profile` rejects unsupported target profiles and 1.0
downgrades of typed slots. The native typed-tree/tabular writer lowers original
slot metadata directly to canonical directive spelling for presentation. It
rejects malformed or multiline native output rather than converting it to a
literal; the retained source arena is unchanged.

## Native consumers

`decode_native_prelude_value` validates the captured role, required profile,
original owner and directive/value edge. It preserves reference versus expression
kind. Namespace property decoding accepts those original directive handles;
`validate_typed_schema_prelude` returns the existing following-scope contract.
`SchemaHostControl.attribute` retains compatibility as the original source site:
an attribute, literal prelude text, or typed directive, with the original native
value exposed by `SchemaHostSource::NativeSelector`.

For native schema execution, `attach_captured_names` retains the validated typed
contracts and the existing runtime region scheduler consumes them when entered.
Namespace execution uses `attach_captured_namespaces` with explicit property
preparation/publication, or `with_namespace_lifecycle` for dependency coordination.
Pending aliases, completed providers, directed grants and cumulative bounds use
the shared namespace machinery. Missing metadata and incomplete values keep the
following schema or namespace unavailable. Directive payloads never enter the
ordinary application walk, including under an empty schema model.

`CemQlSchemaDeclarationHost::set_operation_control` attaches the caller's shared
operation and execution scope. Selector evaluation, lifecycle handoffs and
publication honor that operation. A stopped operation returns the resolver's
`OperationStopped` error at lifecycle checkpoints; the supplied operation keeps
the specific cancellation/deadline cause. Replacing the operation expires prior
namespace proofs. Direct namespace activation validates the same current
snapshot and original dependency assignments as publication, and cancellation
during context preparation installs no partial context assignments.

The native tests cover cardinality, invalid targets, original providers,
empty resets, alias timing, nested restoration, dependency cycles and diamonds,
grants, budgets, cancellation, stale results and retained-source retries. This
includes public 1.1 import, canonical/tabular round trips and executable reload
with fresh contexts and directed grants. Native and Nx verification is recorded
in the checklist.

## Delivery and verification

Implementation is tracked immediately after the completed specification item
in [todo.md](todo.md). Add each fixture before its consumer implementation.
Required layers are source recognition and capture;
namespace/schema preparation and readiness; canonical output, binary/reload
transport and version admission; then end-to-end native lifecycle integration.
Existing literal, host-bound, wrapping and following forms must retain their
current behavior throughout.
