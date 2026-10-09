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
precedence, runtime loops, scoped variables, non-composite `group-by` with native
XSLT group context, stable dynamic multi-key sorting, XPath conditionals and
simple text construction. The transform/CLI adapter uses this strict XSLT 3.0 path. WASM
`compileXsltBundle` / `retainXsltStylesheet` expose default compiler options;
native-built module closures load through the explicit bundle API. Standard parsing
and typed recovery use CEM import. Native result construction, output AVTs,
whitespace and static-style exports are implemented. The bounded viewer passes
native/CLI, WASM and browser parity; full XSLT conformance remains outside this
profile. Grouping uses
XPath key equality and preserves native items; the CEM `seq:group_by` identity
contract is not substituted for XSLT semantics. Sorting uses common numeric
promotion and native XPath comparisons; stylesheets explicitly author any
missing/invalid-last policy. Sort control AVTs use outer focus, keys use original
population focus, and template bodies use sorted positions.
Shared `CallDepth` accounting measures active nesting; completed and recovered
calls release depth while `FunctionCalls` stays cumulative. The existing limits
apply equally to named, lambda and native calls.

## String helpers

Tier A `str:` functions include literal `split(value, separator)`,
`trim`/`trim_start`/`trim_end`, `char_at`/`at`, and `index_of`/`last_index_of`.
They complement the existing case conversion, substring, containment,
replacement, and joining functions. `trim`/`trim_start`/`trim_end` and
`normalize_space` accept an optional `"xml"` profile for XML whitespace only
(space, tab, CR and LF); omitted or `"default"` preserves the existing behavior. Indices use Unicode codepoints; split
returns a sequence and preserves empty fields, including boundary empties
when the separator is empty, following Rust `str::split`. For example, a whitespace word
count that retains repeated words and treats blank input as zero is:

```cem-ql
seq:count(seq:where(
    str:split(str:normalize_space(text), " "),
    fn(word) => word != ""
))
```

See the [string-function contracts](../../docs/cem-ql-stack-design-impl.md#112-cemstdlibstrings)
for signatures, edge cases, and differences from JavaScript.

## URL functions and parameter core

The URL core uses the independently versioned `cem-url` 0.1.0 fork of upstream
`url` 2.5.8. The exact path/version dependency supports local development; the
fork must be published before this crate. See the
[fork release contract](../../vendor/url/RELEASING.md) and run
`yarn nx run cem_ql:verify:url-package` for archive and packaged-feature checks.


The native Rust `stdlib::url_params::UrlParams` core provides immutable ordered
query parameters, form parsing/serialization and strict CEM-QL value adapters.
Mapping records use deterministic key order; pair streams retain order and
duplicates; parameter sorting is stable by UTF-16 code units. Values remain
ordinary typed records and arrays, with no JSON handoff or host URL service.

The Tier A `url:` alias (`cem:stdlib/url`) now registers these functions from
the [accepted URL contract](../../docs/cem-ql-url-contract.md):

| Function | Result |
| --- | --- |
| `can_parse(input, base?)` | Boolean; invalid syntax returns false |
| `parse(input, base?)` | Zero or one parsed record |
| `href(input, base?)` | Canonical anyURI; invalid syntax raises an error |
| `assemble(parts, base?)` | anyURI; requires parts.href or explicit base |
| `with_parts(input, parts)` | anyURI; immutable fixed-order component updates |

Text arguments accept one string or anyURI, without implicit atomization.
Parts reject unknown/read-only fields and overlapping host/hostname/port or
search/query assignments. Ignored setters emit source-mapped warnings; partial
host changes survive an invalid port. No ambient base or host URL service is used.

```cem-ql
(
  url:href("../asset.svg", "https://example.test/docs/page"),
  url:with_parts("https://example.test/a", { pathname: "/b", hash: "details" })
)
```

All 13 `url:params*` operations are registered: `params`, `params_size`,
`params_entries`, `params_keys`, `params_values`, `params_get`, `params_get_all`,
`params_has`, `params_append`, `params_delete`, `params_set`, `params_sort`, and
`params_string`. Parameter arguments require exact `{name: string, value: string}`
entry streams; names and values require strings (not anyURI). An omitted optional
value differs from an explicitly empty stream. Updates return new streams.

`url:params` accepts form text, a mapping record, or typed two-string arrays from
bindings; CEM-QL has no square-bracket array literal syntax. Mapping keys use Rust
string order; ordered pair streams preserve duplicates and caller order.
For example, `url:params_string(url:params_set(url:params("a=1&a=2"), "a", "3"))`
returns `"a=3"`. See the [generated implemented reference](../cem_ml/schema-packages/cem-ql/v1/url-functions.md).
Known parser/setter compatibility gaps remain tracked and prioritized in
[the wishlist](../../docs/wishlist.md#rust-url-compatibility-gaps).
Focused coverage: `cargo test -p cem-ql --test url_query --test url_params --test url_params_query`.
WASM query execution is checked with `node packages/cem_ql/tests/url-query-wasm.mjs`
after the package WASM build.

The shared [30-case matrix](fixtures/url/query-matrix.json) runs through native,
Node WASM and browser query APIs. The same [CEMT fixture](fixtures/url/render.cemt)
proves native server HTML and browser DOM output with changed bindings. This is
native renderer evidence, separate from the legacy Edge/SSR transport fixture.
Focused targets are `cem_ql:test:url-integration`,
`cem-elements:test:url-integration` and `cem_ml_cli:test:url-integration`.

The CLI verifies all 30 rows, including imported aliases, local declarations and
ordered nonfatal warnings. Its [explicit module-query mode](../../docs/cem-ql-cli-module-query-design.md)
selects compilation from the declared media type and matching schema. Modules
require a URI header and one final expression; imports are limited to registered
built-ins. The CLI fatal-error summary is checked separately from the original
language diagnostic.

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

`namespace_names::NamespaceQueryTree` accepts a ready
`cem_ml::schema::namespace_references::NamespaceNameCompletion` for the same
original source owner. It exposes completed names per execution through native
fields and axes; `source` explicitly returns authored inspection. All namespace
dependencies in the selected forest must be ready before construction. Parent
and related-node navigation stay inside that forest, and reference descendants
remain authored. Native `#` values retain the execution view, while
`retained_cem_node` recovers the unchanged original handle. Evaluation, crossing
grants and lifecycle activation remain explicit consumer stages.

The shared declaration host also provides `attach_captured_namespaces` and
`prepare_namespace_scope`. This explicit namespace consumer follows reference
chains under request/destination bounds and directed grants, then admits exactly
one original namespace declaration with a ready destination context. Missing
metadata, pending bindings and incomplete inputs remain inspectable; namespace
preparation does not extract URIs from schema/data nodes, evaluate a selected
pending declaration's own value slots, activate a scope or rewrite source targets.

`prepare_namespace_property(declaration, limits)` consumes an original captured
native namespace attribute's owning value slot. Its report retains the original
`NativeNamespaceProperty` (destination prefix and value handle), bounded
`NamespaceScopePreparation` and separate property/metadata readiness issues.
Original reference constructors use the shared resolver; a general expression
uses the existing lifecycle expression hook in that same traversal. Only that
authored slot is executable; selected expression-looking nodes remain targets.
Occurrence contexts supply pre-declaration inputs, and request/destination limits,
directed grants and source diagnostics remain active. Ready singleton results can
be passed to `NamespaceNameCompletion` under the original declaration ID. The
selected target's prefix does not replace the property's prefix. Empty default
URIs are ready resets. Calls neither cache results nor write source targets,
complete captured bindings, activate a scope or consume a selected pending
declaration's own slots.

`activate_namespace_properties(captured, roots, prepared, callback)` consumes ready
property reports, completes selected names and installs original occurrence
contexts through the existing lexical handoff. It preflights owner/property
identity, duplicate reports, readiness, selected dependencies and assignment
conflicts before callbacks or mutation. The callback receives the original
occurrence, completed lexical snapshot, enclosing scope and this execution's
`Arc<NamespaceNameCompletion>`; it returns runtime context readiness and local
policy overrides. A `None` context stays pending. The returned activation exposes
that same completion and the installed scopes. Bind `NamespaceQueryTree` in
contexts or supply the completion to shared native query ingress with the original
source. Selector-only roots can activate before governed children; already assigned
selectors remain outside subsequent roots. Activation does not reevaluate selectors,
reset bounds, grant crossings or publish results on the source. Independent
executions retain separate views.

`publish_namespace_property(&prepared)` explicitly publishes a ready result for
later target admission within its producing execution-input snapshot. It retains
the original property handle and literal binding provider separately; no parser
binding, synthetic binding ID or source targets are created. Pending, stale or
mismatched reports reject, and matching publication is idempotent. A changed host
context expires publications; changed scopes on visited dependencies make them
unavailable, including dependencies inherited through reused publications.
Unrelated occurrence handoffs do not alter those inputs. Later consumers still
apply their own grants and request/destination bounds without reevaluating the
published declaration's value. Independent hosts do not borrow each other's
results. `with_namespace_lifecycle(captured, roots, limits, prepare, consume)` coordinates
those stages at an explicit invocation. It prepares original selectors only after
their captured pre-declaration bindings are ready, publishes ready values, retries
pending declaration dependencies under a cumulative work cap and supplies one
matching completion and temporary occurrence scopes to `consume`. Existing
explicit occurrence inputs remain authoritative. Missing inputs return an
incomplete snapshot for caller retry; disjoint ready roots remain consumable.
Dependencies outside a selected query forest supply bindings without entering
its axes. Scope assignments, invocation names and publications restore on return,
error or unwind; retained query/validation output remains inspectable.

Use `snapshot.ready_roots` for partial consumption or require
`snapshot.is_complete()` before consuming the whole selection. Original namespace
attributes with completed parser bindings or current published consumer results
are scope controls during structural validation, rather than application data
attributes needing another node contract. Unconsumed native properties and
ordinary lookalikes retain their existing contracts. The native validation session
adapter offers an explicit opt-in hook; ordinary parse/load/query preparation
never invokes this coordinator.

`with_completed_namespace_names(completion, callback)` exposes a registered
original owner's selected-forest name completion to schema target admission and
control discovery and structural attribute lookup during that callback.
`consuming_expanded_name` supplies the effective name; `captured_expanded_name` remains original. Names outside the forest
keep original capture readiness. Nested same-owner invocations temporarily replace
completion; other owners retain theirs. Return, errors and unwind restore earlier
completion. This provides no runtime context, crossing grant or target-binding
readiness, and does not rewrite source/capture or reset reference budgets. Pending
schema QNames retain their original producer form; completed namespaces still
identify core controls versus foreign data.

Completed namespace views also reach native query ingress and explicit lexical
context handoff. Structural attribute permissions and literal typing use the same
completion for owning and reference-selected placements. Missing names defer
presence/field checks and native slots with pending names; original source/value
handles and bounded
reference traversal remain intact. Retained validation/behavior and attribute-target
views now retain this execution's completed names after invocation restoration or
host disposal. Their host-backed `source` field exposes the authored native tree;
attribute `valueNodes` retains original slots. Source handles/maps, placement/model
boundaries and native target identity remain intact. Pending names within authorized
attribute targets defer dependent behavior. Ready names alone do not establish
validation readiness. See the [scope-property adoption audit](../../docs/cem-ql-cem-ml-node-references-design.md#scope-property-consumption-audit-2026-10-06)
for implemented contracts, fixture coverage and remaining work.

### Native values in templates

CEM-QL accepts optional `$` prefixes on expression references in queries and
templates: `$s ?? $a` and `s ?? a` resolve the same bindings. Declarations and
parameters remain bare, as in `{ let value = 1; $value + value }`; field names,
record keys and type names also remain bare. The prefix must touch the name.
Strings and comments preserve literal dollar signs. CEM-ML's content marker
is separate: `{$s ?? $a}` contains the query `s ?? $a`, while
`@value="{$s ?? $a}"` contains `$s ?? $a`.

Named attribute types compose with local restrictions; bounds and regex patterns
must all hold after final conversion. CEMV version 3 preserves the complete
contract through clones, XPath-selected attributes, workers and saved pipelines,
while retaining reads of versions 1 and 2. See the
[native value contract](../../docs/cemt-native-values.md) for normalization,
receiver constraints and resource controls.

`{$node}` reuses a retained node subtree in body content. Use
`{$dom:text(node)}` for text alone. `@alt="{$node}"` retains the native value
until the final attribute projection extracts text. Mixed attribute content
keeps its ordered literal and native segments between transformation phases.
Sources remain immutable; only `dom:clone(values)` requests independent node
storage. `dom:element(elements)` creates empty named element shells, and
`dom:reference(values)` constructs an explicit native reference.
Cloning a reference preserves its node kind and shared target relationships.
Select `.targets` explicitly to clone each target independently or to pass its
elements to `dom:element`. This behavior is identical after portable transport;
see the [operation guide](../../docs/cemt-native-values.md#choosing-a-native-value-operation).
`cemt:apply_templates(values, mode)` calls matching templates through the active
CEMT host and returns native content for further querying or insertion. It requires
both arguments and reports a missing-host error in standalone query evaluation.

`dom:text()` reads the active matching-template focus. In an expression hook,
the focus is the entire expression sequence, including empty input. Missing
focus is an error. For example:

```cem
{template @on=expression @into=attribute | {$dom:text()}}
```

Matching supplies `node` without a redundant parameter declaration. Matching,
named and expression-hook templates accept direct bodies, including imported
module templates. Use direct content or one explicit `body`; mixed and duplicate
bodies are errors. Each source is an implicit module: imports and declarations
need no `module` wrapper, and executable root content needs no `body` wrapper.
Matching rules need no `@name` unless called by name. Explicit wrappers remain
supported with the same body rules. See the
[compact forms](../../docs/cemt-native-values.md#compact-template-bodies).

Attribute bodies and `@value` interpolation both use attribute hooks. Controls
and template calls retain that destination and its metadata; constructed node
content uses content hooks, and nested attributes use their own attribute hooks.
Direct hook returns bypass redispatch. Native types and node identities survive
until the destination applies its conversion and validation.

Scalar types, regex/range constraints and final representations are distinct
attribute contracts. Portable native CEM artifacts preserve them across workers
and saved pipelines. JavaScript carries artifact bytes and control metadata;
CEM-ML owns graph import, and CEM-QL consumes native views. See
[the complete native-value contract](../../docs/cemt-native-values.md) and
[the cell override examples](../cem-elements/demo/cell-overrides.html).

Native integers outside `i64` keep integer datatype metadata and use the
existing decimal evaluator before and after transport. Query type checks see
decimal atoms for those values. Matching numeric operands and existing decimal
limits still apply: use `value + 1.0` for decimal addition, with no implicit
floating-point conversion.

Receiver `{attribute @name=count @type=integer @minInclusive=1}` declarations
validate incoming native values before rendering. Imported expression hooks
provide module defaults below caller overrides. XPath functions also consume
constructed and portable CEM values through a cached shared native projection;
they preserve output occurrence parents separately from reference targets.

### Original schema attribute type bindings

`attribute_datatypes::bind_attribute_datatype` binds one original schema attribute
`@type` slot to an exact ready `ExecutableDatatype`. Literal slots use an explicit
host binding recorded by `CemQlSchemaDeclarationHost::bind_literal_attribute_type`;
native `@type={#target}` slots evaluate in their captured scope. Both use the shared
bounded reference resolver, require a singleton datatype target, and honor directed
scope grants. Retained slot, declaration and datatype owners remain available in
`BoundAttributeDatatype`. Its `local_constraints()` accessor preserves the authored
attribute metadata, including the pending native-type flag; `constraint_fields()`
retains every original field occurrence and source span. Metadata follows the
existing last-authored-slot precedence. Effective constraint fields must have
completed, unqualified names and literal values (apart from the native type slot).
Pending constraint values produce no binding; foreign namespace fields are rejected
instead of losing their namespace during projection. A same-named declaration from
a replacement owner cannot satisfy an older compilation snapshot.

This API binds descriptors only. It does not prepare literal values, invoke
conversion, apply attribute-local facets, or clear model readiness. The dedicated
`schema:attribute-type` metamodel contract admits native slots only on attribute
declarations, preserving literal QName checks and pending compilation state.

### Lexical preparation for datatype validation

`DatatypeImplementations::select_preparation` explicitly selects a registered
`NativeLexicalPreparer` for an original scalar implementation, or
`RegisteredLexicalPreparation::list_items` for tokenization through the effective
item preparer. `datatype_shipped::lexical_preparation` supplies the shipped
capabilities. Derived descriptors inherit their base's original preparer; replacing
that capability on a derived declaration is currently rejected. A selected
unavailable preparer prevents compilation readiness; no selection leaves a
usable typed-only descriptor.

`ExecutableDatatype::prepare_lexical` takes the original `LexicalInput`, optional
native candidate, caller runtime/control and `PreparationLimits`. It prepares the
typed value, then runs every effective datatype restriction. The report retains
the original input, preparer identity, diagnostics and ordered list token spans in
decoded UTF-8 bytes. Scalars require exactly one correctly typed output; lists use
their registered tokenizer and prepare each scalar item without converter calls.
Shipped boolean presence is true, while `1` and `0` remain invalid validation input.
Integer `003` prepares as integer `3` while retaining the original spelling.
String values preserve whitespace; other shipped lexical values follow their
existing boundary-whitespace admission rules.

Preparation and validation share input-visit and diagnostic budgets; preparation
calls, input bytes and output counts have explicit caps. Callbacks must cooperate
with the supplied control and limits. Rejection differs from pending/unavailable,
malformed, interrupted or exhausted execution. Partial preparation exposes no
value; complete prepared values may remain inspectable after validation rejects
or stops. Publication requires `accepted == Some(true)`. Typed `validate` calls
bypass preparation and conversion. Automatic attribute-local facet integration
and model activation remain pending in the TODO.

### Registered attribute facet profiles

`DatatypeImplementations::select_facets` selects a `RegisteredFacetProfile` for an
original datatype declaration and scope. The profile records an implementation ID
and explicit `FacetFamily`; authored names and scalar representations alone select
no facet behavior. Derived descriptors inherit the same original profile. Duplicate,
unrelated or representation-incompatible selections fail. An explicitly unavailable
profile prevents readiness; an absent profile leaves typed validation available.
Replacing a derived descriptor's inherited profile is deferred and rejected.

`BoundAttributeDatatype::compile_facets` requires that profile and compiles local
facet applicability and syntax with the shipped validators. The resulting
`BoundAttributeFacets` retains the original binding and profile. Its contract's
`validate_with_check` checks local restrictions with `FacetLimits` and caller control;
`FacetContext` supplies original source attribution and declaring-scope diagnostic
bindings. Scalar checks retain URI/path/string/numeric semantics. Lists receive
already-prepared counts without another tokenization pass; nodes receive counts
without scalar extraction. Attribute-local `values` still compare lexical values;
datatype enumerations separately use registered equality. Local whitespace checks
do not overwrite authored input or the prepared typed value.

The returned acceptance covers local facets only. Completed datatype validation is
also required. Original metadata and the source model's native-type readiness guard
remain intact.

### Combined explicit attribute validation

`BoundAttributeFacets::validate_lexical` joins registered lexical preparation,
complete datatype validation and local facets in one `AttributeValidation` report.
`validate_nodes` instead takes a completed native `ValidationInput`, validates its
whole sequence and applies local count constraints. The host must finish reference
resolution before calling it; pending selection cannot be supplied as an empty
sequence. Native inputs preserve the original query views and authored descendants.
These entry points never invoke converters or stringify native values.

Only `accepted == Some(true)` permits publication. A completed rejection remains
false regardless of diagnostic severity; a stopped phase makes the combined result
incomplete. `AttributeDatatypePhase` retains original inputs, prepared values and
individual datatype reports; `facets` retains a completed local result. An input
rejected during preparation has no fabricated value or local facet report. Scalar
and list facets use the original lexical text, and list counts come from the prepared
sequence without another tokenization pass.

`AttributeValidationLimits` carries preparation/validation limits and a facet model
byte cap. Local facets receive only the diagnostic allowance left after preparation,
rule diagnostics (including execution diagnostics), enumeration diagnostics and typed
cardinality rejections. All phases use the same lifecycle control. Preparation and
datatype input visits share their existing allowance; local node/list cardinality
checks read the resulting count without revisiting targets or items.

This explicit adapter requires declaring-scope diagnostic bindings in `FacetContext`.
The declaration-readiness API below checks those dependencies and validates defaults.
Accepting pretyped scalar/list inputs with lexical provenance and activating automatic
schema validation remain separate integration work in the TODO.

### Attribute declaration readiness

`BoundAttributeFacets::check_declaration_readiness` checks an explicit
`AttributeDiagnosticBindings` snapshot against the original schema returned by the
source host. The host supplies compiled diagnostic bindings and their collection/
behavior completeness; equal schema URIs or names do not replace source identity.
Referenced diagnostics require a complete snapshot, matching codes and the expected
engine diagnostic family. An incomplete catalog is pending; a missing binding in a
complete catalog is invalid. Declarations with no diagnostic dependencies do not
wait for unrelated catalog work. This prevents declared diagnostics from silently
falling back to built-in messages.

A literal default runs through `validate_lexical` with its original `@default`
attribute as candidate and source attribution, following the `@values` constant
precedent. No use-site node is fabricated. Absent and authored-empty defaults remain
distinct; the last authored slot supplies the effective default. List defaults
retain token spans. Literal defaults cannot become native node references.

`AttributeDeclarationReadiness` retains the declaration, diagnostic schema, original
default slot and complete phase report. Rejection is invalid regardless of diagnostic
severity; unavailable preparation, stopped validation or cancellation is incomplete.
Checks use the supplied runtime and limits, with control checked again before return.
A ready report covers these declaration prerequisites for this invocation. It neither
activates the schema nor replaces validation when the default is used in a different
consumer context; hosts must recheck when source, bindings or runtime inputs change.

### Retained-node navigation

`dom:parent(node)`, `dom:children(node)`, `dom:descendants(node)` and
`dom:attribute(node, selector)` navigate the retained tree without
copying or re-importing it. Direct calls accept zero or one native node; empty
input returns empty, the document root has no parent, and other types or
multiple items produce `cem.ql.type_error`. Named pipeline steps apply these
helpers to each input node. Returned nodes retain the same owner, identity and
source maps. Imported CEM views navigate the original source arena, including
separate text/CDATA nodes; XPath views retain their existing semantic projection.
The `.children` field remains unchanged; `.parent` is not a new record field.

Descendants exclude the starting node and follow children in depth-first source
order. Attributes and reference targets are separate axes; references have no
structural children. Use `.targets` explicitly to navigate their original nodes.
Pipeline calls such as `nodes.dom:descendants()` preserve input order and
duplicates, without sorting or deduplication.

Attribute strings select an exact **unqualified** name. Qualified attributes use
an exact two-field CEM-QL control record:

```cem-ql
dom:attribute(node, "id")
dom:attribute(node, {namespace: "urn:catalog", name: "id"})
nodes.dom:attribute({namespace: "urn:catalog", name: "id"})
```

The descriptor requires exactly one string in each field. Local names must be
unqualified lexical names; prefixes, wildcards and malformed selectors raise
`cem.ql.type_error`, including with empty node input. Missing attributes return
empty. Matching values remain native attributes with their typed contents and
contracts; use `dom:text(attribute)` for text or `.values` for native contents.
CEM source views retain source namespace declarations; XPath views retain their
existing namespace-node exclusion. No namespace bindings are inferred.

For a retained `<name>` element, select sibling `<id>` elements with ordinary
CEM-QL filtering, without a separate sibling-selector language:

```cem-ql
seq:where(dom:children(dom:parent(node)), fn(sibling) =>
    sibling.kind == "element" && sibling.namespace == node.namespace
        && sibling.name == "id")
```

This returns all matching siblings in source order. A caller chooses explicitly
whether to take the first result. Sorting a sequence does not reparent its nodes.
Host `QueryItemView` implementations receive the active `QueryContextScope`
through the `parent`/`children`/`attributes` capabilities and must preserve restrictions
on returned views. Denied navigation raises fatal `cem.ql.scope_violation`;
unsupported views fail explicitly rather than reporting a missing parent.
The built-in imported/XPath views grant access to their complete retained
document; this does not add subtree grants or a general host access-control
registry. Restricted hosts must supply restricted views instead of unwrapping
them into unrestricted native documents.

Navigation polls cancellation, charges traversal work (including unmatched
attributes) and charges each returned node to the inherited item budget,
discarding partial results on failure. Children are collected from
an iterator under that budget. This is bounded navigation of a materialized
tree, not a claim that the full query pipeline or an incoming AST stream can
execute with no buffering. An ID arriving after a streamed name requires
retention or deferring the row until it is complete.

Processing instructions expose their target as `name` and their data through
`value` (`data` is an alias). For `<?keep inert?>`, these are `keep` and `inert`;
consumers must not split the value to recover the target. Native Rust views also
expose `target`, but CEM-QL's `.target` pipeline step is reserved for reference
resolution; use `node.name` to query the PI target. CEM-ML import resolves
these fields and retains the original lexical PI and ranges. Source values keep
literal line endings; XPath's semantic view applies its existing normalization.

Bare unprefixed native fields use the same generic view when a local variable
or parenthesized expression has an inferred node type. Their result type remains
dynamic, and missing fields return an empty selection. Explicit method calls,
prefixed names and registered pipeline functions keep their existing dispatch.
This also applies to unary-reference values without evaluating authored reference
expressions. The adopted public contract uses `.targets` for one-step edge access,
preserving order, duplicates and reference-node targets. `.target` remains the
legacy HTML ID helper and does not resolve native references. Existing `same_node(a, b)`
compares occurrence identity (the first item of each operand), not target lists.
Compare available target lists by length and positional node identity when needed.

The [adopted query/transport design](../../docs/cem-ql-cem-ml-node-references-design.md#public-query-reference-access-adopted-2026-10-06)
also defines the implemented `reference` refinement of `node` and boolean
`.targets_available` field. Use `r is reference` for a type test; unary construction
carries the refinement through compiler/IR/runtime. Available empty lists report
true; absent stored source targets report false. Neither field access executes a
source expression or claims that a consumer's current resolution is complete.

`cem_ml::ast::reload::ReferenceReloadBundle` exports/reloads a bounded debug CEMB
payload with a versioned passive lexical sidecar and verified source manifest.
Capture attaches to the decoded allocation; `require_lexical` exposes a typed
missing-metadata dependency for AST-only reload. Source bytes can be supplied
later through `ReloadSource::supply_bytes`. Callers supply runtime contexts, policy
and grants afresh. `api::reference_transport::RetainedReferenceSource` retains
source/capture for explicit parsing, bundle reload/export and inert native queries
under a caller's fresh `StandaloneExpressionContext`. `attach_bundle` atomically
attaches missing verified metadata/bytes to the same arena; existing result/session clones keep their previous view. Shared engine
reload admission and CLI `validate`/`check --reload-bundle` are implemented.

`api::reference_lifecycle::ReferenceValidationSession` is an explicit native
schema/namespace validation consumer. It retains input and schema source owners,
with caller-supplied owner defaults and occurrence overrides, schema-derived policy (or explicit
host overrides), and directed crossings. `run` constructs fresh scopes, compiles
the retained schema and consumes the input without caching source targets. Its
report distinguishes readiness, failure, dependencies and attributed diagnostics.

Low-level WASM exposes `beginReferenceValidationSession(source, schema)`;
source indices 0 and 1 name the input and consuming schema. Register additional
owners with `registerReferenceValidationSource`. Set an explicitly ready or pending
context through `setReferenceValidationContext(session, index, ready, bindings)`;
bindings are control JSON `[{"name":"items","valueId":nativeResultHandle}]` over
retained native values, not AST records. Empty bindings with `ready=true` differ
from a pending context. `allowReferenceValidationCrossing` supplies directed host
authority separately. `setReferenceValidationPolicyBounds` overrides bounds while
preserving the schema's unresolved-link policy. Run/dispose through
`runReferenceValidationSession` / `disposeReferenceValidationSession`.
`attachReferenceReloadBundle` refreshes only its source handle's passive view;
prepare a new session to consume attached metadata. Sessions retain owners after
source/result handles are disposed. Source/capture and contexts stay local; only
explicit bundle/CEMV bytes cross heaps.

`startReferenceResourceExecution(session)` freezes requesting contexts, policies
and authority and returns a local execution handle. `advanceReferenceResourceExecution`
returns control JSON `{state: "awaitResources", result: requests}` or
`{state: "finished", result: report}`. Each request carries a local correlation
ID, fragment-free URI, content type hint, requested public part and source map.
The host performs I/O outside WASM and completes the whole pending batch with
`completeReferenceResource(execution, requestId, bytes, contentType, finalUri)`
or `failReferenceResource(execution, requestId, reason)`. Successful completion
returns `{sourceId, sourceIndex}` over the coordinator's original imported owner;
failed import/transport returns `null` and leaves validation incomplete. No
filename-based MIME inference or automatic URL/ID lookup is performed.

After all resources settle, prepare loaded sources through
`setReferenceResourceContext(execution, sourceIndex, ready, bindings)` and grant
specific relationships through `allowReferenceResourceCrossing(execution, from, to)`.
These calls are separate from resource bytes. Resume with `advanceReferenceResourceExecution`;
nested entered controls can yield another batch. Request counts and retained-byte
charges survive rounds, while reference traversal keeps effective request and
destination bounds. Parent context/policy/authority changes reject late completion
and require a new execution. Cancellation and disposal use
`cancelReferenceResourceExecution` / `disposeReferenceResourceExecution`.
Disposing transport sources or the parent handle leaves frozen inputs retained.
`setReferenceValidationOccurrenceContext(session, sourceIndex, valueId, index,
ready, bindings)` overrides an original source occurrence and its owning subtree.
The nearest override wins; pending shadows a ready default.
`clearReferenceValidationOccurrenceContext(session, sourceIndex, valueId, index)`
restores inheritance. Native methods also accept an original AST node ID together
with its explicit registered source. Foreign/constructed handles are rejected.
`setReferenceResourceOccurrenceContext` provides the corresponding loaded-owner
operation after the complete resource batch settles;
`clearReferenceResourceOccurrenceContext` restores its owner/ancestor default.

After supplying destination inputs and any required directed crossings, call
`prepareReferenceResourceNamespaces(execution, sourceIndex)` (native:
`ReferenceResourceExecution::prepare_loaded_names`). This explicitly runs the
shared namespace lifecycle over that loaded owner's original capture and returns
a readiness report. Request and destination limits apply; preparation retries
and resource requests share the execution's cumulative work cap. A bounded pass
that fails before returning its accounting conservatively spends its assigned cap.
Preparation requires a settled resource batch and a live, unchanged execution.
Replacing owner/occurrence inputs discards its current prepared completion;
another explicit preparation is required. Earlier saved views remain immutable.

`completeReferenceResourceWithExports(execution, requestId, bytes, contentType,
finalUri, exports)` supplies an explicit embedding-host public export contract:
control JSON `[{"part":"leaf","select":"..."}]`. Selectors run over the original
imported native document; they must select one valid original schema declaration.
Missing, foreign or ambiguous exports stay incomplete. This separate host API
creates no directed crossing or package replacement authority; no ID scan is used.

`prepareReferenceResourceQuerySnapshot(execution, sourceIndex)` retains saved
names from that resource execution, without a second import or reference evaluation.
Index 0 uses the latest input namespace snapshot; loaded indices retain the
latest explicitly prepared completion, or the captured ready forest from import
before preparation. Pending names remain excluded; creating a snapshot does not
evaluate namespace properties. Original
schema/additional registered owners without a saved completion are rejected.
Saved views and results survive parent/execution disposal and later input changes.
Validation installs the current loaded-owner completed-name views for selected
schema compilation, restoring the previous views afterward. A bounded scan of
each selected declaration's owning subtree checks QName and native namespace
readiness before compiling declarations or evaluating their selectors. Pending
names keep the governed region incomplete; unrelated pending roots do not block
a ready selected schema. Request and destination work limits both apply to a
compilation-wide metadata budget, separate from reference traversal accounting.
Replacing destination inputs requires fresh explicit namespace preparation;
saved query views remain immutable.

Embedding validation sessions supply foreign-owner completions through
`SchemaValidationSessionInputs::loaded_namespace_completions`. Declaration hosts
with pending lexical metadata opt into readiness checks through
`SchemaDeclarationHost::declaration_name_metadata_required`, supplying current
expanded names and namespace-property readiness without evaluation. Fixed legacy
AST hosts retain their existing complete-name path. Original owner/node handles
are preserved; the name scan does not follow reference target or context edges.

`prepareReferenceQuerySnapshot(session)` creates an immutable local namespace
execution view. `inspectReferenceQuerySnapshot` returns its readiness report;
`queryReferenceQuerySnapshot(snapshot, expression, queryUri)` returns a native
result handle. Its `input` is the selected ready root sequence. Pending roots are
excluded; an empty ready sequence never falls back to the original document.
Readiness covers namespace preparation, independently of full schema validation.
Authored descendant references remain inert. Snapshots over one arena can retain
different completed names and survive later parent changes. Results retain native
views after `disposeReferenceQuerySnapshot`; `.source` exposes authored nodes.
Original `queryReferenceSource` continues to inspect the original names.

The resource and snapshot examples in
[`tests/reference-transport-wasm.mjs`](tests/reference-transport-wasm.mjs) execute
in both the main heap and an independent worker. Run all maintained reference
consumer contracts with `yarn nx run cem_ql:test:reference-consumers` (also wired
into CI). See the [consumer matrix](../../docs/reference-consumer-verification.md).

WASM exports `parseReferenceSource`, `importReferenceReloadBundle`,
`exportReferenceReloadBundle`, `inspectReferenceSource`, `queryReferenceSource`
and `disposeReferenceSource`. Source handles and query result handles stay local;
`exportNativeValueArtifact` exports guarded CEMV bytes, and
`disposeNativeValueArtifact` releases results. Results retain the source capture
after source-handle disposal. New failures throw JSON strings containing `code`,
`kind`, `message`, and applicable `sourceMap`/`sourceUri`; existing compatibility
exports keep their string errors. Worker clients transfer explicit bytes, not
AST records, handles or live contexts. These entry points perform inert queries;
schema/namespace lifecycle consumer registration remains separate.

CEMV preserves materialized acyclic values, including constructed references.
`eval::portable::export_values` and `export_values_with_control` now return typed,
attributed outcomes for unsupported executable source references and graphs,
invalid values, limits and cancellation. Source references reject even with a
stored empty/nonempty target list, including when reached through another native
node. Empty constructed references remain valid. Failed query streams cannot
export partial items successfully. Existing string/control encoding APIs remain
available with the same guards; control failures carry source maps and explicit
unsupported-source/graph codes. CEMB separately preserves supported single-arena
cycles. No transport persists live contexts or resumable pending sessions. See the
[preservation matrix](../../docs/cem-ql-cem-ml-node-references-design.md#graph-export-and-native-transport-adopted-2026-10-06).

`api::native_capability_session::NativeCapabilitySession` is the transient
consumer channel for source-backed capability inputs. It shares lifecycle
registration, directed grants and request/destination bounds with the element
consumer; source selection reference chains resolve together while target
descendants stay authored. Selected namespace names complete per execution.
`evaluate` and label `render` retain the original native input; `export` names
the guarded CEMV presentation boundary. WASM provides
`prepareNativeCapabilitySession`, `nativeCapabilitySessionLength`,
`exportNativeCapabilityView`, `renderNativeCapabilityTemplate` and
`disposeNativeCapabilitySession`. View export returns only root-count/artifact
control metadata; take the opaque bytes with `takeRenderValueArtifact`.
Handles remain heap-local and are never a serialized resume authority.
Native and WASM cases are included in `test:reference-consumers`.

`suggestions::SuggestionsPlan` adapts retained canonical CEM, HTML data or HTML
option/group sources once per immutable capability session. Scalar `value`,
`label` and image `alt` attributes must already be materialized; descendant
content references retain their authored source handles. Source-order native
views keep duplicate values distinct and apply strict local contains/prefix or
upstream external/none filtering. Full default case folding uses the pinned
[Unicode 17.0.0 data](src/suggestions/CaseFolding-17.0.0.txt) under its included
[Unicode license](src/suggestions/UNICODE-LICENSE.txt), without value rewriting
or implicit Unicode normalization. WASM builds and the packaged cem-elements
runtime include that notice. `evaluate_suggestions`, `render_suggestion`
and `render_suggestion_group` bind the derived native view and original content
inside each execution frame. WASM exposes `prepareNativeSuggestions`,
`exportNativeSuggestionsView` and `renderNativeSuggestionTemplate`; preparation
returns only scalar counts, identity and attributed diagnostics. Explicit scalar
projections export CEMV, while a whole live view fails rather than dropping its
source/content edges. These APIs do not publish a public suggestions capability.

`NativeCapabilitySession::datalist` caches a scalar native datalist projection;
`bind_datalist_frame` binds it to the reserved `suggestions` slice without
exporting its original source nodes. The view exposes ordered option value/label
attributes and non-owning `source` links, retaining duplicate-value identities
and authored descendant references. Groups and captured label templates are
rejected; disabled/hidden rows are omitted and empty values emit attributed
warnings. NativeDatalistConfig accepts only an empty explicit control object,
rejecting query/filter/label/selection controls. The view has no commit state and
cannot be exported as CEMV; explicit scalar extraction remains available.
This Rust consumer is preparation for the [native profile](../../docs/cem-suggestions-native-profiles-design.md).
Worker/WASM transport, provider list claims and declarative/browser integration
remain delivery tasks; it does not yet enable a browser datalist profile.

`data:node_key(node)` and `data:line_number(node)` read source metadata from
either an imported CEM node or a retained XPath node. They accept an optional
single native node; empty input returns empty, and other types/cardinalities
raise `cem.ql.type_error`. Keys are opaque versioned strings, and lines are
one-based integers. Missing original provenance returns empty.

```cem-ql
for row in data:read(source, "csv").root.children.children {
    (data:node_key(row), data:line_number(row))
}
```

The matching XPath functions are `Q{urn:cem:source}node-key($node)` and
`Q{urn:cem:source}line-number($node)`. Both languages use the same native tree
accessors. Identical source identity, bytes and import/projection profile keep
the key across rerenders; edits or profile changes invalidate it. Keys do not
change XPath node identity and do not track nodes across edits. Existing
`data:read(...).id` values remain source-derived selection keys across unchanged
rerenders. Runtime node identity distinguishes independently imported CEM
document owners even when their source bytes and selection keys match;
Selections and copied handles of one retained owner preserve its runtime identity.
Repeated reads within one query may reuse its memoized import result.
Parser-only lifecycle/compatibility imports without
original input bytes have no source key, though retained locations remain usable.

For CSV, `data:read(source, "text/csv;header=present")` explicitly selects named
columns; `header=absent` retains every row as an array. XPath's
`Q{urn:cem:import}parse-csv($text, map {'header':'present'})` selects the same
header mapping; its one-argument profile continues to default to `absent`.
All CSV parsing and header mapping are owned by CEM-ML import.

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
parser's accepted input. The separate standard string-import profile now backs
XPath `json-to-xml` and native `data:parse` below; it also handles unmatched
surrogates according to its escape option. This existing reader projection
keeps its stricter input profile. Schema-typed output, custom fallback functions
and liberal JSON are not added.

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

### Native string parsing and URI metadata

Tier B `data:parse(source, format, options?)` exposes the same CEM-ML string
import profiles as XPath `parse-xml`, `json-to-xml`, and the CSV/YAML import
functions. It returns a native CEM document directly, with ordinary
`children`/`attributes` fields and native XPath interoperability. Every parse
creates a new retained owner. `data:read` and its cached report remain unchanged.

```cem-ql
{ let document = data:parse(source, "csv", {header: "present"});
  document.children.children }
```

Formats are `xml`, `json`, `yaml`, `csv`, or their media types. JSON selects the
native XML-shaped projection; options are `duplicates` (`retain`, `use-first`,
`reject`) and boolean `escape`. CSV defaults to `header: "absent"`. All formats
accept a string `"base-uri"` option; no network access occurs. Invalid options
fail explicitly. Format and option resolution stays inside CEM-ML import.

`data:base_uri(node)` and `data:document_uri(node)` read retained URI metadata,
returning an optional URI atom. Inherited XML base URI is respected; parsed
strings have no document URI and no base URI unless supplied. Like
`data:node_key` / `data:line_number`, these functions accept zero or one native
node and reject ordinary records. Query values never replace the native owner.

Recover malformed input with `try { data:parse(source, "xml") } catch (code,
message) { message }`. Malformed and duplicate-key diagnostics retain typed
import names and parser diagnostic metadata. Resource limits, unsupported
capabilities and cancellation remain fatal. Source limits remain 32 KiB,
depth 64 and 4096 values/events. See the
[parity audit and exact error contract](../../docs/xpath-cem-ql-viewer-parity.md).
The [paired demo's detailed use cases](../cem-elements/demo/xpath-functions.html#cem-ql-use-cases)
cover imports, URI/provenance, predicates and native node selection.

### Retained document inspection

Tier B `cemml:inspect(document)` returns an inert, tabular CEM-ML display string
through the shared typed inspection projection and writer. Pass a retained CEM
document or an XPath document backed by that same owner. Inspection visits the
original CEM source arena, preserving separate whitespace/CDATA, PI targets and
data, source ranges and namespace provenance. It does not re-import source or
traverse an external parser tree. XML, JSON, YAML and CSV all use this same path.

```cem-ml
{cem-data @name=document @select=source @type="{$format}"}
{pre | {code | {$cemml:inspect(document.root)}}}
```

CEMT emits the string as escaped text. It contains no terminal coloring and is
not executable template content. Empty input returns an empty sequence;
strings, records, multiple items and non-document nodes produce
`cem.ql.type_error`. `cemml:format` remains the separate source-formatting
surface; its current string pass-through is not a structural inspection API.

Inspection charges source nodes against the enclosing query's item budget.
The effective scope's `memory_bytes` bounds source payload and final display
bytes, and operation-control memory permits also respect ancestor limits and
existing charges. These are payload limits, not a bound on every temporary
allocation in the shared formatter. Child scopes can lower environment limits.
Cancellation is cooperative: checks run during arena preflight and surround the
existing synchronous projection/writer. Cancellation, `cem.ql.inspect_limit`,
control exhaustion and `cem.ql.inspect_writer` failures discard the whole
result and cannot be converted to partial output by `try`/`catch`.

### Native DOM chains

Use `dom::chain(node).parent().children().find(|n| n.name() == "id").text()`
for fluent native navigation with empty-result propagation. The
[chain API guide](../../docs/cem-ql-chains.md) documents all methods, Rust-style
closures, immutable sorting, resource limits and transport behavior. Try the
[DOM functions samples](../cem-elements/demo/functions/dom.html), with their
own **CEM Elements/CEM-QL DOM Functions** Storybook group.

String values also support Rust-style `.split("/").nth(6)` for the seventh
segment of a URL. `nth` is zero-based in both chains and `seq:nth`; invalid
indices are errors and out-of-range selection is empty. The chain selection
names are `next`, `nth`, `rev` and `rfind`. These immutable query chains retain
CEM's optional-result representation rather than exposing Rust `Option` values.
Try the [interactive URL example](../cem-elements/demo/functions/str.html).

### Collections and presentation dispatch

Two reusable Tier B collection operations preserve original items and native
identities:

- `seq:group_by(items, keyFn)`: first-occurrence groups with `key` and
  `items`; keys are empty or one atomic value.
- `seq:sorted(items, keyFn, direction?, mode?)`: stable sorting, default
  `ascending` and `text`; `descending` and finite `number` comparison are
  supported. Missing/nonnumeric keys stay last in either direction.

Both evaluate the key once per item and return the original native nodes with
their source owner, identity and provenance. Group order and member order follow
first occurrence. Group keys use CEM atomic identity: an empty key, an empty
string, integer `1` and string `"1"` are distinct. Grouping is over the supplied
sequence; callers group each parent's children separately for sibling groups.

Text sorting compares string values without locale collation. A present empty
string sorts before nonempty strings ascending and after them descending; a
missing key stays last. Number mode parses finite numbers; missing, empty,
malformed and nonfinite keys share the last position, keeping their input order.
Equal valid keys also keep input order in both directions. A failing key or
exhausted scope budget returns no partial collection. Host policy supplies the
limits; child scopes may constrain them further. The helpers consume common
CEM nodes under the existing query budgets.

The [XML-VIEW-2 audit](../cem-elements/docs/xml-viewer-migration.md#xml-view-2--grouping-and-stable-sorting)
maps this bounded AC-QO-6 subset and its native tests to the viewer contract.

CEMT `template @match='predicate' @mode=inspect @priority=10` declares a
presentation rule. `apply-templates @select=items @mode=inspect` binds each
native item as `node`, forwards `@with:...` parameters, and runs the first
matching rule. Priority is a static signed integer (default 0); higher wins,
then local over imported, then later declaration. Modes are static on rules.
Unmatched values emit nothing. Parameter scopes are restored and recursion is
bounded. Imported match rules participate without copying their bodies into
the consuming module.

The [seven-case demo](../cem-elements/demo/data-table.html) authors repeated-row
discovery, headings, cells, tree/table rendering and sort-key selection in
[CEMT](../cem-elements/demo/data-table-view.cemt). There is no Rust table-view
API. An [imported extension](../cem-elements/demo/data-table-aspects.cemt)
changes notes to a tree and an IP-filter record to a local preview form.
The [cell override lessons](../cem-elements/demo/cell-overrides.html) import
the viewer: its `cell` mode selects the original source node, and an inline
name match extracts an ID from the sibling URL with `.text().split("/").nth(6)`
to render Pokémon images beside their names. Pokémon JSON and stock XML are
local files with separate source previews.
A separate `inspect` predicate replaces zero stock with a warning. Unmatched
cells retain the imported presentation; source values and sort keys stay intact.
Two independent XSLT cases use the same native import and rendering lifecycle
through explicit scalar parameters, including an imported presentation module.
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

## Template whitespace

CEM templates can opt a constructor or control node into an inherited, lexical
whitespace policy with `@cem:whitespace=layout` or `@cem:whitespace=preserve`:

```cem
{textarea @cem:whitespace=layout |
    {$text}
}
```

`layout` suppresses source trivia made entirely of ASCII spaces, tabs, CR and LF
when the run includes a line break. As in unannotated templates, opening trivia
before the first body item is skipped. Inline spaces after body items, Unicode
spacing, nonempty literal text runs, triple-backtick content and evaluated
values remain exact. This is not a trim/dedent operation on the rendered value.
The example emits exactly `text`, including any whitespace in that value.

`preserve` keeps all body trivia after an explicit `|`, including opening
spaces/indentation. Nested nodes inherit the selected policy and may override
it; siblings outside the scope retain their previous policy. Selection is
static at compilation, including named templates and imported modules, not
inherited from the runtime caller. Without an annotation, existing whitespace
behavior is unchanged. Both controls are consumed before output attributes are
compiled. Missing, dynamic, unknown or duplicate values produce
`cem.ql.render.whitespace_policy_invalid` at the authored attribute range.

Suppressed layout retains byte provenance as zero-width text in portable
template IR. The input tokenizer/source is unchanged. The browser does not
trim control values, and XSLT's separate `xml:space` behavior is unchanged.
See the [native whitespace fixtures](tests/template_whitespace.rs) and
[DOM-merge demo](../cem-elements/demo/dom-merge.html).

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
parsing-function semantics are owned by the XSLT compatibility layer.

## Native result construction

`result-document`, `result-element`, `result-attribute` and `result-sequence`
select an explicit native output path. For example:

```cem
{result-document |
  {result-element @name=p |
    {result-sequence @select='(1, 2)'}{$ "x"}
  }
}
```

This produces `<p>1 2x</p>`. Ordinary `{$ (1, 2)}` still produces `12`.
Native CEM/XPath selections remain nodes until construction; there is no markup
reparse or document-record conversion. Arrays flatten, text merges, attributes
retain sequence order, duplicates use the last expanded-name value, and
unsupported maps/functions/records fail explicitly. Attributes use simple-content
atomization. Calls and buffered recovery carry pending values until their parent
is built; a pending value without a parent constructor is an error.

The typed artifact variant makes the capability explicit to loaders. Language
lowering supplies error-name and origin metadata; generic CEMT has no implicit
XSLT error contract. Expanded-name output uses optional `qualified_name` metadata
in the native render plan; when present, `namespace` is the URI. Existing CEMT
name/namespace behavior stays unchanged when the metadata is absent. The
[bounded profile](../../docs/xslt-runtime-lowering.md#native-output-construction)
describes namespaces, provenance, limits and output integration.

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

## Retained schema declaration references

`schema_references::CemQlSchemaDeclarationHost` is an explicit native consumer
for references in schema `{elements}`, `{attributes}`, `{behaviors}`,
`{diagnostics}`, `{constraints}` and `{field-contracts}` collections. Register retained source and library trees with
their effective policies, supply a
`StandaloneExpressionContext` for each evaluating scope, and grant directed
scope crossings before calling `compile`. Missing contexts report Pending;
empty selections resolve successfully. `set_context` allows a later lifecycle
snapshot to complete the same retained source without rewriting references.

`eval::RetainedCemNode` and `eval::retained_cem_node` expose checked original
source handles from imported CEM views. The compiler shares
`RetainedCemTree::ast_owner`, preserves declaring schema aliases, and uses the
shared bounded resolver for nested references. Effective child scopes can be
supplied through `assign_subtree_scope`; runtime scope handles belong to their
host and add no authored IDs.

`register_scope` creates an explicit relationship boundary. For a lexical-only
change, `register_lexical_scope(parent, context, policy)` retains a distinct
context/artifact snapshot inside the parent's boundary and original tree.
Supply its complete runtime context and effective policy explicitly; missing
inputs stay pending. Source assignment remains explicit through the subtree
or sibling handoff methods. Crossing grants use relationship boundary identity,
so lexical snapshots share their boundary's directed grants without granting
the reverse direction. Context replacement invalidates only that snapshot's
artifacts; request and effective lexical-scope limits remain independently active.

For parser-captured occurrences, construct a tree with
`RetainedCemTree::from_shared(captured.document().clone(), ...)`, register its
relationship scopes, and call `attach_captured_lexical_scopes(&captured, prepare)`
at the consumer lifecycle stage. `prepare` receives the original typed source,
saved namespace/schema snapshot and nearest existing scope. It supplies an
optional runtime context and effective reference policy. Missing inputs or
unready schema selection must return `None` context; source capture cannot supply
runtime readiness. Returned node/scope pairs support later `set_context` updates.

Attachment checks the original owner and rejects already assigned occurrences
before calling preparation. It prepares every occurrence before changing the
host, preserves existing subtree/sibling relationship boundaries, and creates
no crossing grants or compiled selections. Use `set_context` to refresh attached
inputs. XML callers can obtain the same captured owner and semantic projection
from `import_xml_ast_with_lexical_scopes`, then use this handoff after preparing
schema readiness and runtime inputs. Automatic engine/package preparation remains
separate work.

For a ready per-execution namespace view, call
`attach_completed_namespace_lexical_scopes(&completion, prepare)`. Only captured
occurrences in that selected forest are attached. Registration, repeat-assignment
and **all captured pending prefix** checks finish before any callback or mutation;
an unused pending prefix still blocks namespace-aware context preparation. An
incomplete attempt is retryable. A declaration's own selector can be selected
separately and use its original pre-declaration bindings.

The callback receives the original source, a `NamespaceLexicalSnapshot` and nearest
relationship scope. Use `snapshot.namespace_uri(prefix)` for completed bindings,
`snapshot.original()` for saved scalar namespace/schema metadata, and
`snapshot.completed_bindings()` for original typed declaration provenance. Return
runtime inputs and `ReferenceScopePolicyOverrides`; `None` context stays pending.
Contexts can retain the matching `NamespaceQueryTree`, preserving completed native
navigation and authored `source` access independently for each execution. The
handoff keeps original captured-name metadata, local policy provenance and directed
grants, and performs no reference evaluation or scope activation.

The shared native bridge accepts `QuerySourceOwner::NamespaceCompleted` through
query preparation or `cem_ml::query::run_query_with_source_owner`. It binds the
matching completion's selected roots through `NamespaceQueryTree`, preserving
root order, empty forests, native source handles and authored `source` inspection.
It exposes ready name metadata without evaluating captured expressions or
preparing their lexical contexts. Supply a completion from the explicit
`with_namespace_lifecycle` invocation when dependency coordination is required;
query ingress itself stays passive.

`assign_following_scope(tree, boundary, scope)` records a caller-completed
sibling-position switch using the original retained boundary node. Earlier
siblings and the boundary keep their scope; following siblings and their
subtrees inherit the supplied default. Inner switches end with their containing
scope, and explicit child subtree mappings take precedence. A repeated identical
handoff succeeds; a conflicting repeat is rejected. This handoff does not parse
scope syntax, evaluate expressions, scan authored IDs or wire parser frames
into the runtime. Context readiness and crossing grants remain explicit.

Package loading keeps source references pending unless a runtime compiler is
explicitly installed. The [native bridge](../cem_ml_transform_cem_ql/README.md#schema-package-lifecycle-compilation)
connects this host to the engine's package compilation stage before coordinated
publication. Remaining schema consumers are tracked in
[todo.md](../../docs/todo.md#5-integrate-schema-validation-and-construct-reuse).

`CemQlSchemaDeclarationHost::validate_input` explicitly validates structural
input references with the consuming schema model. It reuses the same retained
owners, runtime contexts and scope grants as declaration compilation. Ordered
selected subtrees share one bounded traversal per authored reference; nested
containment cycles remain incomplete. Inspect both `complete` and `failed` in
the result. Pending child selections defer field contracts while independent
attribute errors remain visible. Optional engine runtime integration supplies the
same lifecycle stage before retained native behavior checks.

Node-valued attributes require an explicit schema `node` contract. Native reference
slots and general expressions such as `@target={items}` consume retained targets
under the same scope/grant and cardinality rules. General expressions use the
explicit `SchemaDeclarationHost::evaluate_input_expression` hook, delegated by
`InputReferenceHost`; custom hosts without it report pending consumption. The native
host preserves query errors and rejects scalar results. Composite `value_nodes`
share one traversal in authored order, including cumulative destination budgets.

Consumed attribute `.value` and local-name behavior conveniences expose native
nodes within captured selected subtrees. Original source handles retain authored
expressions and descendant references. Query access does not expand those links,
follow outside parents/contexts/targets, copy source arenas or generate DOM IDs.
The standalone ML reference-slot API remains restricted to one authored reference;
placement validation supplies the general-expression/composite consumer.


Explicit XML native attribute slots use core `expression-attributes` metadata;
unlisted XML brace strings stay literal. Existing host-bound child schema and
namespace overrides and opening literal block preludes are documented in
[the CEM-ML syntax guide](../../docs/cem-ml-syntax.md). Runtime inputs, directed
grants and consumption remain separate from source syntax and import.

## Native element relationship lifecycle

`api::element_references::ElementReferenceExecution::prepare` stages passive source
selections into native template slices over original retained owners/capture. It
requires an explicit requesting source, per-source readiness/policies and directed
grants, and supplies fresh invocation contexts. `.project` applies the existing
bounded relationship-to-ID consumer to one produced forest without changing source
references. All slots share the request/destination budget.

`renderTemplateWithNativeValues` and `renderXsltComponentWithNativeValues` accept
optional producer instance identity and lifecycle metadata as their final two
arguments. Metadata names heap-local retained source handles; it contains no AST
records or context IDs. Supplying metadata requires an instance identity. Host
browser/worker callers import explicitly selected debug CEMB bundles and dispose
local handles after the invocation. Pending or invalid consumption returns
`referenceProjectionComplete: false` with no replacement nodes; warning/ignore
policies retain their actual diagnostics and never create a successful relationship.
See the [consumer design](../../docs/cem-element-reference-ids-design.md) and
[verification matrix](../../docs/reference-consumer-verification.md).


### Granted producer placements

`ElementReferenceExecution::prepare_with_placements` accepts
`ElementPlacementInputs`: passive selections over original retained sources,
opaque admission tokens, producer/path/revision/ID reservations, property-specific
requester grants and committed producer revisions. Native callers with evaluated
nodes can supply `ElementPlacementSnapshot` directly to
`project_element_reference_ids_with_host_and_placements`. Both APIs require the
independent directed source-scope grants and share request/destination bounds.
Local placement wins; local ambiguity cannot be repaired by a foreign grant.

A host-owned `prepared_transaction` can admit uncommitted placements only for
its declared participants and revisions. The embedding coordinator must publish
the entire group atomically. Ordinary publication cannot consume this snapshot.
`.project_with_placements()` returns the new DOM plan and `ElementPlacementUse`
metadata; it changes no source AST or foreign producer. Uses identify the token,
producer, revision, reserved ID, consumer path/property and optional transaction.

The explicit WASM lifecycle metadata adds optional `placements` with
`admissions`, `grants`, `committedRevisions` and `preparedTransaction`.
Admission selectors must select one original retained element, never a constructed
record. Native output adds `elementPlacementUses` beside the terminal DOM plan.
The browser bridge maps consumer paths to exported render identities; DOM elements
remain in `CemElementPlacementCoordinator`. `CemSsrPlacementCoordinator` stages
and commits terminal export plans, then emits authority-free resume hints.
Neither an ID, source bundle, serialized hint nor template can issue host grants.
