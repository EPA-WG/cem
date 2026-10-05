# General datatype compilation proposal

Status: temporary design draft. Registered schema-owned implementations,
native `@rule` binding, separate validation/conversion roles, duplicate-name
rejection, intersecting scalar restrictions and ordered typed list conversion
results, rejection of list declaration `values` and explicit rule acceptance
with diagnostics, empty lists unless constrained and a dedicated `node` kind are
adopted. Node rules validate complete target sequences, and candidate requirements
are per registered capability. Derived declarations inherit an omitted kind from
a resolved base, with explicit list kind for item-base semantics. Exact signatures and
remaining kind contracts need decisions. General datatype compilation must be
designed before enabling native attribute `@type` consumption.
This draft does not adopt a new executable grammar or enable datatype references.

Workstream boundary: general datatype execution, conversion, equality and enumeration
are separate from core reference adoption. Preserve the adopted contracts here, but
complete reference binding and readiness boundaries independently. The specific
executable native attribute `@type` consumer stays guarded until this compiler is
ready; this is not a blocker for other reference consumers.

## Problem and accepted boundary

The current schema document compiler constructs attributes and their local
facets. It does not construct a general executable model for schema `{types}`.
Several validators identify built-in types by the local part of a type name.
The separate native attribute conversion contract already supports a base model,
intersecting restrictions, and one final converted value.

Attribute `@type={#datatype}` adopts an explicit reference selecting one named
`{type}` declaration. The source declaration, its lexical schema and its owner
must remain retained. Compilation controls evaluation context and timing.
Source readiness guarding keeps this slot pending until its consumer exists;
malformed slots are errors. Literal type lookup remains compatible during the
transition. An unknown type must not become an untyped/string fallback.

The shipped vocabulary contains `lexical`, `scalar`, `list`, `grammar` and
`reference` kinds. Some `rule` values are names such as `local-name`; others are
prose such as `signed decimal integer`. List declarations use `base` for an item
shape, for example `name-list` with `base=identifier`. Those existing forms must
be inventoried and tested before claiming a general datatype compiler.

## Proposed compiled representation

Use a compiled datatype descriptor distinct from the source AST. It holds:

- The original retained declaration handle and declaring lexical bindings.
- A consumer-owned internal identity and public schema/type name when declared.
  Storage addresses are internal; contexts and scope roots need no authored ID.
- Its representation/conversion contract and its resolved base or item contract.
- Ordered restriction records with their original source attribution.
- Readiness, dependency issues and diagnostics, separately from available partial
  descriptors. Warning or ignored links remain incomplete.

Do not clone declarations into the consuming source document or flatten a
restriction chain into one overwritten facet set. Compiled scalar metadata may
be reused; reference and source identity remain original handles. Distinguish
conversion of scalar values from retained node validation.

## Compilation and reference evaluation

1. Collect declarations and lexical environments before binding dependencies.
   Existing element/attribute collection precedence remains compatible. Reject
   distinct datatype declarations claiming the same name in one lexical schema;
   repeated selection of the same original declaration is valid.
2. Bind literal local/QName names through the declaration's own schema and
   declared exports. Native selection uses the shared lifecycle resolver and
   requires exactly one named `{type}` target. Do not infer URL lookup or grants.
3. Build a bounded dependency graph without recursive source expansion. Bases,
   item contracts and native chains share the enclosing request and destination
   budgets, active link identities and explicit scope-crossing rules.
4. Classify each descriptor and check that all authored fields have a supported
   meaning. Never silently ignore an unsupported kind, rule or native edge.
5. Compose conversion and restrictions, check defaults when their effective type
   is ready, and publish schema/converters/artifacts together through existing
   package readiness rules. Keep the previous complete package active meanwhile.

Both literal lookup and native selection should eventually produce the same
compiled datatype descriptor. The legacy literal path remains explicit until
parity fixtures prove migration safe. A user declaration named `integer` must
not receive an intrinsic implementation merely because its local name matches.

## Datatype kind contracts and remaining work

| Kind | Meaning | Remaining design work |
| --- | --- | --- |
| `scalar` | One value with an explicit primitive conversion/validation contract and restrictions | Built-in registry binding, normalization and custom implementations |
| `lexical` | A value checked by a declared lexical predicate | Executable `rule` binding and compatibility with shipped descriptions |
| `list` | Ordered typed values satisfying an item datatype contract, with authored lexical source retained; declaration `values` rejected | Registered tokenization, cardinality admission and concrete adapters |
| `grammar` | A value checked by a registered grammar consumer; source prose is descriptive | Concrete registered parser/validation adapters and parity |
| `reference` | A symbolic-reference value contract such as a QName | Separate lexical validation from consumer-requested dereferencing; do not imply URL or AST link evaluation |
| `node` | Original retained nodes checked by registered native validation rules; lexical facets rejected | Rule invocation unit, typed sequence signature and metamodel admission |

For inherited scalar restrictions, the adopted composition is intersection: the base
and every restriction must hold, including locally authored attribute facets.
This agrees with `AttributeValueContract.restrictions`. Conversion occurs once;
restrictions cannot change the final representation. Type-dependent checks wait
while dependencies are pending; independent malformed rules or facet syntax can
still be reported. A derived restriction may narrow a base contract, but cannot
widen it or substitute a different conversion result. An empty intersection
accepts no values; statically provable contradictions are compilation errors,
while arbitrary rule satisfiability is not assumed to be decidable.

## Adopted executable rule approach

Bind executable semantics to explicitly registered schema-owned implementations,
using native predicates/parsers and schema behavior capabilities. Shipped prose
remains descriptive metadata attached to explicitly registered built-in contracts.
Neither a matching description nor a same-name vendor type acquires an intrinsic
implementation. Registration must retain its declaring identity and authorized
owner; unknown or unavailable implementations cannot activate a datatype.

Do not introduce an executable grammar language for `@rule`. Registered grammar
consumers may reuse existing content-model parsers. A source rule description is
not reparsed as executable code, and imported document formats continue to resolve
only at the retained CEM AST import boundary.

Registration supplies a capability, not a scope-crossing grant. Native binding
must obey the shared reference resolver's context, budget and permission rules.
Compilation checks the implementation's datatype compatibility and retains its
original declaration; rule execution happens when a consumer validates or
converts a value. Exact typed input/result signatures still need specification.

## Adopted custom binding in source

An explicit native `@rule={#behavior}` selects exactly one named `{behavior}`
declaration with a registered datatype-compatible implementation. Literal `@rule`
values keep their descriptive role. The explicit constructor distinguishes
executable binding from prose without adding a second field or making strings
implicit references. A separate `@behavior` attribute is not adopted.

The compiler must retain the selected behavior's original owner, lexical schema,
signature and source maps. Selection, intermediate references and related datatype
dependencies share the enclosing compilation traversal. Missing context, denied
crossings and bounded/cyclic chains keep the descriptor pending or unresolved
according to effective scope policy; malformed targets and complete zero/multiple
selections are schema errors. Selecting a behavior does not import its entire
schema or authorize arbitrary code execution.

This binding is designed but not enabled. The metamodel admission contract,
registered implementation signature and lifecycle adapter must be specified
before implementation. Generic CEM-ML parsing still only retains reference nodes;
the datatype consumer owns the behavior-target and signature checks.

## Adopted validation and conversion roles

`@rule` validates the consumer-supplied value and reports
acceptance or diagnostics. Conversion/normalization is a separate registered
datatype capability that produces one canonical value when the consumer explicitly
requests conversion. Rules and inherited/local restrictions check that value;
they cannot replace it. Validation alone does not invoke conversion implicitly.
This preserves the existing conversion-once restriction contract without limiting
which datatype kinds can obtain registered implementations.

A rule cannot return a replacement typed value or mutate an authored source node.
A conversion implementation needs its own typed signature and declared output
representation. Canonical values must still satisfy all inherited and local
restrictions. Conversion failure, invalid input, unavailable execution and
malformed implementation results remain distinct consumer outcomes.

The following are proposed semantic input roles, not enabled function names or
metamodel `source` literals:

| Role | Proposed typed input | Purpose |
| --- | --- | --- |
| Candidate | Explicit retained native node | Original input/source provenance and allowed candidate navigation |
| Datatype | Original retained `{type}` declaration | Declaring identity and source contract without a copied record |
| Value | Representation declared by the datatype kind: scalar, ordered items or retained nodes | The consumer's current value, with authored source still retained |
| Context | Runtime-supplied evaluation frame | Evaluation time/scope and access grants, without an authored root ID |

The adapter must reject incompatible signatures before rule execution. Existing
legacy `object` candidates do not substitute for explicit native node parameters.
The result protocol must distinguish invalid input from pending/unavailable
execution and malformed implementation results. Scalar extraction, normalization
and node resolution remain explicit consumer operations. No role permits ambient
URL handling or wider source navigation than the consumer grants.

Exact input/result bindings follow the selected validation/conversion contract.
List representation and kind-specific grammar/reference behavior remain
separate explicit TODO items.

## Adopted datatype registry collisions and customization boundary

Bind names in the declaring lexical schema, using its local declarations and
explicit namespace aliases/exports. An internal scope handle is given by the
compiler/runtime; it does not require an authored root ID. Original declaration
owners remain part of dependency and implementation identity. Matching a local
name never substitutes for an explicitly registered implementation.

Two distinct declarations claiming the same scoped datatype name are a
compilation error, with both original source locations available for diagnostics.
Reuse of the same original declaration from multiple attribute sites remains
valid. Equal local names in separate lexical scopes/namespaces are independent
and require the normal explicit boundary contracts for cross-scope selection.
Registered package replacement retains its origin/grant and coordinated
activation rules; ordinary duplicate declarations do not authorize replacement.

Incremental customization can derive a separately named type and add intersecting
restrictions to its base. Automatic later-declaration replacement is not adopted:
source order must not silently discard restrictions or change a bound datatype.
Native selection retains the original declaration identity and is not redirected
by another declaration with the same name.

Explicit declaration overrides are deferred. Before admitting them, specify the
replaced declaration identity, authorization, dependency rebinding and coordinated
activation contract. This does not adopt override syntax or a replacement grant
at the individual datatype level. Keep this work separate from the first general
registry; existing package replacement is not an implicit declaration override.

## Adopted list value representation

The shipped `name-list` and `wildcard-name-list` validators consume whitespace-
separated lexical strings and reject an empty value. Their `base` describes each
item, rather than scalar inheritance. Preserve those validation contracts during
migration; do not silently treat commas as separators or erase duplicate items.

The canonical compiled representation is an ordered sequence of typed items for
explicit list conversion, retaining the authored lexical source separately.
A registered list conversion capability specifies tokenization and canonical
serialization; the shipped name lists use their existing whitespace semantics.
Validation alone checks the supplied representation without requesting conversion
implicitly. List rules can examine individual items and the complete sequence.

Canonical lexical output is an explicit serialization of the typed sequence;
it does not replace that sequence or overwrite authored source. Each item carries
its datatype representation, with item diagnostics attributed to the retained
input where available. Preserve order and duplicates through conversion. A list
containing native nodes must retain their original owner/handle identity rather
than copy nodes into scalar records. This contract does not itself admit native
node item types or introduce a new list syntax.

The representation is adopted but conversion remains disabled pending registered
signatures, item contracts and migration fixtures. Existing scalar conversion
APIs must not silently reinterpret a string result as a typed sequence.

## Adopted list restrictions

Reject `values` on a `{type @kind=list}` datatype declaration. Item restrictions
belong to the item datatype selected by `base`; a registered list rule validates
the complete ordered sequence. This avoids giving one lexical field two meanings.
Existing attribute-local `values` behavior remains a separate compatibility
surface and is not changed by this declaration contract.

Whole-sequence enumeration and a list-level item-vocabulary shorthand are not
admitted. Any later shorthand needs explicit typed equality, lexical parsing and
normalization semantics, plus its own metamodel and migration contract. These
forms must not be inferred from an authored string or a native reference.

General lists permit empty sequences unless an effective contract forbids them;
shipped name-list contracts continue to reject empty values. Custom tokenizers
and source syntax for declaring list cardinality remain separate work.

## Proposed datatype execution adapter

Existing schema behaviors expose typed input bindings, function signatures and a
`schema:diagnostic-result` description with source-range and detail metadata.
Their diagnostic execution contract is not automatically a datatype validation
contract. Selecting a native behavior must also select an explicitly registered
capability with a compatible datatype validation signature.

Compilation should check the registered capability's owner, the retained behavior
identity, required input bindings and declared representation before publishing a
ready descriptor. Runtime invocation supplies the original candidate and datatype
handles and the consumer's current typed value. Those handles must not be adapted
to legacy `object` candidate records. Unknown execution or missing required runtime
context remains unavailable, rather than invalid data or successful validation.

The consumer retains the lifecycle envelope for complete, pending/unavailable and
failed execution. An implementation cannot claim completeness for a missing
dependency or extend scope grants. Conversion continues to use a distinct
capability with declared typed output, followed by every effective restriction.
Concrete function parameter names and metamodel types remain to be specified.

## Adopted validation result protocol

A completed datatype rule returns explicit boolean acceptance together with zero
or more attributed diagnostics. Acceptance is independent of diagnostic severity:
a warning does not automatically accept an invalid value, and an informational
diagnostic need not reject a valid value. Aggregate datatype acceptance requires
acceptance from every effective base, derived and attribute-local restriction;
a later successful rule cannot erase an earlier rejection.

A rejection without explanatory diagnostics receives a consumer-generated failure
at the retained input. Preserve implementation-provided source attribution and
relate it to the original datatype/behavior declaration; use the original input as
the fallback location. Diagnostics cannot substitute for a boolean acceptance
field, and result metadata cannot substitute for retained input handles.

| Execution outcome | Datatype acceptance | Consumer treatment |
| --- | --- | --- |
| Complete, accepted | Accepted by this rule | Retain diagnostics and continue remaining restrictions |
| Complete, rejected | Rejected by this rule | Retain diagnostics; provide an explanation if absent |
| Pending/unavailable | Not established | Retain lifecycle issues; do not publish successful validation |
| Failed or malformed result | Not established | Report an execution contract failure; never infer acceptance |

The consumer owns the lifecycle envelope. A completed result cannot claim that
missing dependencies are ready. Diagnostic severity remains available to the
consumer's reporting/publication policy, independently of the explicit datatype
acceptance result. Validation does not implicitly convert, and neither acceptance
nor diagnostics can replace the consumer's typed value.

This protocol is adopted but not enabled. Specify the registered validation
adapter's concrete typed result, input signatures and compatibility boundary
before invocation. Existing diagnostic-only behaviors require an explicit adapter
with a declared validity mapping; they do not acquire datatype compatibility just
because their result has diagnostic metadata. No JSON record substitutes for a
retained candidate or datatype node.

## Adopted general list emptiness

Admit an empty sequence unless the registered list contract or an effective
cardinality/rule restriction forbids it. There is no implicit nonempty restriction
for every list. Preserve the shipped name-list and wildcard-name-list nonempty
contracts explicitly. An item base constrains each present item; it does not by
itself require an item to exist.

An empty sequence and an absent attribute are distinct. Required-attribute checks
still apply to absence, and an authored empty value must pass its effective list
contract. A derived contract cannot remove an inherited nonempty restriction.
Converting a supplied empty sequence does not manufacture an item, a default or
a null value. Lexical empty-input handling belongs to the registered tokenizer;
representation-level emptiness does not authorize dropping invalid lexical tokens.

Source syntax for declaring custom list cardinality, registered tokenizers and
exact adapter signatures remain separate action items. No conversion adapter or
new cardinality field is enabled by this decision.

## Registry implementation boundary

Keep lexical name binding and original declaration identity separate. Name lookup
uses the declaring schema's local bindings and explicit namespace aliases/exports;
native selection retains the exact declaration returned by the shared resolver.
Both routes converge on its compiled descriptor. Internal scope handles and owner
identity are runtime values, not authored IDs or document-wide fragment lookup.

Collection must precede dependency binding so a forward literal name is not
mistaken for an unknown datatype merely because of source order. Repeated use of
one original declaration reuses its descriptor; distinct declarations claiming
one scoped name report a collision with both source locations. A descriptor's
base, item and behavior dependencies retain their own lexical bindings and
original owners rather than inheriting the consuming attribute's aliases.

Registry membership grants no new scope crossing or executable implementation.
Keep unknown names, unavailable native selection, invalid target shape and invalid
registered signatures distinct. Incomplete descriptors remain inspectable and
block activation according to existing coordinated package readiness rules.
These requirements guide the retained registry implementation; the exact public
API and metamodel admission still need specification and fixture actions.

## Adopted native node datatype declarations

Add a dedicated `node` datatype kind for reusable custom retained-node contracts,
with registered native validation rules and original node handles. Preserve
existing built-in `schema:node`/`cemml:node` contracts and their cardinality
semantics. The shipped `reference` kind describes symbolic values such as QNames;
it does not become an AST reference constructor or a retained target datatype.

A node contract receives the targets supplied by its consumer. The consumer owns
reference resolution, context and evaluation time; declaring a node datatype
does not automatically expand references or change the target subtree. Authored
descendant references remain native reference nodes until explicitly consumed.
Navigation remains bounded by the granted source view and effective traversal
limits. Original owners, target identity and source attribution must survive.

Reject lexical facets such as `values` and `pattern` on node datatypes. Registered
node rules express target constraints without extracting a string or replacing
nodes with copied records. Validation cannot mutate targets or return replacement
nodes. Scalar conversion rejects node input; any separately registered capability
for native nodes must preserve original handles and declare its own contract.
Adding this kind does not silently admit a node datatype as every list's item base.

The kind is adopted but custom node datatype execution is not enabled. Metamodel
admission, base compatibility and typed adapter signatures remain required work.
Reference-to-ID conversion remains deferred to the `cem-element` consumer track.

## Grammar and symbolic-reference consumer boundaries

Grammar datatypes bind explicitly registered implementations, including adapters
for existing content-model parsers. Literal rule text is descriptive metadata;
there is no new executable grammar language. Validation checks the supplied value
without implicit conversion, and explicit conversion needs its own declared
representation and normalization contract.

Symbolic-reference datatypes validate their declared lexical representation, such
as a QName resolved against the original declaring aliases where applicable.
Checking lexical well-formedness is distinct from consumer-requested lookup of
a declaration or node. Neither a symbolic-reference type nor a registered rule
implicitly loads URLs, uses document-wide IDs or grants a scope crossing. Missing
lookup context remains a consumer lifecycle issue rather than a lexical parse
failure. These boundaries do not yet specify every parser/output adapter.

## Adopted node rule invocation unit

A node datatype rule validates the complete ordered target sequence supplied by
the consumer once, using a typed native sequence parameter. It can check each
target as well as cardinality and relationships between targets. The original
candidate parameter remains one explicitly typed source node, distinct from the
target sequence. Acceptance and diagnostics follow the adopted result protocol;
existing effective traversal accounting applies to all target access.

Invoke the sequence rule for a complete empty selection as well, subject to its
effective cardinality and required-input contracts. Do not treat zero targets as
a reason to skip validation. Incomplete resolution is a lifecycle outcome, not an
empty successful result. The adapter does not automatically invoke the same rule
once for every target; a rule may explicitly inspect its permitted target view.

This unit is adopted but not enabled. Existing attribute cardinality constraints
remain effective alongside datatype rules; neither can erase the other's
rejection. Concrete native sequence parameter types and metamodel admission
remain required before execution.

## Typed adapter signature requirements

The current standalone scalar converter returns `TypedAttributeValue` containing
a datatype name and canonical lexical value. Native attribute consumption instead
retains `SchemaDeclarationNode` targets. A general adapter must preserve these
separate representations rather than flatten sequences to strings or route node
values through legacy object parameters.

| Binding | Required representation | Compiler/runtime check |
| --- | --- | --- |
| Datatype | Original retained named `{type}` declaration | Declaring identity, lexical scope and registered capability ownership |
| Candidate, when supplied | Original retained input node | Explicit native node parameter; no synthesized candidate record |
| Scalar/lexical/grammar/symbolic value | Declared scalar representation | Registered input type; no inferred primitive from a local name |
| List value | Ordered typed item sequence | Registered item contract and sequence type |
| Node value | Ordered retained target sequence | Original owners and permitted native view |
| Validation result | Explicit acceptance and attributed diagnostics | Completed typed result distinct from unavailable/failed execution |
| Conversion result | Declared canonical scalar/items/native representation | Separate explicit capability; all effective restrictions validate the result |

A registry entry must declare which of these capabilities and required inputs it
supports. The compiler checks source signatures against that registration; the
runtime checks that the actual consumer inputs are available. Reusing diagnostic
metadata does not turn an `object` function into a native datatype adapter.
Typed representation names for schema/function declarations remain to be specified;
this table does not introduce enabled metamodel literals or serialize AST handles.

## Adopted source-less value consumers

Document validation naturally supplies an original candidate node, but the current
standalone scalar conversion API accepts a lexical value and a source-map stack
without an AST candidate. The adapter needs a policy for such calls.

Candidate availability is declared per registered capability. Standalone value
consumers may invoke capabilities that do not require a candidate; a rule requiring
one remains unavailable when none was supplied. When present, the candidate must
always be an original explicitly typed native node. Never fabricate a document,
copy an input record or invent a context ID to satisfy a missing input.

Preserve the current standalone scalar API until its general-adapter compatibility
is implemented. A missing required candidate is an unavailable execution input,
not rejection of the supplied value. Original datatype/behavior provenance and
available source-map attribution remain retained even for source-less scalar input.

## Implemented registry collection foundation

`schema::datatype_registry::DatatypeRegistry` retains caller-supplied lexical scope
handles and named original declarations. Local name lookup and exact declaration
lookup remain separate. Repeated insertion of one original declaration is reuse;
a distinct same-name declaration reports both original sources and leaves the
existing binding intact. Original owners stay alive while their identities remain
registry keys. Scope roots require no authored IDs.

The six registry fixtures cover reuse, collision provenance, independent scopes,
equal node addresses in different owners, retained lifetime and malformed
collection targets. Together with datatype inventory, native value contracts and
declaration reference regressions, 83 focused tests pass. This is collection only:
namespace admission, QName binding, descriptors, dependency compilation, registered
implementations and native `@type` execution remain separate work. Membership does
not grant intrinsic semantics or indicate a ready executable datatype.

## Adopted omitted kind for derived datatypes

A derived declaration may inherit the kind of its resolved base. The compiler
waits for an incomplete base rather than guessing a kind or falling back to
strings. A list declaration explicitly states `kind=list` when its base selects
the item contract. A declaration with neither an explicit kind nor an inheritable
base cannot become a ready datatype merely from its local name.

An omitted field is distinct from an explicitly empty or unsupported kind. Source
descriptors preserve this distinction; inference must not hide malformed authored
fields. Existing shipped declarations all specify their kind and remain compatible.
Whole-list inheritance still needs a distinct dependency contract before it can
be consumed; inferred kind alone does not resolve a base edge's meaning.

## Implemented retained source descriptors

`DatatypeSource` retains the caller-supplied original lexical scope, declaration
and every authored attribute node. Unknown fields and repeated occurrences remain
available in source order; the last-authored field view preserves scalar or native
value nodes without evaluating them. A descriptor outlives the registry and source
owner variables through its original retained owners. No AST is copied or rewritten.

Four source fixtures verify native dependency fields, unknown-field visibility,
omitted versus empty kind, source lifetime and last-authored precedence. Together
with the adjacent registry/inventory/value/declaration-reference tests, 87 focused
tests pass. These are source descriptors, not compiled executable descriptors;
kind inference, dependency resolution and registered adapters remain guarded.

## Adopted list dependency boundary

Shipped list declarations use `base` for their item datatype. For other derived
datatypes, `base` denotes inheritance. A general compiler must not silently switch
these meanings when a dependency is itself a list.

Retain shipped list `base` item semantics and defer whole-list inheritance until
a separate explicit syntax is designed. A list item contract and a list rule
express reusable item and sequence constraints. A nested list and a derived list
are not interchangeable, and an omitted kind must not silently reinterpret an
existing item's base as whole-list inheritance. No dedicated item-type field or
whole-list inheritance syntax is admitted in the first compiler.

## Implemented retained source classification plans

`DatatypeSource::plan` classifies explicit kind metadata or retains the need for
inherited classification. It distinguishes list-item, inherited-base and native
validation-rule dependencies, keeping original dependency attributes and native
reference handles. Literal rules remain descriptive source fields. Empty, unknown
or native kind metadata is reported at its original field; malformed native base
or rule shapes and forbidden list/node `values` retain original issue attribution.

A plan neither evaluates dependencies nor owns a traversal budget. The later
lifecycle compiler must consume its dependency fields in the shared bounded
request. An explicit kind or absence of planning issues does not establish
executable readiness: namespace admission, inherited facet applicability,
registration, target compatibility and effective restrictions still need checks.
Unknown authored fields remain visible through the source descriptor.

Five plan fixtures and the adjacent source/registry/inventory/native-value/
declaration-reference regressions pass (92 total). Native datatype execution
remains guarded. This implements source classification, not kind inference across
a resolved dependency graph or a complete datatype compiler.

## Adopted inherited base compatibility

Shipped grammar `content-model` has a string base, and symbolic `type-reference`
has a qualified-name base. A strict same-kind-only policy would reject these
existing contracts; unrestricted scalar representation equality would also admit
relationships not justified by the registered validation/conversion contracts.

An inherited base must be explicitly compatible with the derived datatype's
registered contract. Registration declares accepted base representations/kinds,
and compilation checks the selected original descriptor against that contract.
Preserve shipped cross-kind relationships through their registered implementations,
rather than a local-name exception table. Matching representation alone does not
acquire a primitive implementation or a new grant.

A derived declaration with an inherited kind reuses the resolved base contract;
it does not gain an implementation through its local name. Any additional native
rule must have its own registered compatible signature. When a declaration
explicitly selects another kind, the effective registered contract must authorize
that base relationship and preserve the one canonical value checked by all
inherited restrictions. Unknown registration or incomplete base metadata cannot
establish compatibility; an established incompatible relationship is a schema
compilation error with both original declarations retained for attribution.

List-item edges are not inherited-base edges, and whole-list inheritance remains
deferred. Node contracts cannot acquire a scalar conversion merely through matching
metadata. Registration checks do not add scope access or replace traversal limits.

## Adopted inherited converter selection

A general datatype may inherit a base and add restrictions without changing its
representation. Validation and conversion remain separate capabilities, and a
consumer must explicitly request conversion. The compiler still needs a rule for
selecting the one converter when several declarations contribute constraints.

Inherit the base converter unless the derived datatype has
an explicitly registered compatible conversion capability. Select one effective
converter during compilation, invoke it once when conversion is requested, then
validate that result against every base, derived and attribute-local restriction.
A derived conversion capability may replace converter selection, but a validation
rule or restriction cannot return a replacement value. Never run base and derived
converters as an implicit pipeline, or fall back to another converter after a
failure. The selected output must satisfy the registered representation and base
compatibility contract.

Missing conversion capability remains distinct from unavailable validation and
invalid input. If an explicitly selected converter is unavailable, retain the
candidate and preserve the last complete active package under the existing
coordinated readiness policy; do not silently choose another implementation.
A validation-only datatype may expose no conversion capability, and absence of
an optional converter does not by itself make its validation contract unavailable.
Concrete capability identities and registered output signatures still need wiring.

## Shared dependency traversal implementation plan

The existing resolver supports an original owning container and consumer-selected
original children under one active reference stack and request/destination budget.
Reuse this boundary for datatype source roots and native dependency fields. Keep
original base/rule attributes as dependency containers so singleton selection and
target shape errors can be attributed to the correct field. Native references
remain original nodes, and behavior selection does not descend into an entire
imported schema.

Literal QName binding is a compiler-owned symbolic dependency, not ordinary
containment. It needs explicit lexical lookup and edge authorization under the
same request accounting. Do not synthesize a Reference AST node, use structural
containment to bypass crossing checks, or start a fresh resolver per field.
Preserve incomplete dependency branches separately from complete empty selections.
Registration, converter selection and restriction composition follow dependency
binding; none can make an incomplete traversal appear ready.

## Adopted scalar values equality

Scalar datatype declaration `values` needs a comparison contract. Existing
attribute-local `values` behavior remains a separate migration compatibility
surface. A restriction must not silently change the candidate representation to
make an enumeration match.

Compare scalar declaration values using equality declared by the registered
datatype contract. For an integer contract, numeric `003` and `3` may compare equal;
for a string contract, distinct lexical strings remain distinct unless its declared
equality says otherwise. Preparing enumeration constants during schema compilation
and comparing a supplied value do not authorize rewriting that value or implicitly
invoking its converter during validation. Unknown equality/constant interpretation
cannot establish a ready values restriction.

Each inherited values restriction retains its original declaring contract,
constant interpretation and equality binding. A derived equality does not replace
a base restriction's equality or widen the inherited allowed values. Existing
attribute-local vocabulary behavior remains unchanged until migration is explicit.
Registered equality invocation and constant preparation are not enabled yet.

## Implemented bounded native dependency traversal

`traverse_native_datatype_dependencies` consumes original datatype roots and
base/rule fields as one forest through the shared consumer-container resolver.
Consumer field wrappers retain original handles and edge roles; they are not
synthetic AST nodes or references. Intermediate native references preserve the
requested dependency role, and an invalid rule target does not trigger traversal
of unrelated datatype dependencies. Selected behavior declarations remain leaves.

The traversal retains source plans, field-grouped targets, hard target issues and
deferred dependencies. Native cycles, request/destination limits and denied scope
crossings keep selection incomplete. A complete empty or multiple selection is a
cardinality error, whereas a pending selection is not an empty result. Literal
QName dependencies now use the explicit `lookup_literal_type` lifecycle hook.
It receives the original `DatatypeSource`, field and unchanged QName; the host binds
names in that declaring lexical scope and returns original typed targets. Symbolic
links retain original attribute metadata without synthesizing Reference AST nodes.
They share native budgets, depth, active-link detection, unresolved policy and grants.
An absent lookup adapter is pending. Missing original lexical source context remains
deferred. Wrong target kinds and malformed source fields fail independently of
unresolved-reference disposition.

Thirteen traversal fixtures and the adjacent source/registry/inventory/native-value/
declaration-reference regressions pass (105 total), including mixed edges, declaring
scope preservation with conflicting names, crossing grants and shared limits.
Completeness here describes dependency selection, not executable datatype readiness.
Compiler namespace/capability adapters, effective facet checks and conversion/equality
execution remain pending in this separate datatype workstream; existing native
attribute type consumption stays guarded.

## Deferred datatype decision: enumeration constant authoring

The existing `values` attribute is a string whose current compiler helper splits
whitespace-separated tokens. It can express a vocabulary such as `true false` or
numeric tokens, but cannot represent one string constant containing whitespace.
Registered datatype equality does not itself supply a new literal syntax.

Recommended first contract: preserve the existing token form and defer richer
retained enumeration constant authoring to a separate design item. Token constants
must still be checked through their registered datatype contract; constraints
beyond this source form can use registered validation rules. Do not invent nested
quoting, parse JSON, or treat the whole attribute as a single enum constant.

Alternative: design retained enumeration constant declarations now, including
original owners, typed scalar literal admission, references and compatibility with
the existing token form. This supports whitespace-containing strings and richer
values directly but adds metamodel and source-selection work before activation.

This decision is deferred to the separate datatype workstream and remains unanswered.
Choose this source boundary before enum constant compilation. Whichever form is
chosen, list/node declaration `values` rejection and the separation of existing
attribute-local vocabulary migration remain effective.

## Verified inventory and current native boundary

The Rust inventory retains the original AST owners for all 18 declarations in
`cem-ml-generic.cem`. It confirms four lexical, nine scalar, two list, one grammar
and two symbolic-reference declarations. `name-list` and `wildcard-name-list`
have item bases; `type-reference` has `qualified-name` as a symbolic value base.
Named and prose `rule` values coexist in the same shipped vocabulary.

| Current surface | Evidence | Compiler implication |
| --- | --- | --- |
| Shipped string/boolean/integer/number conversion | Native conversion outputs pass the existing schema validator | Reuse these explicit conversion contracts, preserving normalization |
| Lexical, URI/semver/media-type/path, list, grammar and symbolic-reference conversion | Standalone conversion rejects the inventoried unsupported examples | Supply registered implementations; do not default them to strings |
| Native node attributes | Existing retained validation uses explicit node contracts; scalar conversion rejects nodes | Keep a distinct retained-input contract |
| Literal schema validation | Existing predicates recognize additional built-in names | Predicate availability alone does not establish a conversion implementation |

This is a migration baseline, not parity for every datatype. Conversion may
accept alternative lexical forms and produce a canonical value: for example,
boolean conversion maps `1` to `true`, while direct schema validation requires
its existing lexical contract. Validation and explicit conversion remain distinct
consumer operations. Full kind-specific parity and rule-registration fixtures
remain required before replacing legacy lookup or enabling native `@type`.

## Implementation sequence after decisions

The actionable steps and adjacent verification scenarios are in
[todo.md](todo.md#general-datatype-compilation-design).
Start with an inventory of existing declarations and native contract parity,
then establish the descriptor and lexical registry, bind dependency graphs,
implement kind consumers, and finally enable native attribute type consumption.
Keep function reference contracts, child-scope override syntax, and
`cem-element` ID conversion on their existing deferred tracks.
