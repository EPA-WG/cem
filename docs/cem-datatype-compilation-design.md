# General datatype compilation design

Status: adopted 2026-10-09. This is the maintained datatype contract, including
bounded singleton attribute `@type` consumption. The lifecycle contract below
supersedes the historical availability notes in the dated implementation records.
Those records preserve the detailed adopted signatures, provenance rules and
verification history; deferred extensions remain in [todo.md](todo.md).

## Supported lifecycle and activation

An explicit `CemQlSchemaPackageCompiler::with_datatypes` registration supplies
original sources and their executable implementations. `with_datatype_discovery`
also discovers the original type collections, namespace captures and explicit
exports. Both paths now bind every effective typed attribute before package
activation. Merely installing ordinary CEM-QL adapters or parsing a schema does
not select implementations.

Literal QNames and native `@type={#datatype}` slots bind the same executable
descriptor. Native selection must resolve to exactly one named original `{type}`;
complete empty/multiple selections are invalid. A pending, denied, cyclic,
unavailable or over-budget selection cannot become an untyped/string fallback.
Literal lookup uses the original attribute slot's captured prefix binding and its
own declaring schema's `uses` aliases and explicitly admitted exports. Neither
lookup nor registration grants a scope crossing or infers URL loading. Installed
catalogs take precedence over old manual bindings, including when lookup fails.

The compiler preserves effective attribute declaration precedence and each
original owner. It installs a consumer only after datatype compilation, lexical
preparation (including list items), local facet applicability, declaring-scope
diagnostic dependencies and default validation complete. Nodes use the retained
native sequence path. Defaults use their original `@default` source as candidate.
Validation never invokes conversion; rules, inherited restrictions and local
facets intersect. A completed rejection remains invalid even when its rule emits
only warnings or no diagnostics.

Attribute binding has a shared finite work allowance across the model's slots;
each reference selection also obeys the host's effective depth/work policy and
crossing grants. Preparation, rules and facets keep their existing explicit
limits and operation control. Rebinding clears previous installed consumers
before checking the new invocation. The package compiler creates a fresh host
and checks the compilation's original owner on every attempt. Incomplete or
invalid candidates remain inspectable while the previous schema, converters and
artifacts stay active together; retry reuses the retained candidate source.

Installed `SchemaDocumentModel::attribute_datatypes` consumers validate actual
attribute values through the same composition. Retained input validation passes
original attribute candidates and authorized native target forests. Attribute
candidates expose their own scalar metadata and provenance; they grant no ancestor
or unevaluated payload navigation. Changing an input
context reselects its targets without caching them on the source. Source-less
lexical validation is supported only when the registered capabilities permit an
absent candidate; a required candidate remains incomplete. Runtime incompleteness
is distinct from completed rejection and cannot report successful validation.

The source-only schema compiler retains its guarded native type behavior and
legacy literal path. Execution still requires explicit registrations. The
[pretyped attribute provenance contract](#adopted-pretyped-attribute-input-provenance)
and its sealed native consumer are implemented. Whole-list inheritance uses the
explicit `@list-base` contract below. Checked inherited preparer/tokenizer
replacement is implemented through explicit host selections. Checked inherited
facet-profile replacement retains every ancestor profile. Explicit typed-only
producer admission and retained scalar enumeration constants are implemented
below. Explicit declaration
overrides are implemented through the host transaction specified below.
The [scalar composition design](cem-scalar-composition-design.md) adopts native
behavior `@function` selection while preserving these datatype dependency roles.
Its function consumer now supports explicit coordinated package activation.
The separate [typed prelude contract](cem-typed-prelude-design.md) is implemented
under explicit CEM-ML 1.1 admission, with versioned writer/reload support.

## Problem and accepted boundary

The source-only schema document compiler constructs attributes and their local
facets. The explicit executable compiler constructs the retained `{types}` model.
Several validators identify built-in types by the local part of a type name.
The separate native attribute conversion contract already supports a base model,
intersecting restrictions, and one final converted value.

Attribute `@type={#datatype}` adopts an explicit reference selecting one named
`{type}` declaration. The source declaration, its lexical schema and its owner
must remain retained. Compilation controls evaluation context and timing.
Source readiness guarding keeps this slot pending until its registered consumer
is ready; malformed slots are errors. Source-only literal lookup remains compatible. An unknown type must not become an untyped/string fallback.

The shipped vocabulary contains `lexical`, `scalar`, `list`, `grammar` and
`reference` kinds. Some `rule` values are names such as `local-name`; others are
prose such as `signed decimal integer`. List declarations use `base` for an item
shape, for example `name-list` with `base=identifier`. Those existing forms must
be inventoried and tested before claiming a general datatype compiler.

## Compiled representation

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
   declared exports. Native dependency selection uses the shared lifecycle resolver
   and requires exactly one named `{type}` target. Do not infer URL lookup or grants.
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

## Datatype kind contracts

| Kind | Meaning | Remaining design work |
| --- | --- | --- |
| `scalar` | One value with an explicit primitive conversion/validation contract and restrictions | Built-in registry binding, normalization and custom implementations |
| `lexical` | A value checked by a declared lexical predicate | Executable `rule` binding and compatibility with shipped descriptions |
| `list` | Ordered typed values satisfying an item datatype contract, with authored lexical source retained; declaration `values` rejected | Shipped tokenizer/item conversion adapters and parity |
| `grammar` | A value checked by a registered grammar consumer; source prose is descriptive | Concrete registered parser/validation adapters and parity |
| `reference` | A symbolic-reference value contract such as a QName | Separate lexical validation from consumer-requested dereferencing; do not imply URL or AST link evaluation |
| `node` | Original retained nodes checked by registered native validation rules; lexical facets rejected | Automatic attribute consumption and migration parity |

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
converts a value. Explicit validation signatures are implemented below; conversion
capabilities still need their own typed contracts.

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

Native `@rule` selection remains guarded by the unfinished effective datatype
compiler. The explicit registered validation adapter below checks retained
behavior signatures without enabling automatic source datatype consumption. Generic CEM-ML parsing still only retains reference nodes;
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

The following semantic input roles describe the contract; the exact enabled
bindings and metamodel `source` literals follow the adopted signatures below:

| Role | Typed input | Purpose |
| --- | --- | --- |
| Candidate | Explicit retained native node | Original input/source provenance and allowed candidate navigation |
| `datatype` | Original retained `{type}` declaration | Declaring identity and source contract without a copied record |
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

### Explicit declaration overrides (adopted 2026-10-09)

The host lifecycle may replace a public datatype binding through an explicit
`DatatypeOverrideRegistry` transaction. This adds no authored override syntax.
Source order, equal local names, schema imports, successful reference selection
and package replacement grants confer no datatype replacement authority.

A `DatatypeOverrideRequest` names the exact consuming schema scope, expanded
public name, expected original `DatatypeSource` (owner, declaration and declaring
scope), replacement original source, and a finite list of original dependency
attribute slots to rebind. Both sources must already have original name/source
admission. A replacement may have a different authored name; the explicit public
binding is the alias. Other exports/import scopes retain their existing binding.
Duplicate requests for one public binding or dependency slot are invalid.

The embedding host explicitly calls `authorize` to issue an opaque
`DatatypeOverrideGrant` for that complete request and the registry's current
generation. Grants are process-local, nonserializable and tied to one registry;
they cannot be inferred from a source declaration, copied name, package manifest,
reloaded arena or a grant for another replacement. Missing, altered, stale or
foreign grants fail before selection. Directed scope-crossing permission remains
separate and is required for both the new public binding and every rebound edge.
Revoking authorizations advances the generation without changing the active model.
Continue the same registry for successive overrides: recreating one from public
model/catalog views is rejected because those views alone do not retain the
original-to-effective dependency policy.

Public symbolic consumption uses the replacement only after successful activation.
Datatype dependencies continue to resolve against each declaration's original
lexical catalog. Existing compiled/native source handles remain pinned. To change
an existing inheritance or list-item base dependency, the request must explicitly list its original
attribute slot; validator/function slots and ordinary attribute-consumer slots
are not datatype dependency rebinding sites. The compiler first resolves the
unchanged source slot with the current contexts, singleton checks, bounds and
grants. Its selected original target must equal the retained expected target.
Only then can the explicitly authorized effective edge point to the replacement.
No source AST, reference target edge, QName, alias or expression is rewritten.

A subsequent explicit rebind keeps the original authored target as its selection
check while replacing the previously active effective target. This permits
successive authorized updates without making old grants or expression results
reusable. Slots omitted from the request retain their existing effective binding.

Preparation clones the host's execution configuration and public name catalog,
then recompiles the complete active datatype source closure plus replacements.
All changed edges are installed in the compilation graph before compiling any
root. This rebuilds transitive dependents, preserves unrelated native identities,
and detects cycles, incompatible representations, widened restrictions, invalid
facets and missing capabilities using the ordinary compiler. Replacement sources
use their own original dependencies and explicitly registered implementations;
no old implementation, preparer, facet profile or validator is transferred merely
because a public name is reused. Diamonds may reuse one newly compiled descriptor
within the attempt. No descriptor from the previous active model is a fallback.

The staged host uses the caller's operation for selector evaluation. One finite
work/depth allowance covers request admission, graph traversal,
compilation and the final attribute binding pass. Cancellation or incomplete
selection keeps the candidate unavailable. Attribute contracts are rebuilt on a
private model with the candidate public catalog, so literal consumers switch with
the name while native consumers remain pinned. Existing structural/function
registrations remain the supplied model's contracts; they receive no new authority.

Activation checks the registry generation, exact host input snapshot and current
operation again. The input check includes contexts, grants, scope assignments,
source/name admission and namespace completions. It publishes the prepared host,
public catalog, datatype contracts and attribute model as one synchronous native
transaction. Nothing fallible runs after these checks. Invalid, pending,
cancelled, stale or partially bound candidates leave the preceding host and model
active. Holding an older model keeps its original immutable descriptors valid;
activation does not mutate them. The API exposes the ready model for an embedding
package lifecycle to publish through its existing package readiness boundary.

These grants do not implicitly replace an inherited lexical preparer, tokenizer or facet profile.
Their checked replacement requires a separate explicit host capability selection
and retains runtime base-admission checks.

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

Explicit registered native list conversion uses the descriptor adapter below.
The shipped name-list converters use the consuming descriptor's registered
tokenizer and retain each occurrence's decoded UTF-8 span and original lexical
input through native atomic views. Item validation runs after conversion; no
item converter is inferred or chained. Existing scalar conversion APIs must not
silently reinterpret a string result as a typed sequence.

### Explicit list serialization (adopted 2026-10-09)

Serialization is a separate host-selected capability on an exact original list
declaration and declaring scope. `RegisteredListSerializer` declares its scalar
item representation, implementation identity and candidate requirement. The
compiler checks the effective list representation and binds the original native
datatype view. No selection leaves validation/conversion available; a selected
unavailable capability prevents compilation readiness. A list never inherits a
serializer from its item base. Serializer source syntax, function/query serializers and native-node/list-of-list
items remain separate work. A whole-list derivative inherits its base serializer
unless the host explicitly selects a compatible replacement.

`ExecutableDatatype::serialize_list` takes an explicitly supplied typed sequence,
an optional original native candidate, and the caller's runtime and finite limits.
It validates all effective typed item/list restrictions before invoking the selected
serializer once. A rejected or incomplete validation cannot invoke serialization.
Serialization never invokes conversion, preparation, tokenization, stringification
of native nodes, or a fallback serializer. Callbacks borrow the original ordered
item views, preserving duplicate occurrences, view access boundaries and source
provenance. A serializer must implement its registered lexical format without
reordering or merging items; representation alone does not define that format.

The typed execution outcomes are `Serialized` (text and diagnostics), `Rejected`
(diagnostics, no text), and distinct pending/unavailable/failed/limit states.
Public text is available only after complete accepted validation and serialization.
An accepted empty list can serialize to a present empty string; absent input is
not represented by that empty list. Diagnostics never substitute for the outcome.
The result retains validation evidence and registration identity separately from
the exported text. Output is an explicit lexical export, not a document/AST
serialization or a replacement for the typed sequence or authored lexical input.

The shipped `name-list` and `wildcard-name-list` serializers emit item spellings
unchanged, in order, separated by one ASCII space, with no leading/trailing space.
They reject invalid item spellings rather than escape or silently normalize them.
Their registered list bounds continue to reject empty values. This export grammar
is explicitly selected; it does not infer the inverse of an arbitrary tokenizer.
Original token text, decoded byte spans and source maps remain unchanged even
when canonical separator bytes differ. The existing scalar API result types and
attribute ingress boundary are unchanged. Attribute-local lexical facets still
belong to the attribute consumer and require their original lexical provenance;
serialization cannot supply that provenance by stringifying a typed input.

The default output bound is 1 MiB; existing validation limits bound typed input,
rules, comparisons and cumulative diagnostics. Callback limits receive only the
remaining diagnostic allowance after validation. Callbacks cooperate with the
same operation control/query budget and must bound their own work and allocation.
The boundary checks control before/after callbacks, output bytes and diagnostics
before publication; a cancelled or oversized result publishes no text.

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
shipped name-list contracts continue to reject empty values. Registered tokenizer contracts and source cardinality are implemented below;
shipped conversion adapters remain separate work.

## Adopted datatype execution adapter

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
Datatype validation parameter names are fixed by the adopted input-role contract
below. Result construction and boundary checks follow the adopted contract below;
shared record metamodel admission and result adapters are implemented below.
The explicit registered validation adapter below implements source signature
checking and execution binding. Effective datatype compilation remains separate.

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

Adopted 2026-10-08: the completed validation result is a dedicated typed result
with two required fields: `accepted`, a boolean, and `diagnostics`, a sequence of
attributed diagnostics that may be empty. A missing or mistyped field is an
execution contract failure, not a completed rejection or implicit acceptance.
The result cannot be supplied through an unvalidated legacy object shape.

The explicit result consumer implements this protocol through shared value
contracts. Registered invocation uses the checked signatures and explicit
execution/compatibility binding below. Existing diagnostic-only
behaviors require an explicit adapter with a declared validity mapping; they do
not acquire datatype compatibility just because their result has diagnostic
metadata. No JSON record substitutes for a retained candidate or datatype node.

## Adopted result construction and consumer boundary

Adopted 2026-10-08: the dedicated validation result contract permits ordinary
CEM-QL record construction. A special constructor is optional. The registered
rule must declare the contract; it is checked statically where possible and
enforced at the consumer boundary for CEM-QL and native implementations alike.

Each completed invocation returns exactly one result. Its required `accepted`
field contains exactly one boolean, without truthiness conversion. Its required
`diagnostics` field is a sequence of valid diagnostic values, which may be empty.
Missing fields, incompatible values or invalid cardinality fail execution; they
do not constitute rejection of the input value. Pending/unavailable execution
remains outside the completed result in the consumer-owned lifecycle envelope.

| Concern | Contract owner |
| --- | --- |
| Authority to execute a datatype rule | Explicit capability registration and retained behavior/owner identity |
| Correctness of the returned value | Declared result contract, static checks where possible and runtime boundary checks |
| Diagnostic source attribution | Retained source handles and source-map information |

Matching result fields do not register a legacy behavior or satisfy its source
signature requirements. A constructor cannot grant execution authority or replace
boundary checks. The dedicated type denotes the declared, enforced contract;
construction through a particular function is not required for membership.

The outer result record is newly produced validation information. Constructing it
does not project the candidate or datatype AST into records. Original node handles
retain their owners and identity throughout invocation and reporting.

The shared diagnostic value contract below covers severity, message/code and
attribution. The shared record metamodel and explicit result adapter implement
these checks without requiring a result constructor. The registered adapter below
checks fixed function roles and explicit cardinalities before execution.

The shared result validator connects explicit CEM-QL/native results to this
contract; registered signatures must be checked before rule dispatch. Checking
rule results must not recursively invoke the unfinished general datatype compiler.

## Adopted diagnostic value contract

Adopted 2026-10-08: the same consumer adapter accepts existing native diagnostics
and newly constructed, checked CEM-QL diagnostic records. Both representations
feed the existing report pipeline under the shared diagnostic contract.

| Record field | Contract |
| --- | --- |
| `code` | Required; exactly one diagnostic-code string |
| `severity` | Required; exactly one of `info`, `warning`, `error` or `fatal` |
| `message` | Required; exactly one string |
| `source` | Optional; zero-or-one original native node |

For an existing native diagnostic, preserve its source maps and metadata rather
than rebuilding it from the fields exposed by the query view. The current native
view exposes only part of that information; adapter implementation must retain
the underlying diagnostic attribution.

For a newly constructed record, derive attribution from the supplied `source`
node. When no source node is supplied, use the invocation's available input
attribution. Source-less consumers need no fabricated candidate or context ID.
A rule validating a node sequence can identify the particular offending target;
the target retains its original owner and identity. Do not perform ID lookup to
interpret `source` or replace its native node with an object projection.

Missing required fields, incompatible values or invalid cardinality make the
diagnostic malformed and fail the invocation. An invalid source value does not
select fallback attribution. Diagnostic severity does not determine `accepted`;
host reporting and abort policies remain applicable independently.

Declare these field rules in the schema-owned contract and compile them into
consumer boundary checks. Use the existing report pipeline rather than a separate
datatype reporting model. The explicit native/CEM-QL result adapter below uses
these declarations without a recursive dependency on the unfinished general
datatype compiler. Diagnostic-only behavior compatibility still requires its
explicit acceptance mapping.

## Adopted shared value-contract declarations

Adopted 2026-10-08: add a small shared value-contract declaration for reusable
record fields and sequence cardinality. Validation results and diagnostics use
these declarations; behavior-local result/detail metadata does not need to repeat
each diagnostic shape. Compile the contracts without invoking user datatype rules.

Field presence and value-sequence cardinality are distinct. For example, the
validation result must contain `diagnostics`, but its sequence may be empty;
`accepted` must be present with one boolean. Diagnostic `source` is optional and
contains zero-or-one original native node. Reusing the diagnostic contract retains
these constraints in each consuming result contract.

Adopted 2026-10-08: value-contract records reject undeclared fields by default.
An explicit schema option may allow additional fields. Allowing extensions does
not relax the types, presence or cardinality of declared fields. A misspelled
`soruce` field therefore fails a closed diagnostic record contract rather than
silently selecting fallback attribution. This policy governs record-field
validation; existing native diagnostic metadata remains preserved by its native
adapter and is not discarded or rejected as undeclared record fields.

The bounded declaration surface and compiler below implement this decision.
Existing AST element field-contracts and legacy behavior result/detail contracts
remain compatible. Contract compilation grants no behavior registration or
datatype execution.

## Implemented value-contract surface and result adapters

Adopted and implemented 2026-10-08: a schema's `value-contracts` collection holds
named `value-contract` records with `value-field` children. `allow-extra=true`
explicitly admits undeclared record fields; the default is false.

Each field declares `name`, exactly one of `kind` or `type`, optional `required`
(default false), optional `cardinality` (default `one`) and optional `values`.
`kind` selects the primitive representation `string`, `boolean` or original native
`node`. `type` selects another named record contract. The supported cardinalities
are `one`, `zero-or-one`, `zero-or-more` and `one-or-more`. `values` is a
whitespace-token vocabulary on string fields only. Arrays remain single values;
checking a sequence never implicitly flattens an array. String/boolean fields
require atomic representations; a node or record's scalar accessor does not
implicitly convert it to a scalar field value.

The shipped schema declares `datatype-validation-result` and
`datatype-diagnostic` in this vocabulary. Their field rules and severity
vocabulary come from the source declarations. This does not repurpose the legacy
`schema:diagnostic` datatype or broaden existing function return-type literals.

`schema::value_contracts::ValueContracts` compiles original retained schema
handles with caller-supplied declaring namespaces and QName bindings. CEM sources
use owner-checked captured names; pending names do not fall back to lexical names.
Unprefixed contract references use the declaring namespace, qualified references
use its supplied bindings, and targets must be present among explicitly supplied
sources. The compiler loads nothing and grants no scope crossing. Distinct
same-name declarations, unknown types, unsupported fields/facets and cyclic record
contracts fail compilation; repeated use of the same source is reusable.

The first profile uses overridable limits of 256 contracts, 4096 fields,
64 nested record edges and 100,000 validation work units. Field/value visits share
one validation budget across nested records. Source expressions and native
reference operands are not evaluated by this record-contract compiler; future
source consumers must be explicitly designed before admitting them.

`cem_ql::datatype_results::DatatypeResultAdapter` binds the result and diagnostic
contracts, checks their protocol representations, then consumes an explicitly
completed query/native result. Native record fields and scalar observations are
snapshotted once so decoding uses the values that were checked. Runtime result
checking preserves original handles and emits no report as a side effect.
The returned shared `Diagnostic` values enter the host's existing report pipeline;
`original` retains the result and native owners. Source-node attribution uses the
original retained owner; source-less calls use explicitly supplied fallback
metadata. Existing native diagnostics retain all their metadata.

Execution errors retain their original failed stream and diagnostics. Diagnostics
already emitted by query execution remain separate from returned rule diagnostics;
neither determines the acceptance boolean. A rejection without a diagnostic gets
a generic consumer explanation. Malformed result errors identify the contract
source and field path where available. Diagnostic-only legacy behaviors do not
acquire acceptance by returning a matching record.

These result APIs are also used by the registered dispatch adapter below. Full
datatype compilation and automatic native attribute `@type` activation remain
separate work.

## Implemented registered validation profile

Adopted and implemented 2026-10-08: `DatatypeBehaviorContract` checks the original
schema-owned `{behavior @execution=datatype-validation}` declaration against an
explicit `ValidationSignature`. The owner must contain that exact behavior in its
`behaviors` collection. Captured names and QName bindings come from the declaring
source; a matching name in another owner has no registration authority.

The three `inputs/input-binding` declarations use fixed `name` and `source`
values: `value`, `datatype`, `candidate`. `datatype` has explicit `node` type,
`required=true` and cardinality `one`. An optional `candidate` has `node` type,
`required=false` and `zero-or-one`; a required candidate has `required=true` and
`one`. All three argument bindings are supplied even when the candidate is empty.
Missing roles, duplicate roles, renamed inputs, object substitutes and incompatible
cardinalities fail source compilation.

The registered value representation is a singleton scalar, ordered scalar-item
sequence, or ordered native node sequence. The first scalar adapter recognizes
the CEM-QL atomic representations `string`, `boolean`, `integer`, `decimal`,
`double` and `uri` explicitly. Qualified primitive names must bind to the schema
namespace; an unrelated `vendor:string` does not inherit string semantics.
List/node sequences declare `cardinality=zero-or-more` and may be empty. These
representation checks do not establish full shipped datatype semantics, list
item compatibility, tokenization, normalization or wider numeric conversion.

Native implementations use `implementation=engine` and an exact registered
`primitive` identifier. Query implementations use `implementation=function`, a
selected inline `function`, and one retained expression in `body`. Each function
`param` repeats its checked role, type, required flag and cardinality. The ordinary
result profile declares `result @type=schema:datatype-validation-result`, with
cardinality `one`; its function declares `returns=datatype-validation-result`.
The source consumer rejects unsupported extra fields, defaults and selector forms.
Existing AST-validation/legacy object functions do not acquire this profile.

`DatatypeValidationRegistry` has no default entries. Registration binds the
checked original owner and behavior to a native callback or the compiled original
query body, plus the shared result adapter. Duplicate registrations fail; retained
bindings keep their original implementation when a registry clone changes. A host
binds only after authorized dependency selection and effective-kind compilation,
passing the original named datatype node and the effective kind. Registration
and binding do not resolve a native `@rule`, authorize a scope crossing or publish
a complete datatype descriptor.

`validate_rules` takes the host's effective ordered restriction list and complete
runtime inputs. It checks every input before executing any callback, preserves
native handles and order, and checks the same operation control before and after
calls. Queries receive the fixed bindings and the consumer's current runtime
capabilities. No new control budget or context ID is created per restriction.
Default batch caps are 256 rules, 100,000 input values and 100,000 accumulated
diagnostics, all host-overridable. Native implementations must cooperate with
operation control and bound their own work; query execution uses the supplied
control directly.

Every completed rule is retained with its original datatype and behavior source.
Acceptance is the conjunction of all completed restrictions. A rejection does not
skip later rules or let their acceptance overwrite it. Missing required candidates,
pending/unavailable execution, cancellation, malformed results and failed queries
leave aggregate acceptance unset and preserve prior completed results. An original
candidate supplies fallback attribution when present; source-less consumers use
their supplied attribution. Explicit diagnostic source handles and native metadata
retain their own attribution.

Diagnostic-only compatibility is opt-in per registration. It declares
`result @type=schema:datatype-diagnostic @cardinality=zero-or-more`; a query
function declares `returns=diagnostic-sequence`. The host must separately register
an acceptance mapping: reject selected codes, reject selected severities, or
accept only an empty returned diagnostic sequence. There is no implicit mapping.
All returned diagnostics pass the same structural checks before mapping; previously
emitted query reports remain separate. Code-based mappings can reject informational
diagnostics or accept error diagnostics, so severity alone never determines
validity. Existing legacy object/result formats still need an explicit host bridge
to this checked diagnostic sequence; they are not reinterpreted automatically.

The validation adapter now feeds the explicit descriptor compiler and package
readiness integration below. Conversion/equality adapters and automatic native
`@type` consumption remain in `todo.md` with their scenarios.

## Implemented native diagnostic query prerequisite

The existing native diagnostic view exposes `severity` through direct field access
and record enumeration, using `info`, `warning`, `error` and `fatal`. Placing a
native diagnostic inside a constructed CEM-QL result retains its full underlying
diagnostic metadata, source map and identity; reading it does not emit a report or
interpret validation acceptance. This is query access, not result validation.

A native query fixture covers all four severities and retained attribution, with
22 existing query/template/XSLT error-recovery regressions passing alongside it.
Registered validation dispatch is implemented separately below; native attribute
`@type` consumption remains guarded.

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

The adopted sequence-bound fields and registered tokenizer boundary are implemented
below. Tokenization does not enable conversion; its typed capability contract is
adopted separately and implementation remains open.

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

The explicit descriptor compiler below implements node kind admission, native
sequence validation and compatible inherited bases. Automatic attribute `@type`
consumption remains guarded pending complete namespace/facet and migration checks.
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

The explicit adapter implements this invocation unit for registered native node
sequences, including empty sequences. Existing attribute cardinality constraints
remain effective alongside datatype rules; the effective compiler must include
all restrictions before enabling attribute `@type` execution.

## Adopted datatype validation input roles

Adopted 2026-10-08: registered datatype validation rules use fixed input names.
Registration does not map arbitrary source parameter names to these roles.

| Input name | Role |
| --- | --- |
| `value` | The consumer's current typed value, in the registered kind-specific representation |
| `datatype` | The original retained datatype declaration whose contract is being validated |
| `candidate` | The original retained input node, when supplied; required only when the registered capability declares that requirement |

The compiler checks these roles against the selected behavior signature and
registered capability. A differently named parameter does not acquire a role by
position or inferred aliases. `datatype` and a supplied `candidate` retain their
native owners and source attribution; neither becomes a copied object record.
`value` remains scalar, ordered typed items or an ordered retained node sequence
as required by its contract. A node-valued `value` is distinct from `candidate`.

Candidate availability follows the adopted source-less consumer policy below.
Missing a required `candidate` leaves execution unavailable; do not manufacture
a node or infer acceptance. Adopted 2026-10-08: an optional `candidate` is a
zero-or-one native node sequence, empty when absent and containing the original
input node when present. Multiple nodes violate this input contract. Absence is
not a null/object placeholder or a fabricated node.

Adopted 2026-10-08: the registered signature records zero-or-one candidate
cardinality for an optional input and exactly-one for a required input. Compilation
checks the source declaration against that signature; runtime enforces actual
cardinality. General CEM-QL cardinality syntax may remain deferred without hiding
this requirement from compilation. An empty candidate value does not imply an
omitted function argument. Missing a required candidate remains unavailable, as
specified above.

Completed validation returns explicit acceptance plus attributed diagnostics.
Validation cannot replace `value`; conversion remains a separate capability.
The decision fixes validation input names and candidate cardinality. Conversion
ABI, parameter order and concrete metamodel admission remain separate work;
the explicit validation profile below fixes named binding and metamodel admission.
The explicit native conversion adapter below consumes compiled validation descriptors;
full datatype/attribute migration remains separate.

## Typed adapter signature requirements

The current standalone scalar converter returns `TypedAttributeValue` containing
a datatype name and canonical lexical value. Native attribute consumption instead
retains `SchemaDeclarationNode` targets. A general adapter must preserve these
separate representations rather than flatten sequences to strings or route node
values through legacy object parameters.

| Binding | Required representation | Compiler/runtime check |
| --- | --- | --- |
| Datatype | Original retained named `{type}` declaration | Declaring identity, lexical scope and registered capability ownership |
| `candidate` | Original retained input node; zero-or-one when optional, exactly-one when required | Compiler checks source against registered cardinality; runtime checks native identity and count; missing required input is unavailable |
| `value` (scalar/lexical/grammar/symbolic) | Declared scalar representation | Registered input type; no inferred primitive from a local name |
| `value` (list) | Ordered typed item sequence | Registered item contract and sequence type |
| `value` (node) | Ordered retained target sequence | Original owners and permitted native view |
| Validation result | Exactly one result under the dedicated contract; ordinary CEM-QL record construction allowed | Required singleton boolean `accepted` and sequence `diagnostics`; static checks where possible and runtime boundary checks |
| Conversion result | Declared canonical scalar/items/native representation | Separate explicit capability; all effective restrictions validate the result |

A registry entry must declare which of these capabilities and required inputs it
supports. The compiler checks source signatures against that registration; the
runtime checks that the actual consumer inputs are available. Reusing diagnostic
metadata does not turn an `object` function into a native datatype adapter.
The explicit profile below specifies validation representation names and source
cardinalities. Conversion representations and full datatype compatibility still
need their own compiler checks; none of these signatures serialize AST handles.

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
Whole-list inheritance requires `@list-base`; inferred kind alone does not
reinterpret an existing `@base` item edge.

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
kind inference, dependency resolution and automatic datatype consumer activation
remain guarded; explicit validation registration is available separately.

## Whole-list inheritance (adopted and implemented 2026-10-09)

Shipped list declarations keep `@kind=list @base=item`: `base` selects the scalar
item contract. A separate `@list-base` selects one complete list declaration and
inherits its whole sequence contract. Both literal QNames and a single explicit
native reference constructor are supported:

```cem
{type @name=names @kind=list @base=identifier}
{type @name=short-names @list-base=names @min-items=1 @max-items=3}
{type @name=pair @kind=list @list-base={#selectedList} @min-items=2 @max-items=2}
```

These examples assume the original `identifier` and root `names` declarations
have explicit host registrations. A declaration name never supplies an
implementation. `selectedList` is supplied by the original declaration's native
runtime context and must resolve to exactly one authorized list declaration.

`@list-base` and `@base` are mutually exclusive on a declaration. A derivative may
omit `kind` and inherit it, or explicitly state `kind=list`; another kind is
invalid. Empty, malformed, composite or repeated fields remain invalid. A plain
`@base` selecting a list without `kind=list` is invalid (`list-base-required`);
`@kind=list @base=another-list` remains an unsupported nested item contract. There
is no implicit inference between item selection and sequence inheritance, and no
new item-type override field. To introduce a different item contract, declare a
new root list with its own registrations.

### Dependencies and compatibility

`DatatypeDependencyRole::InheritedList` retains the original `list-base`
attribute. Literal lookup uses its original lexical scope, aliases and namespace
admission; native lookup uses the existing bounded reference consumer. Both obey
scope grants, singleton target checks, cycle detection and the shared compilation
work/depth budget. Pending lookup or an incomplete ancestor keeps the candidate
unready. A resolved scalar/node target or a different effective representation is
invalid, with the original edge and incompatible base retained for attribution.
Cross-kind compatibility registrations cannot authorize a non-list ancestor.

A derivative needs no implementation registration when it only adds restrictions.
A local implementation, if present, must declare the same list kind and scalar
item representation. The descriptor retains its base `Arc` and shares the exact
effective item descriptor with all list ancestors; item preparation, rules and
enumerations remain attached to that original item contract.

| Contract | Whole-list inheritance |
| --- | --- |
| Item datatype | Share the exact effective scalar item descriptor; no replacement. |
| Cardinality | Intersect registered, authored and every inherited bound; reject an empty intersection. Keep each restriction's original source. |
| Validation rules | Inherit ancestor rules in order and append local rules. Each rule retains its declaring datatype and behavior owners. |
| Tokenizer | Inherit by default. Explicit `CheckedReplacement` preserves base admission and exact spans; ordinary `Ready` cannot replace a base. Unavailable selection stays pending. |
| Lexical preparer | Inherit by default. Explicit `CheckedReplacement` preserves original base admission and immutable output representation. No capability is invented for an absent base. |
| Facet profile | Inherit by default. Explicit `CheckedReplacement` retains every original profile and intersects local acceptance under the same representation. |
| Converter | Inherit by default; an explicitly selected compatible converter replaces selection. No chaining or fallback. |
| List serializer | Inherit by default; an explicitly selected compatible serializer replaces selection. No chaining or fallback. |
| Enumerations | Whole-list `values` remains unsupported. Every inherited scalar item enumeration still applies to each occurrence. |

Output conversion and serialization remain explicit operations. They validate the
complete effective list and item contracts. Lexical preparation uses the inherited
tokenizer and item preparer and preserves ordered duplicate occurrences and their
decoded UTF-8 token spans. The preparation and facet owners remain the original
registered declarations. Attribute-local facets can further restrict the result.

### Execution, publication and rebinding

The validation schedule checks the flattened sequence rules once, then validates
each scalar item once, regardless of list inheritance depth. It does not recurse
through whole-list bases during execution. Preflight accounts for all effective
rules and values before callbacks; existing cancellation, candidate, diagnostic,
enumeration and shared query budgets still apply. Sealed native receipts retain
the exact derived attribute binding and provenance; consumption freshly checks
all inherited and local constraints without repeating lexical preparation.

Package compilation publishes the whole graph atomically. Pending or invalid
ancestors cannot replace the last ready model. The existing explicit datatype
override transaction recognizes `InheritedList` edges: changing a public name
leaves existing literal and native edges pinned unless the request and grant name
the exact original `list-base` slot. Authorized rebinding recompiles transitive
derivatives, selects the replacement's effective item contract and capabilities,
and rechecks compatibility and cardinality before publication. An incompatible
replacement is rejected. Old immutable descriptors remain pinned; committed
attribute contract replacements retire their old native preparation authority.

The schema metamodel admits `list-base`, and shipped schema aliases that derive
whole lists use it. Generic shipped root lists retain their original item bases.
Nested/native-node list items remain separate work. Retained scalar enumeration
constants are supported under the explicit authoring extension below.

## Checked lexical capability replacement (adopted and implemented 2026-10-09)

The host can explicitly select `PreparationBinding::CheckedReplacement` for an
original derived datatype, or `TokenizerBinding::CheckedReplacement` for an
original whole-list derivative. Both preserve the resolved base's lexical
admission at execution time. Declaration names, implementation IDs, ordinary
`Ready` selection and datatype override grants alone do not authorize replacement.
No new source syntax or transport authority is introduced.

The compiler verifies the exact original declaration/scope, effective kind and
output representation. A replacement without an inherited base is invalid; a
missing inherited capability is pending. An unavailable ancestor never falls back
to a child implementation. Transitive ancestor checks retain their original
registered owners and bound datatype views. Inherited candidate requirements
remain effective even when the child declares an optional candidate. A dependency
rebind recompiles these checks against the newly authorized effective base; it
cannot reuse a guard from another source graph.

### Scalar preparers

On lexical ingress, execute retained ancestor preparers from oldest to newest,
then the selected preparer. Every step receives the same original lexical input,
token slice/span (for an item), candidate and runtime; each keeps its own original
bound datatype view. Outputs are compared, never passed as input to the next
preparer. A completed rejection ends ingress with `accepted=false`. Pending,
unavailable, malformed, failed, cancelled or budget-exhausted work stops without
publishing a value. There is no fallback to an earlier successful result.

Each output must have the declared primitive representation and immutable storage:
primitive atoms or the engine's retained scalar/token adapters. Arbitrary query
views and representation-name claims are insufficient. Checked replacement uses
exact representation equality: strings/URIs and stored decimal/wide-integer
spellings must match; doubles must preserve their bits (including signed zero and
NaN payloads). This intentionally does not invoke numeric coercion, datatype
comparators, conversion, normalization or query equality. A replacement can narrow
lexical admission, but cannot change the base's prepared value. A mismatch reports
`PreparationStop::IncompatibleReplacement` with no published value or receipt.

Immutable comparison storage is bounded by `max_lexical_bytes` before copying.
Retained scalar source maps, when present, must match the original input. Retained
token views must use the original lexical owner, source and exact occurrence span.
The published report and sealed receipt continue to retain original lexical text,
source and ordered token spans even when the result is a primitive atom.

### List tokenizers and orchestration

A checked tokenizer runs every retained base tokenizer on the same decoded input
before its selected implementation. Each implementation's spans are checked for
UTF-8 boundaries, ordering, nonempty tokens, bounds and complete separator
coverage using that implementation's own separator policy. All resulting spans
must be identical by occurrence. Returning fewer tokens, merging/splitting tokens
or widening a separator policy cannot pass compatibility merely because token
text or a tokenizer ID matches. A tokenizer can explicitly return
`TokenizationError::Rejected` to narrow admission; incompatible spans and invalid
outputs remain incomplete failures.

`RegisteredTokenizer::checked_replacement` retains a flat bounded check sequence.
`TokenizationLimits::max_tokenizers` defaults to 256 and preflights every invocation
before any callback. Existing byte/token limits and operation checks apply to each
step. Explicit shipped list conversion uses the same checked tokenizer.

A checked whole-list `list_items` preparer selects a new original registration
identity while retaining the shared item contract and all base obligations.
Tokenization and item preparation each run once through their effective check
sequences; list ancestors do not cause repeated preparation of every item. No
arbitrary whole-list preparation callback is admitted by this extension.

### Budgets, sealed evidence and publication

`PreparationLimits::max_preparations` includes scalar ancestor/selected callbacks
and list/tokenizer orchestration. A list reserves its preparer-step count plus
its tokenizer-step count minus one shared orchestration unit; each item's scalar
checks are charged separately. Tokenizer reservations remain spent on early failure.
Scalar chains preflight their full callback/input-visit allowance, and lists
preflight all item chains after bounded tokenization. Candidate requirements are
checked before callbacks. Diagnostics share one cumulative allowance and retain
original input attribution; query failure and cancellation stop later steps.

Successful sealed issuance spends the guard budget once and retains the checked
immutable result. Subsequent pretyped consumption freshly validates all rules,
enumerations and local facets without repeating lexical guards. No receipt is
issued for a mismatch or partial result. Original context, operation, publication
and exact binding checks still govern receipt lifetime. Defaults use the same
checked lexical path and all inherited/local validation. Pending or invalid
replacement candidates preserve the active package and its receipts; committed
attribute contract replacement retires previous preparation authority.

## Implemented retained source classification plans

`DatatypeSource::plan` classifies explicit kind metadata or retains the need for
inherited classification. It distinguishes list-item, inherited-base, explicit
whole-list and native validation-rule dependencies, keeping original dependency attributes and native
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

List-item, scalar/node inherited-base and whole-list `@list-base` edges remain
distinct. Node contracts cannot acquire a scalar conversion merely through matching
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
Explicit native equality and constant preparation are implemented below; authored
source/query capability profiles remain deferred.

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

## Adopted enumeration constant authoring

Adopted 2026-10-08: preserve the existing whitespace-token `@values` form.
Each whitespace-separated token is one constant. This form can express
vocabularies such as `true false` or numeric tokens, but cannot represent one
string constant containing whitespace.

Token constants must be checked through their original registered datatype
contract and compared using its registered equality. Equality does not supply
new literal syntax or rewrite the candidate. Unavailable constant interpretation
keeps the executable contract incomplete; do not fall back to lexical equality.
Constraints beyond the token form can use registered validation rules.

The separate retained-constant extension adopted below adds explicit whole-value
literals and reference selection. Existing `@values` tokens keep their meaning:
no nested quoting, implicit JSON interpretation, or treating the entire attribute
as a single constant.

## Retained scalar enumeration constants (adopted and implemented 2026-10-09)

A datatype may author an ordered sequence of `constant` children instead of
`@values`. Every child has exactly one literal `@value`; the complete decoded
attribute string is one constant input, including whitespace and the empty string.
For example:

```cem
{type @name=status @kind=scalar |
    {constant @value="In progress"}
    {constant @value="Done"}
    {constant @value=""}
}
```

This source is passive. The host must register the datatype implementation,
constant interpreter and equality before it can become executable. No new scalar
literal parser, converter, JSON boundary, or implicit name-based capability is
introduced. The interpreter receives the full decoded string and its original
`@value` candidate, returns exactly one scalar in the registered representation,
and compilation validates that value against the effective datatype rules and
inherited enumerations. Thus a registered integer interpreter can admit an exact
wide integer while a registered string interpreter preserves spaces. A missing or
unavailable capability keeps compilation incomplete.

### Source forms and metamodel

`type` admits `constant` children in the schema metamodel. A `constant` admits
only its required literal `value` attribute, plus already-consumed namespace
metadata. Empty string is present; an omitted/valueless field, native attribute
slot, duplicate/unknown/foreign field, nested child, or nontrivia text is invalid.
Comments and whitespace between declarations are ignored. Effective names use
the original host capture; a foreign element merely named `constant` is invalid.

A declaration must choose token `@values` or retained child slots. Mixing them is
invalid, even if a reference might later select nothing. Retained references that
select no constants in total are an empty vocabulary and invalid. A declaration
with neither form adds no vocabulary. Effective list/node kinds still cannot
declare whole-sequence enumerations; scalar item vocabularies remain available.
Attribute-local `values` remains a separate lexical facet.

### Native reference selection and ownership

A native reference in the type's child sequence may select zero or more original
retained `constant` declarations, including chains of references. It uses the
existing lifecycle host, current context, directed scope grants, destination
bounds and shared remaining compilation work. Each occurrence retains its order;
repeated selection is not deduplicated. A selected type, attribute, atom, arbitrary
record, expression element or reconstructed lookalike cannot stand in for a
constant declaration. Only original reference nodes are evaluated; selected
constant bodies and `@value` slots are never evaluated as expressions.

Source plans retain child slots without evaluating them. Compilation selects the
whole vocabulary before preparing any of its constants. Unresolved, pending,
denied, cyclic, invalid or bounded-out selection cannot publish a partial vocabulary,
regardless of diagnostic disposition. Reference diagnostics and failing original
occurrences remain in the compilation report. Valid targets from another owner
retain that owner and their original value attributes; no consumer AST is built.

`ConstantToken` remains the lexical inspection/callback carrier. Its explicit
`form` distinguishes `WhitespaceToken` from `RetainedLiteral`. A retained literal's
span is `0..decoded_value.len()`, including `0..0`; this is a decoded value span,
not a fabricated document source range. `PreparedConstant::declaration` retains
the original constant element. Vocabulary-level outcomes identify the original
`@values` attribute for tokens or the original type declaration for child slots.
Preparation candidates and fallback diagnostics identify each original value
attribute. Source/query constant profiles receive these same full values and
candidates through their existing fixed roles.

### Typed storage, inherited meaning and bounds

Retained literals admit immutable primitive atoms and the engine's private
immutable scalar representation (including wide integers). Public native-view
representation tags do not prove immutable scalar storage. There is no node
atomization, runtime-input conversion, or coercion to a matching primitive.
Registered interpreters own preparation semantics; all effective typed validation
still runs before a constant is retained. The lexical source is preserved even
when explicit interpretation yields a different canonical scalar.

Each vocabulary retains its original datatype, interpreter and equality. Derived
vocabularies intersect inherited restrictions and must themselves satisfy those
restrictions during preparation. Reusing a constant declaration in another type
is a fresh interpretation under that type's explicit contract; it never changes an
already-compiled ancestor vocabulary. Existing token vocabularies are never
reinterpreted by this extension.

Preparation shares `max_constants`, `max_lexical_bytes`, validation/comparison/
diagnostic allowances and remaining traversal work across all roots. Each repeated
selection consumes its own allowance. Retained literal output text additionally
shares `max_retained_value_bytes` (default 1 MiB) across the compilation; this cap
is distinct from the existing decoded-input byte cap. Callback implementations
must bound their internal work; operation control is checked before and after
callbacks. Immutable prepared values are retained only after scalar and datatype
validation. This extension does not change legacy token interpreter admission.

Package publication remains atomic: invalid source, incomplete selection,
unavailable interpretation, rejected constants and exhausted budgets preserve the
previous complete schema/converter/artifact publication. A fresh ready retry
prepares the original candidate again. No compiled constant authority is stored
in source references or restored through serialization.

List/node declaration `values` rejection and the separate migration of existing
attribute-local vocabularies remain effective. This source decision does not
enable executable native attribute `@type`; its other compiler contracts remain
open.

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

The completed implementation sequence and adjacent verification scenarios are in
[archived checklist](archive/todo-snapshot-2026-10-10.md#general-datatype-compilation-design).
The sequence established declaration inventory and native contract parity, the
descriptor and lexical registry, dependency binding and kind consumers before
enabling native attribute type consumption. Function reference contracts,
child-scope override syntax and `cem-element` ID conversion retain their separate
adopted contracts.

## Adopted sequence bounds and registered tokenizers

Adopted and implemented 2026-10-08: datatype declarations of effective kind
`list` or `node` admit literal nonnegative `min-items` and `max-items` fields.
Defaults are zero and unbounded. Registered, inherited and authored bounds
intersect; every restriction retains its original declaration. Contradictory
bounds are compilation errors. Scalar declarations reject sequence bounds.
The existing attribute-local `minItems`/`maxItems` facets remain separate.

An absent lexical input is distinct from supplied empty input. An explicitly
registered tokenizer returns ordered, nonempty UTF-8 byte spans into the retained
lexical input, preserving source maps and duplicates. Spans cannot overlap or
skip content outside the tokenizer's declared separator policy. The provided
Unicode-whitespace tokenizer is a capability value that the host must select for
an exact list declaration; there is no implicit tokenizer or new `@tokenizer`
syntax. Custom tokenizers declare their separator policy and cooperate with the
caller's operation control. Default limits are 1 MiB of input and 100,000 tokens,
with host overrides and checks before/after execution and during span inspection.

Tokenization does not convert tokens into typed items or execute item rules.
An optional absent tokenizer permits validation of already typed items; an
explicitly selected unavailable tokenizer prevents executable readiness. Shipped
name-list nonemptiness must be registered explicitly during the pending migration.

## Implemented validation descriptors and package readiness

`cem_ql::datatype_compilation::compile_datatypes` compiles retained sources through
`traverse_native_datatype_dependencies`. All requested roots share the remaining
work budget, including descriptor compilation. Scope grants, destination limits,
cycles and incomplete dependency states remain owned by the shared resolver.
Literal names enter through an explicit immutable namespace/export catalog or the
legacy original-scope binding API. The catalog implementation is described below;
literal and native schema/package collection discovery are implemented below.
Native `@base`/`@list-base`/`@rule` targets retain their original owners and source attributes.

The host registers a kind, typed representation, accepted cross-kind bases,
optional bounds/tokenizer and optional default validator for an exact original
datatype and lexical scope. This registration does not follow a same-name vendor
declaration. Omitted or identical derived kinds can reuse the resolved base's
contract. A different kind requires an explicit compatibility entry, and every
base restriction must inspect the same representation. Node/scalar mixing is
invalid even with a compatibility entry. An explicit list's `base` selects its
item contract; the initial executable list representation admits scalar items.
Whole-list inheritance uses `@list-base`; list-of-list/native-node items remain unsupported.

An explicitly registered primitive scalar or native-node representation can be
validated without additional predicates. Lexical, grammar and symbolic-reference
contracts require a registered rule; a representation alone does not establish
those semantics. Both host-selected validators and authored native rules restrict
the value. An authored rule still requires complete authorized selection even
when its target is also the registered default. Compilation checks exact behavior
owner, effective kind and value signature. Inherited rules retain their original
declaring datatype handles. Literal rule prose remains descriptive.

The descriptor validates complete native values without conversion. It checks
representations and required candidates for the entire scheduled invocation before
any callback. Node rules receive the complete ordered sequence once; list item
rules receive each item in order, preserving duplicates, and list rules receive
the whole sequence. Bounds and every rule restrict acceptance. All calls share
operation control and aggregate rule/input/diagnostic limits; cancellation or
incomplete execution leaves acceptance unset and preserves completed outcomes.
Cardinality rejection records retain the declaring source, required bounds and
actual count. Descendant references remain authored native nodes.

Unknown fields/structural children, duplicate fields, incompatible bases and
invalid bounds cannot compile. Node lexical facets and list declaration `values`
are rejected. Scalar `values` keeps compilation incomplete until registered
constant interpretation and equality are available; it is never ignored.
Explicit native conversion is implemented below. Grammar/reference implementations,
source/query equality and constant profiles are implemented below; full shipped parity remains open.

`CemQlSchemaPackageCompiler::with_datatypes` adds an explicit lifecycle compilation
callback using the current prepared host. It attaches the resulting immutable
`DatatypeCompilation` snapshot to `SchemaDocumentModel`; incomplete snapshots
prevent activation. The engine verifies that the snapshot belongs to the exact
retained request owner. Missing capabilities preserve the candidate for inspection
and leave the last complete schema, converters and artifacts active together.
A retry reuses unchanged source ownership but recompiles under current capabilities.
A ready snapshot from an old or unrelated owner cannot authorize a replacement.
Host callbacks must include every datatype they intend to expose as executable.

This is an opt-in validation compiler. Automatic literal/native attribute `@type`
consumption remains guarded until namespace binding, full facet contracts and
migration parity are complete. CEM-ML parsing does not evaluate these references.

## Adopted separate conversion and equality capability contracts

Adopted 2026-10-08; the native conversion adapter is implemented below; equality
and query bindings remain tracked separately. A conversion
registration names the exact original implementation, accepted input representation,
canonical output representation and candidate requirement. Inputs retain the
original datatype and optional/required native candidate; a lexical input retains
its original text and source maps. Node-to-scalar extraction is never implicit.
The consumer explicitly requests conversion under its existing runtime context
and operation budget.

A completed conversion has one of two typed outcomes: `Converted`, containing one
canonical scalar or an ordered item/native sequence of the declared output type
plus attributed diagnostics; or `Rejected`, containing diagnostics and no value.
An empty converted sequence is a present successful value, distinct from rejection
or absent input. Pending/unavailable execution, control failure and malformed
output stay separate from both completed outcomes. A source record/query adapter
must check this tagged result and exact output cardinality before publishing it.
Select one inherited or explicitly replaced compatible converter, invoke it once,
then validate the result with every effective restriction. Failure never falls
back to another converter, and diagnostics cannot replace the tagged outcome.

Scalar equality is a separate exact registration declaring the operand
representation and original contract. It accepts two singleton values of that
representation and returns an explicit equality boolean with attributed diagnostics.
Pending/unavailable/failed execution has no comparison boolean. Equality neither
converts operands nor changes their representation; it shares the consumer's
operation control. Sequence equality and node equality are outside this scalar
capability.

Enumeration preparation is another explicit registered operation: interpret each
existing whitespace token as one scalar constant under the restriction's original
contract, retaining its token span and owner. Its typed success/rejection follows
the conversion outcome distinction without implicitly selecting the runtime
converter. Prepared constants must satisfy their original contract before use.
Each inherited vocabulary retains its own interpreter and equality implementation.
Unavailable interpretation/equality blocks that restriction's readiness; there is
no lexical fallback. These semantic signatures introduce no new executable
`@rule` behavior, conversion source syntax or richer constant syntax by themselves.


## Implemented explicit native conversion

`DatatypeImplementations::select_converter` selects an exact retained datatype's
registered native converter or explicitly records its unavailability. With no
local selection, a derived descriptor inherits its base converter. With neither
selection nor inheritance, the descriptor remains validation-only. Local selection
must match the original declaration and scope, effective kind and canonical output
representation. A selected unavailable converter prevents descriptor/package
readiness; a selected failure never falls back to a base converter. Immutable bound
registrations retain their original implementation and declaring datatype owner.

`RegisteredDatatypeConverter` declares a typed lexical or scalar/list/native value
input, canonical output representation and candidate requirement. The caller supplies
lexical text/source maps or an already typed value sequence. Required candidates
are checked before invoking the converter. Representation checks never extract
scalars from nodes. Scalar/list conversion cannot produce native nodes, and native
conversion cannot produce scalars. Native output must clone the exact supplied
query views; rebuilding a broader view for the same underlying node is rejected.
Order, repeated handles and authored descendant references remain intact.

`ExecutableDatatype::convert` invokes the selected native callback exactly once.
The callback receives its original declaring datatype, the original candidate and
the same runtime context/control as subsequent validation. `Converted` carries a
present typed sequence (including an empty list); `Rejected` carries no value.
Pending, unavailable and failed execution are distinct incomplete states. The
boundary checks output count and representation before calling any validation
rule. It then applies the effective descriptor's inherited/local rules, list item
contracts and sequence bounds to that one output. It never chains item/base
converters or tokenizers implicitly. Attribute-local restrictions remain the
responsibility of the still-guarded attribute consumer.

The result exposes `accepted` only after explicit rejection or complete validation.
A converted value remains inspectable when its validation rejects or stops;
publication requires `accepted == Some(true)`. Validation retains its own stop
reason and original restriction outcomes. Native diagnostic attribution is kept;
wholly source-less diagnostics receive original candidate attribution, or the
caller's lexical/source fallback. Severity does not determine acceptance.

Default bounds are 1 MiB lexical input and 100,000 input/output values, all
host-overridable. The existing validation bounds govern subsequent rule execution;
conversion and validation share its cumulative diagnostic allowance and operation
control. Native callbacks must poll that control and bound their allocations/work.
Checks before/after the callback and during input/output inspection prevent
cancelled or malformed results from being published.

This implements explicit native capability selection and execution. It introduces
no new authored conversion behavior or function syntax, does not activate native
attribute `@type`, and does not migrate the shipped scalar conversion API. Checked
source/query conversion, equality and constant adapters are implemented below;
shipped parity remains tracked in `todo.md`, with its verification scenarios.


## Implemented native scalar equality and token preparation

`RegisteredScalarEquality` and `RegisteredConstantInterpreter` are separate exact
registrations selected through `DatatypeImplementations`. Each carries the original
datatype declaration, scope, implementation identity and scalar representation.
No local selection inherits the base capability; an explicit unavailable selection
remains unavailable through further inheritance. An absent/unavailable capability
blocks any vocabulary that needs it. Already prepared inherited restrictions retain
their own bound registrations even if a derived declaration selects another one.

`compile_datatypes_with_runtime` requires the consumer's explicit lifecycle context
and operation control to prepare vocabularies. The existing context-free compiler
keeps such declarations pending. Preparation retains each whitespace token's byte
range in the decoded attribute value, the full lexical value and original attribute
owner. This is not a fabricated byte range in the original source document.
For the token form, nonliteral and empty vocabularies are invalid. The separate
retained form is specified above. Token interpretation returns one scalar in the registered representation,
explicit rejection, or an incomplete pending/unavailable/failed outcome.

Before admitting a constant, compilation validates it against the original effective
descriptor's base/local rules and inherited vocabularies. Its own vocabulary is added
only afterward, avoiding recursive membership checks. The original `@values`
attribute supplies the preparation candidate and fallback diagnostic attribution.
Failed contract validation rejects the declaration; incomplete execution preserves
an incomplete candidate. Preparation never invokes a runtime converter. The resulting
immutable snapshot belongs to this lifecycle execution; hosts recompile when its
inputs/capabilities change, rather than mutating a previously published contract.

Runtime validation applies all vocabulary restrictions conjunctively. Within each
restriction, it compares against constants until one registered comparison returns
true. A rejected restriction does not skip later restrictions. Pending/unavailable/
failed comparison leaves overall acceptance unknown, retaining earlier outcomes,
diagnostics and the original stopped vocabulary attribute. Neither operand nor
candidate is converted or rewritten. Explicit comparison booleans determine
membership independently of diagnostic severity; supplied diagnostic metadata is
preserved and only wholly source-less diagnostics receive candidate attribution.

Default preparation bounds are 4,096 constants and 1 MiB of decoded vocabulary
text across all roots, plus cumulative validation allowances (256 rules, 100,000
input/candidate values, 100,000 comparisons and 100,000 diagnostics). Every bound is
host-overridable. Runtime validation shares one comparison and diagnostic allowance
across inherited vocabularies and list items. Callbacks must cooperate with operation
control and bound their own work; boundary checks run before and after callbacks.
Invalid, unavailable or bounded-out preparation never activates a replacement
package. A ready retry on the retained owner publishes the schema, converters and
artifacts together.

These native adapters do not activate native attribute `@type`, alter attribute-local
vocabularies or infer equality from a datatype's name. Source/query execution
profiles are implemented below; attribute integration remains in `todo.md`.


## Implemented explicit datatype namespace/export catalog

`DatatypeNameCatalog::collect` builds an immutable name snapshot from original
`DatatypeSource` handles and lifecycle-supplied lexical environments. Each given
scope has its own namespace, local declarations and explicitly admitted public
exports. Each declaration carries its own effective prefix bindings. A pending
namespace/prefix is represented separately from an undeclared prefix. These inputs
come from the original declaring context, never from an attribute that happens to
consume the resulting datatype. They require no authored scope IDs.

Collection indexes all local names before checking exports, so literal forward
references work without source-order overrides. It checks effective metamodel names
through the host's current native name view, requires one literal local declaration
name, rejects foreign `type`/`name` lookalikes and retains both sources for collisions.
Full kind/facet/capability checks remain in descriptor compilation. Repeated selection
of one original declaration is valid only with the same scope and lexical bindings.
Equal public namespace/local names in independent scopes do not merge those scopes.

An export supplies a public namespace/local name and an exact original datatype
source already collected in its own environment. An explicitly supplied public name
may differ from its source's local name; that rename never changes the target's base
or item dependency bindings. Imports are scoped to their requesting environment.
Two distinct sources claiming the same imported name, or an import conflicting with
a local name in that namespace, are errors. There is no global namespace search,
automatic URL load, document-fragment convention or built-in implementation fallback.

Unprefixed names resolve in the declaring scope's namespace; qualified names use
that declaration's effective aliases and its scope's explicit exports. A pending
binding yields pending lookup; malformed QNames, unknown prefixes and absent public
names remain unresolved. Both literal and native targets then pass the existing
reference consumer's scope grants, context readiness, cardinality, traversal limits
and original-source checks. A public export does not itself grant a crossing or an
executable implementation.

`CemQlSchemaDeclarationHost::install_datatype_names` installs the complete snapshot
only after checking every owner and scope. Failed installation preserves the previous
catalog, and a replacement cannot reassign a retained declaration's original scope.
Host clones keep their existing immutable snapshots. Scopes covered by a catalog
never fall back to legacy explicit name bindings after a lookup is pending or fails;
scopes outside the catalog retain the existing explicit binding API. Native selection
continues to identify original declarations independently of their public names.

Collection defaults to 100,000 work units and 1 MiB of retained name/alias/export
text; callers may override both. Exhaustion returns a source-attributed pending error
and publishes no partial catalog. These discovery bounds are separate from, and do
not reset, the later shared dependency traversal budgets. Consumers must supply a
fresh catalog when lexical environments or admitted exports change. This adds no
authored import/export syntax and does not infer effective aliases from raw source
strings. Schema/package source discovery now connects to these inputs below;
metamodel admission and shipped datatype migration remain in `todo.md`.


## Implemented literal source discovery and package handoff

`DatatypeNameCatalog::discover` consumes selected `DatatypeSchemaSource` inputs:
original schema handles, their matching lexical capture and explicit public exports.
It reads literal `types/type` declarations and `uses/use` aliases, validates effective
metamodel names and collects forward references before building the catalog. It
attaches immutable source-name metadata to an already registered owner; it creates
no runtime inputs, implementations, loaded resources or relationship grants.

CEM lexical capture now retains an `AttributeNamespaceSnapshot` at each original
attribute name. Literal QName discovery reads the dependency attribute's snapshot,
including pending bindings that shadow inherited URIs. Only the consumed prefix
needs resolution. A completed namespace view may supply that binding for the current
selected invocation; its URI is not written back into the source capture. The legacy
expression-occurrence index remains unchanged. `uses` aliases supply explicitly
declared schema bindings when a lexical prefix is absent. Equal bindings agree;
conflicting lexical/`uses` bindings are source-attributed errors, and pending lexical
bindings never fall back to `uses`.

Literal snapshots are passive reload metadata with original-owner checks, validated
attribute/declaration handles and retained source maps. Older sidecars without them
remain readable; a consumer needing a missing snapshot stays pending. Imported XML
schema literals now use the same metadata and native catalog, as described below.
No AST or internal binary codec changes are required.

Source scanning and catalog construction spend one caller-supplied discovery budget.
Duplicate declarations/aliases, malformed fields and content are reported at their
original nodes; collisions retain the related original declaration. Nonliteral schema
fields and schema-level collection replacement remain pending discovery work. No slot
is silently dropped or treated as an empty declaration collection. External schemas
and their public export contracts remain explicit host inputs; a URI is never a
request for implicit loading or a grant of access.

`CemQlSchemaPackageCompiler::with_datatype_discovery` adds an opt-in handoff: its
source callback selects the original schemas/exports, discovery builds and installs
the complete catalog, then its compile callback receives the discovered sources for
exact implementation registration and bounded descriptor compilation. Discovery
errors become inspectable datatype compilation issues on the candidate owner.
Every discovered declaration remains in the compilation's required source set, so
a callback cannot certify readiness by omitting an unsupported type. The existing
package gates preserve the last complete schema, converters and artifacts together
until the new candidate is ready. Source/query capability adapters are implemented
below; remaining metamodel admission and shipped datatype parity stay in `todo.md`.


## Implemented native datatype discovery collections

`DatatypeNameCatalog::discover` now consumes native reference slots inside `types`
and `uses`. A slot accepts zero or more original `type` or `use` declarations,
respectively. Reference chains resolve through the shared lifecycle resolver;
selected declaration descendants stay authored. A selected container is not a
request to splice or recursively discover its contents. General expression slots
and schema-level collection replacement are not newly admitted.

All `types`/`uses` slots for one schema run in one consumer traversal, sharing
request and effective destination limits, directed crossing checks and active
reference identities. `discover_with_limits` accepts explicit traversal bounds;
`discover` uses the standard schema defaults. Across supplied schemas, discovery
carries forward the remaining request work cap. Destination accounting belongs
to each schema's traversal. Scanning, traversal and catalog construction also
spend the shared name-collection work budget. Package discovery passes the host's
explicit traversal limits; descriptor compilation remains its own bounded stage.

All supplied original owners' name metadata is attached before selection. Each
selected datatype must have its original declaring schema among the supplied
`DatatypeSchemaSource` inputs. It is collected under that original schema, using
that schema's selected `uses` and the declaration's captured attribute namespaces.
Missing original schema input keeps discovery pending. Selecting a vendor type
does not introduce a local-name alias in the consuming schema. Public QName lookup
still requires explicit `DatatypeExport` mappings, and exports do not grant
crossings. Selected `use` declarations contribute their explicit literal alias/URI
pair to the consuming schema's alias collection without rewriting their source.

Repeated selection of one original datatype reuses its handle; distinct local
name collisions remain errors. Empty completed selection is valid. Pending,
unresolved, denied, cyclic, invalid and budget-limited selections cannot publish a
complete catalog, even when other slots yield usable nodes. Errors retain their
original failing source when available and retain original query/traversal diagnostics
through package compilation. A pending sibling cannot hide an invalid
or forbidden branch. No targets are written into source references.

Fixtures cover original vendor environments and export visibility, selected aliases,
repeated handles, retained descendants, nested chains, scope denial, depth/work
bounds and empty/pending/invalid outcomes. The package fixture retries the same
retained candidate after its native slot becomes ready, preserving the previous
schema/converters/artifacts until the complete replacement can activate.


## Implemented XML literal namespace capture

The shared XML import boundary now retains `AttributeNamespaceSnapshot` metadata
for every surviving original attribute. Capture happens after processing all
namespace declarations on the start tag: a declaration after `base="p:item"`
still governs that value. Child rebindings remain local, siblings restore the
parent context, and already captured values keep their original namespace URIs.
XML attribute expanded names remain unchanged; the default namespace does not
qualify an unprefixed attribute name. Interpretation of unprefixed datatype values
continues to belong to the common datatype catalog's declaring-schema rules.

The capture uses the importer's predefined `xml`/`xmlns` bindings. Those implicit
bindings have no fabricated authored source span. Explicit bindings preserve
original lexical spans and decoded URI values, including entity references.
XML namespace declarations remain literals: the existing expression-attribute
marker rejects `xmlns` targets. This adds no XML namespace-reference syntax or
new expression occurrences for literal attributes.

When XML expression folding discards attribute/payload nodes, lexical capture
removes their node associations before those arena addresses can be reused.
The surviving reference's lexical snapshot still retains the binding values and
source provenance needed by its expression. No stale namespace declaration or
attribute snapshot can attach to a later unrelated node.

The existing reload metadata carries these snapshots to a fresh original owner.
Older sidecars remain readable; absence of required literal metadata keeps QName
discovery pending, without reconstructing bindings from later declarations.
CEM and XML discovery use the same native catalog, alias-conflict rules and
explicit export/crossing contracts. No XML-specific lookup logic is added to
CEM-QL or datatype compilation.


## Implemented source/query conversion adapters

Adopted and implemented 2026-10-08. The `datatype-conversion` behavior profile
uses fixed `value`, `datatype` and `candidate` roles. Its source inputs and query
parameters must match the explicitly registered `ConversionSignature` in type,
requiredness and cardinality. `datatype` is the original datatype node; candidate
is an optional or required original native node. Lexical query input is the exact
input text as a string; its source map remains on the conversion call and supplies
fallback diagnostic attribution. Value inputs preserve their registered scalar,
list or native-node representation. No source declaration selects a converter
or grants execution authority on its own.

A behavior declares `result @type="schema:datatype-conversion-result"`. Query
implementations declare `returns="datatype-conversion-result"`. The shared schema
owns a closed result envelope: required string `status` (`converted` or `rejected`),
optional `value` sequence, and required diagnostic sequence. The new structural
`value-field @kind="value"` admits typed scalars or native nodes without coercion;
it admits neither record/array substitutes nor arbitrary structural recursion.
The registered conversion signature further checks exact output representation
and cardinality. Structural contract adapters explicitly opt into scalar-value
recognition; no datatype rule executes during structural checking.

`converted` requires a present `value` field, including a present empty sequence
when the output signature allows it. `rejected` requires that field to be absent;
even an empty field is invalid on rejection. Unknown tags, extra fields under the
closed schema, missing fields and malformed output leave completion unset. No tag
represents pending or unavailable execution: those remain explicit host/runtime
outcomes. Diagnostics cannot substitute for the tag. Result fields are observed
once and the same retained snapshot is validated and decoded.

`ConversionBehaviorContract::compile` checks original source membership and the
complete profile. `RegisteredDatatypeConverter::from_query` compiles its query;
`from_source` additionally requires the exact declared primitive identifier for a
host callback returning source records. Both retain original behavior owners and
produce the same explicit converter registrations as native typed callbacks.
The host still selects a registration for an exact original datatype through
`DatatypeImplementations::select_converter`. No new datatype binding syntax,
implicit `@rule` conversion, URI loading or native attribute `@type` activation
is introduced.

Queries use a closed role-binding environment and the caller's existing operation,
execution scope and query capabilities. Ambient bindings cannot replace those
roles. Pending/unavailable callbacks remain separate; cancellation is checked
before and after execution and before decoding source results. Output and
diagnostics are bounded, including reports already emitted by a query. Malformed
records retain those reports alongside a source-attributed adapter error. Native
diagnostics keep their metadata; source-less records use the original candidate
or lexical call attribution.

Successful output enters the existing convert-once path: exact native input-view
identity is checked before every effective datatype restriction validates the
result. Failure never invokes a fallback converter. Source/query scalar equality
and constant-interpreter profiles are implemented below. Shipped datatype parity
and automatic native attribute activation remain separate TODO items.


## Implemented source/query scalar enumeration adapters

Adopted and implemented 2026-10-08. The passive `EqualityBehaviorContract` profile
uses `execution="datatype-equality"` with required singleton `left`, `right` and
`datatype` roles. Both operands use the exact registered scalar representation;
`datatype` is the original native implementation datatype node. The function
repeats those roles and returns `datatype-equality-result`; its behavior result
names a compatible schema-owned record with required singleton boolean `equal`
and required diagnostic sequence. An empty result, missing boolean or malformed
record is incomplete execution, never a false comparison inferred from absence.

The passive `ConstantBehaviorContract` uses `execution="datatype-constant"` with
required singleton string `value` and native-node `datatype`/`candidate` roles.
`value` is precisely one existing whitespace token. `candidate` is the original
vocabulary attribute in its lifecycle-owned view; the native call and prepared
constant retain the original decoded span and owner. The retained-literal
extension supplies the full original `@value` through these same roles. Its function returns `datatype-constant-result`; the checked
schema-owned record requires `status` (`prepared` or `rejected`) and diagnostics.
Exactly one scalar `value` of the registered representation must be present for
`prepared`; rejection requires that field to be absent. Empty prepared output,
wrong types and pending/unavailable tags are invalid results.

Explicit `RegisteredScalarEquality` and `RegisteredConstantInterpreter`
`from_query`/`from_source` constructors bind these descriptors and result adapters.
Native registration requires the exact declared primitive identifier. Registrations
retain the original behavior owner and fixed signature; the host still selects
them for an exact original datatype. Source compilation itself performs no lookup,
reference resolution, execution or grant. Result-contract identity must match the
adapter registration. Scalar constants cannot import native nodes, arrays or
records as substitutes for their registered primitive.

Queries compile against only the fixed role bindings. Each invocation replaces
ambient role values, clears any current item and uses the caller's operation
context/control. Native callbacks receive remaining limits and original fallback
attribution. Cancellation is checked before and after execution; pending,
unavailable and failed execution remain distinct from completed outcomes.
Malformed results preserve already-emitted reports and add an attributed failure.
Checked diagnostic severity never substitutes for a comparison boolean or tag.
Native result fields are observed once for structural validation and decoding.

Both profiles use the existing bounded enumeration path. Preparation runs once
per token, validates the prepared scalar against every effective restriction and
keeps the package incomplete on rejected, unavailable or malformed preparation.
Runtime comparison spends the same cumulative comparison/diagnostic allowance
across all vocabularies and list items. Inherited vocabularies retain their original
interpreter and equality registrations; local replacements cannot reinterpret
base constants. Neither profile calls the runtime converter or changes ordinary
attribute-local vocabulary semantics. Shipped datatype parity, executable
attribute type integration and final design consolidation remain tracked actions.


## Shared shipped contracts and native validation adapters

Implemented 2026-10-08. `document_model::shipped_datatypes::ShippedDatatype`
provides an explicit host-selected capability inventory matching the 18 original
CEM-ML declarations. It shares the existing document validator predicates instead
of introducing a second lexical grammar. `validate_lexical` returns an explicit
unavailable outcome for `content-model`; the absence of a registered grammar is
never successful validation. Name lists remain nonempty. Symbolic-reference
validation checks syntax only and cannot resolve references or grant scope access.

`convert_lexical` reuses the shipped string/boolean/integer/number conversion path,
including whitespace preservation for strings, boolean normalization, integer
normalization, finite numeric checking, original source maps and control polling.
Other inventory members return an absent conversion capability. Unsupported
conversion is distinct from rejection of a particular input. Arbitrarily wide
integer text remains intact in the returned typed lexical value.

`cem_ql::datatype_shipped::register_validation` binds the host's chosen capability
to an original retained behavior profile. It requires matching kind/value/result
signatures and the exact declared `cemml:datatype:<name>` primitive identifier.
Authored names alone never register implementations. String-backed lexical and
symbolic values use the shared predicates; booleans and integers require their
typed atoms, and numbers require a finite decimal atom. List profiles receive
ordered string values, reject empty sequences, and validate each item under its
original lexical predicate without joining, splitting or rewriting it.
Rejections return explicit acceptance plus diagnostics attributed by the shared
result adapter. Runtime control and validation budgets remain the caller's.

This completes shared lexical contracts and native validators, not full shipped
conversion integration. The native scalar bridge below now shares the rendering scalar view and preserves
wide integer identity through signatures, results and exact query comparison. Lexical/list converters and the controlled content-model consumer are implemented
below. The URI declaration prose now agrees with the existing absolute-URI
validator; broader URI-reference admission remains a deferred extension.
Automatic attribute type activation stays guarded pending those parity tasks.


## Native scalar conversion with retained wide integers

Implemented 2026-10-08. `datatype_shipped::converter` creates explicit native
string/boolean/integer/number registrations using the original datatype source,
lexical input and the shipped canonical output representation. The host must
select the capability; names and authored references grant no implementation
authority. Conversion reuses the existing normalization kernel, preserves source
maps and operation control, then lets `ExecutableDatatype::convert` validate every
effective restriction on its one result. Unsupported families remain unavailable;
invalid lexical input is rejected without falling back or narrowing values.

The separate `constant_interpreter` registration reuses that kernel only for the
explicit constant-preparation operation. It works with no runtime converter
selected. Each prepared token retains its original declaration owner and decoded
span and is validated under the original effective datatype before publication.
Diagnostics retain lexical source maps and receive the caller's original attribution.

Rendering and these capabilities now share the existing `cem.typed-atomic` view.
Wide integer values retain normalized text, `integer` datatype and their source
map. The concrete immutable view, not public field names or representation-id
strings, determines its registered scalar representation. Ordinary integer atoms
retain their existing path; an ordinary decimal atom does not become an integer
because its text contains only digits. Conversely, a retained integer's decimal
query projection does not let it satisfy a decimal datatype signature.

Conversion-result and constant-result adapters check retained scalar identity
before inspecting a cached primitive projection, then publish the original value
handle. Native validators also recognize the retained integer representation.
Query comparisons involving retained integers use the shared exact signed-integer
lexical comparator, preserving distinctions beyond floating-point and `i64`
precision. Comparison with ordinary integer atoms remains exact; this change does
not introduce new mixed-numeric coercion or rewrite general query arithmetic.
Registered query equality therefore supports exact membership of wide integer
constants without stringification or an implicit runtime conversion.

Input/output limits, cumulative preparation and diagnostic budgets, cancellation,
explicit rejection and package readiness remain on the existing consumer paths.
The controlled grammar consumer and URI reconciliation are implemented below.
Remaining metamodel parity and native attribute activation are actionable in `todo.md`.


## Adopted shipped lexical and list conversion contracts

Adopted and implemented 2026-10-08. Explicit lexical conversion trims boundary
Unicode whitespace and applies the existing shipped validation predicate. Its
string result preserves all remaining spelling, including case, percent escapes,
internal whitespace and symbolic prefixes. It carries the original source map.
This applies to identifier/qualified-name/symbol-reference/wildcard-name,
URI/semver/media-type/path, type-reference/wildcard-type-reference and unresolved
disposition. URI admission remains absolute-only pending the separate declaration
prose reconciliation. These values grant no namespace, resource or lookup authority.
Separate constant interpretation supports the same scalar contracts without
selecting or invoking a runtime converter; `@values` remains token syntax.

Explicit shipped list registrations select nonempty bounds and the existing
Unicode-whitespace tokenizer. The original identifier or wildcard-name declaration
remains the item contract and must have its own registered validator. Conversion
uses the consuming descriptor's selected tokenizer and returns ordered strings,
then the shared consumer validates every original item rule and effective list
restriction. No item conversion runs implicitly and no list constant syntax is
introduced. A host-provided tokenizer is honored; absent tokenization cannot be
silently supplied at execution.

Tokens retain one shared decoded input and source map plus individual UTF-8 spans.
Spans are not projected into source-file coordinates: escaped/decoded input can
have different byte lengths. Duplicate occurrences and token order survive.
Conversion byte/output caps bound tokenization and retain a distinct limit stop;
operation control is checked through tokenization, construction and validation.
Unavailable tokenizer registrations block package replacement readiness, retaining
the last complete schema/converters/artifacts while the original candidate retries.

Scenarios for later design verification: a namespaced symbolic string never resolves
its target; percent escapes and media-type case survive unchanged; an empty shipped
name list fails even when the authored minimum is zero; a decoded token span never
masquerades as a source-file offset; a replacement with unavailable tokenization
cannot displace the active package.


## Adopted controlled content-model grammar and URI compatibility

Adopted 2026-10-08. The shipped grammar syntax admits empty input or alternatives
of whitespace-separated terms, with lexical QNames, a standalone wildcard and
nonempty parenthesized models. Names/groups support a single adjacent `?`, `*` or
`+`; a wildcard has no extra postfix. Alternatives use `|`; empty branches,
unmatched groups and repeated postfixes fail. This includes the existing CSS
`(animation-name-slot | animation-value-slot)*` declaration. Sequence binds within
an alternative; grouping is explicit. No namespace lookup or document matching
runs while validating syntax.

The consumer scans bounded UTF-8 tokens and uses an explicit stack. Default
byte/token/depth limits are 1 MiB, 100,000 and 64, overridable by the registering
host. Cancellation is checked during scanning and parsing. Syntax rejection,
limit exhaustion and interruption remain distinct. Native validation with exhausted
limits stays incomplete. Explicit conversion produces grammar text with only
boundary whitespace trimmed, retains its original source map and is followed by
all effective restrictions. Invalid spans refer to decoded text, not fabricated
source offsets. Constant preparation parses existing whitespace-delimited tokens
under a separate registration and does not invoke the converter. Registered
equality remains authoritative; no structural grammar equivalence is inferred.

The generic URI declaration prose is corrected to `absolute URI with scheme`.
Existing absolute-URI compatibility predicates and conversion behavior stay intact.
Relative URI admission requires an explicit future contract rather than incidental
broadening during reference adoption. This change supplies neither strict RFC URI
validation nor resource loading authority.

Scenarios for later design verification: all shipped content-model strings parse;
malformed groups cannot acquire readiness through a string base; absent grammar
registration stays incomplete; parser limits never imply rejection or acceptance;
URI fragments do not become native scope references. The maintained package README
owns the public consumer syntax and API contract. Full document matching and future
URI-reference extensions are separately actionable in the roadmap.


## Implemented original shipped parity and attribute type binding

Implemented 2026-10-08. All 18 original CEM-ML datatype declarations compile together
through explicit host registrations. The shipped implementation factory records
cross-kind compatibility for content-model/string and type-reference/symbol-reference
bases, list item contracts and nonempty shipped list bounds. Conversion and validation
checks retain original declaration/base/item owners and typed enumeration restrictions;
shipped names without registrations remain incomplete.

The schema metamodel now uses dedicated `schema:attribute-type` for its type field.
Literal values preserve existing QName checks. Native values are structurally admitted
on attribute declarations only and remain pending until the consumer binds and activates
the original contract. Other type fields retain their existing native-value rejection.

The explicit CEM-QL binding API resolves literal and native type slots through the
same bounded resolver. Literal bindings are supplied for the exact original slot;
they do not borrow a nearby datatype declaration's aliases. Native expressions use
the original scope's current context. Both require one exact compiled original type
and honor directed crossing grants. Pending expression inputs, incomplete compilation,
missing descriptors and traversal failures produce no usable binding. An already
completed literal binding does not require unused expression inputs. Binding itself
does not clear model readiness, prepare values or execute conversion/validation.
Automatic literal/native attribute validation remains actionable in `todo.md`,
using the adopted lexical-preparation contract below.

Scenarios for later design verification: package replacement with the same name and
node IDs cannot substitute for the selected original owner; a QName binding cannot
grant a scope crossing; incomplete native bindings never turn into scalar strings;
attribute validation cannot gain conversion-only lexical admission accidentally.

## Adopted lexical preparation for attribute validation

Adopted 2026-10-08. A literal attribute consumer prepares typed validation inputs
through a separately registered lexical-preparation capability. The capability
belongs to the original datatype implementation and is inherited with that
implementation through compatible bases. Names, reference selection and a
registered converter alone confer no preparation capability. Existing typed
inputs continue directly to descriptor validation; this does not establish
lexical provenance for the combined attribute consumer. Its pretyped extension
uses the contract below. Node-valued inputs retain their native consumer path.

Preparation receives the authored lexical text and its original attribute/source
provenance under the consumer's lifecycle context. It produces the datatype's
declared validation representation while preserving the authored text and source
handle separately. For example, integer `003` can supply typed integer `3` to a
rule without rewriting the attribute. Preparation must preserve the datatype's
lexical admission contract: shipped boolean validation cannot acquire acceptance
of conversion-only `1` or `0`. Existing whitespace/facet rules remain constraints
of the attribute consumer; preparation cannot silently widen them.

List preparation uses the effective registered tokenizer and item preparation
contracts, retaining item order and token provenance. It does not call item
converters. Whole-list derivatives retain these exact contracts. Preparation never invokes
conversion or normalization capabilities implicitly, and it cannot bypass any
inherited datatype restriction, enumeration or attribute-local facet. Prepared
values must pass all effective validation contracts before acceptance.

Completed preparation, lexical rejection and incomplete execution are distinct
outcomes. Missing or unavailable required capability, pending lifecycle inputs,
malformed output, cancellation and exhausted limits cannot become acceptance or
trigger a converter/string fallback. Capability selection retains its original
declaration, scope and implementation identity across package snapshots. Execution
uses the caller's control and cumulative bounds for preparation and subsequent
validation. Missing required preparation prevents automatic consumer activation;
a per-input interruption leaves that validation incomplete.

Native registration and execution are implemented on 2026-10-08. There is no new
authored attribute or behavior syntax. The explicit `prepare_lexical` descriptor
API prepares scalar inputs or composes the registered tokenizer and original item
preparer, then validates all effective datatype restrictions. It retains complete
original input and ordered decoded token spans. Shipped boolean presence, wide
integers and existing boundary-whitespace admission remain compatible.

Typed-only descriptors can compile without preparation. Explicitly unavailable
selection prevents readiness; selected list preparation also requires its tokenizer
and item preparer. Derived descriptors inherit the original base preparer by
default. Ordinary `Ready` selection on a derived declaration is rejected;
`CheckedReplacement` explicitly retains every original base admission check under
the [checked replacement contract](#checked-lexical-capability-replacement-adopted-and-implemented-2026-10-09).
Registering a child capability cannot widen base admission or change its prepared
value. Existing typed and node-valued validation paths bypass preparation.

Preparation calls, input bytes and output counts are bounded. Input visits and
diagnostics share cumulative allowances with subsequent validation. Control is
checked before and after callbacks and during tokenization. No partial preparation
value is exposed; a completely prepared value may be inspected after validation
rejects or stops, but publication requires explicit completed acceptance. Package
replacement retains its last complete active version while preparation is unavailable.
Automatic attribute-local facet integration and activation remain in `todo.md`.

Scenarios for later design verification: boolean presence and lexical forms keep
their existing admission; integer spelling and source maps survive preparation;
lists preserve duplicate items and token provenance without converter calls;
derived/local restrictions still intersect; cancellation and replacement owners
cannot expose partially prepared values as a completed validation result.

## Adopted pretyped attribute input provenance

Adopted 2026-10-09; sealed native issuance, inspection, combined consumption and
the registered native core handoff with lifecycle checks are implemented.
The first pretyped scalar/list attribute consumer accepts an immutable, sealed
preparation result from its own registered lexical preparation path. It retains
the resulting typed values together with evidence of the original lexical input.
It does not accept a freely assembled pair of typed values and a string.

This permits native handoff between preparation and attribute validation without
preparing the same input again. It preserves existing attribute admission and
facet semantics. General typed values remain usable with descriptor validation
and explicit list serialization. Extending the combined attribute consumer to
values with no lexical derivation requires a separate typed facet contract.

### Input states and evidence

| Supplied input | First pretyped attribute contract |
| --- | --- |
| Original lexical input | Existing preparation and combined validation path |
| Sealed preparation result for this attribute binding and live invocation | Consume the retained typed sequence and original lexical evidence |
| Bare typed scalar/list, including an empty sequence | Incomplete: lexical provenance is unavailable |
| Typed values plus caller-supplied text, source maps, token spans or a public preparation report | Incomplete: those fields cannot establish a preparation relationship |
| Explicit conversion result with original input attached | Not preparation evidence; conversion may admit different lexical forms |
| Canonical serializer output | Explicit export only; cannot stand in for original lexical input |
| Authorized native node sequence | Existing node consumer; no scalar extraction or lexical provenance requirement is added |

Missing attributes remain the enclosing validator's absence/default case. A
present empty scalar spelling, a present empty list and an absent attribute are
distinct. Scalar evidence contains exactly one typed value, including when the
original spelling is empty. List evidence contains the complete ordered sequence,
including zero items when preparation admits it; ordinary effective bounds still
decide whether that sequence is valid.

### Sealed native preparation result

The result is an immutable native handle with private construction. The following
is its required information, not a new source syntax or an implemented API type:

| Retained information | Required relationship |
| --- | --- |
| Attribute binding | Exact original attribute declaration, type slot, executable descriptor snapshot and facet registration; equal names or arena-local IDs are insufficient |
| Preparation identity | Exact bound scalar preparer, or list preparer plus tokenizer and item preparer; implementation strings alone are insufficient |
| Original lexical input | Complete decoded lexical text and original source-map stack, retained separately from typed values and any prepared lexical slices |
| Source provenance | Original input attribute or `@default` owner/handle, origin role and exact restricted candidate view when available; an explicitly source-less origin otherwise |
| Typed values | Complete immutable scalar/list representation produced by that preparation; original native atomic views and their access boundaries remain retained |
| List occurrences | One ordered decoded UTF-8 token span per item, with original input ownership, preserving duplicate occurrences |
| Invocation identity | Opaque identity for the live consumer invocation, its input/context revision and bound publication; cancellation and replacement invalidate use |

The preparation path issues this handle only after lexical preparation completes
and its representation, cardinality and token-span boundary checks succeed. No
partial value, pending callback or rejected lexical form can issue it. Issuance
proves which registered preparer produced which values from which input; it is
not a cached datatype or attribute acceptance verdict. A complete preparation may
still produce values rejected by later rules, enumerations or local facets.

The existing public `DatatypePreparation` inspection report has mutable public
fields and is not this handle. Wrapping such a report, copying its fields or
attaching an original source map cannot mint evidence. The future implementation
must issue evidence inside the checked preparation boundary and expose read-only
inspection. Trusted registered preparers retain their existing responsibility for
lexical admission and immutable output; names or matching value representations
cannot select a different issuer.

The original candidate remains distinct from both typed `value` and lexical
evidence. Its metadata still describes the authored attribute; preparation must
not replace the candidate's lexical value with normalized text. Capturing a source
owner does not grant navigation to ancestors, siblings or unevaluated payloads.
A source-less lexical invocation may issue evidence only when its registered
capabilities permit the absent candidate, and remains explicitly source-less.
It cannot later satisfy a required candidate by inventing a node. Diagnostic
fallback text/locations never grant source ownership or candidate access.

### Scalar and list correspondence

Scalar evidence retains the complete original spelling and the one prepared
value. For example, integer spelling `003` and typed integer `3` remain separate.
Neither rendering `3` nor checking scalar equality proves that it came from
`003`. A converter's boolean result `true` from lexical `1` cannot become
validation-preparation evidence for a contract that rejects lexical `1`.

List evidence retains the full decoded input and the tokenizer's checked spans.
There is exactly one span per scalar item in sequence order. Spans are nonempty,
in bounds, ordered, nonoverlapping and on UTF-8 boundaries; skipped text obeys the
registered separator policy. These checks belong to the original tokenization
boundary. The consumer checks the sealed correspondence without tokenizing again.
An empty list has zero spans but still retains its explicitly supplied lexical
input. Separators, Unicode whitespace and duplicate spellings remain observable.

Token spans index decoded input bytes, never guessed offsets in the original
source document. Decoding or whitespace preparation must not rewrite the original
source-map stack. Per-item failures retain the item occurrence index and decoded
span, plus the original attribute/source frames where available. A fabricated
document offset is not an acceptable replacement for that attribution.

Cloning the opaque handle shares its immutable snapshot. Replacing, reordering,
filtering, concatenating or deduplicating its values produces a different typed
input without this attribute evidence, even if each item still has a source map.
The first contract admits no caller-supplied equality or span-matching shortcut.
Native atomic adapters must provide immutable views for the invocation; a live
mutable view without revision tracking cannot be sealed as an immutable result.

### Consumption, facets and outcomes

The consumer checks the binding, input and invocation identities before executing
rules. It then validates the exact retained typed sequence against every effective
datatype rule, enumeration and bound under the current invocation. It applies the
bound local facet contract to the retained original lexical text and, for lists,
the actual typed sequence count. Existing whitespace and lexical facet policies
operate on that original text; they do not mutate the text or typed values.
Attribute-local `values` retains lexical comparison, while datatype enumerations
retain registered typed equality. No comparison policy is inferred between them.

For example, an integer attribute's local lexical `values` restriction can admit
original `003` while a datatype rule receives integer `3`; substituting canonical
`3` would change that lexical comparison. Existing facet applicability still
applies: this does not permit string-only `pattern` or `length` facets on integers.
A list authored as `a  a` has two items,
two original occurrences and its original separators, even if an explicit export
would be `a a`. The local facet phase cannot substitute that export. Presence and
default handling still belong to the enclosing attribute validator.

No preparer, tokenizer, converter or serializer runs implicitly during pretyped
consumption. Original preparation evidence replaces only the preparation phase;
all required validation and facet phases must complete. An earlier accepted report
does not skip use-site checks. Existing declaring-scope diagnostic dependencies
and default checks remain activation prerequisites. Defaults retain their original
`@default` source and cannot issue evidence on behalf of a hypothetical use site.

All existing scalar/list facet families require lexical evidence in this first
extension, even when the declaration has no explicit lexical facet fields. The
current scalar family also performs lexical domain admission. It is therefore
insufficient to look for an absent `pattern` or `length` field and assume lexical
text is unnecessary. Evidence-free values do not bypass that admission.

Complete rejection by a datatype rule or local facet is `accepted = Some(false)`.
Missing, unrelated, malformed or stale evidence, a missing required candidate,
unavailable context, cancellation or a limit stop leaves acceptance unset and
publishes no successfully validated value. Reports distinguish missing provenance,
binding/input mismatch and stale invocation from completed data rejection; they
retain original inputs and phase evidence for inspection. Acceptance requires
every phase plus a final operation/publication validity check.

### Lifetime, limits and transport

Evidence belongs to one live consumer invocation. Its issuer captures the exact
bound consumer and immutable input/context snapshot; the embedding must establish
that invocation before preparation. This is a private native identity, not a
caller-selected integer or a hash of source text. Changed input, context roots,
candidate access, effective grants, diagnostic bindings or datatype/facet
publication requires a fresh invocation and preparation. A cancelled/completed
invocation cannot be revived with a new operation control. Explicit datatype
override activation also invalidates evidence from the replaced binding.

Clones may be inspected or consumed within the same live invocation; they never
replenish its budget. Issuance and consumption share cumulative preparation,
input-visit, rule, comparison and diagnostic allowances and query execution
control. Preparation is charged once when performed; evidence inspection and each
subsequent validation are charged when performed. Existing default bounds of
1 MiB lexical input and 100,000 prepared values remain applicable. Validate
receipt sizes and per-item span metadata before allocating or executing rules,
and check control during bounded inspection and before publishing acceptance.

The core/engine boundary must carry an opaque retained native handle that the
issuing engine can check. Core CEM-ML must not depend on CEM-QL `Item`, and a
public trait object's claim of a compatible representation cannot mint evidence.
Retain owners and views directly: no JSON, reconstructed AST, string round trip,
or scalar-record substitute is an internal handoff. No receipt or invocation
authority enters CEMB, reload metadata, package artifacts or worker messages.
After reload or a cross-heap transfer, re-establish original lexical input and
current capabilities and prepare anew; exported text alone has no provenance.

Completed implementation and fixture evidence is preserved in
[archived checklist](archive/todo-snapshot-2026-10-10.md#general-datatype-compilation-design). The sealed native consumer adds no
source syntax, external provenance issuers or node items within scalar lists.
The separate typed-only contract below admits external values without claiming
lexical derivation. Whole-list derivatives use
the same sealed native consumer with their effective inherited contracts.

### Implemented native issuance and inspection

`cem_ql::preparation_evidence::AttributePreparationInvocation::new` freezes an
exact `BoundAttributeFacets`, `PreparationInput` and validation runtime. Calling
`prepare` executes the selected scalar preparer or list tokenizer/item preparer
without running datatype rules or local facets. Its result contains a public
inspection report and, only after complete checked preparation, an opaque
`SealedPreparationEvidence`. The report has no validation or acceptance verdict;
changing its values, lexical input or spans cannot change or mint the receipt.
Existing `prepare_lexical` still performs its normal validation.

Evidence retains original lexical text and source maps, ordered values and decoded
item spans, original candidate access, exact binding identity and the issuing
invocation. Read-only getters expose these retained values. `verify` checks the
same live invocation; a freshly compiled lookalike binding has a different identity.
Primitive atomic values and the engine's private immutable scalar/token adapters
are admitted. A public representation claim cannot establish immutable storage.
Original candidate issuance currently requires an original retained literal
attribute whose decoded text and source maps match the input. Dynamic attribute
values require a separate derivation proof. Optional source-less inputs remain
explicitly source-less; required candidates are never fabricated.

Invocation clones share cumulative preparation, output, input-visit and diagnostic
allowances and query control. Reentrant issuance stops before invoking another
callback. An unwinding callback conservatively spends its reserved allowances.
Cancellation, explicit `close`, query failure or dropping all invocation owners
expires authority. A standalone embedding must call `close` before input, context,
grants or publication change. The registered core handoff adds lifecycle checks
and publication retirement as described below. Retained
evidence remains inspectable after expiry but cannot verify as live.

Original lexical and retained typed text obey the configured lexical byte bound.
Additional source and diagnostic metadata has a 1 MiB bound, inspected before
receipt cloning; nested diagnostic control metadata is limited to depth 64.
Ordered output and span counts obey preparation limits. No receipt constructor,
codec or persisted authority is exposed. Combined consumption runs fresh typed
validation and local lexical facets in the same invocation, as described below.

### Implemented explicit combined pretyped consumption

`BoundAttributeFacets::validate_pretyped` accepts the issuing
`AttributePreparationInvocation`, optional sealed evidence, a current host-supplied
`FacetContext` and a finite facet model byte limit. Its runtime and remaining
preparation/validation allowances come from the invocation; callers cannot supply
a replacement runtime or replenish those allowances. The embedding must close
the invocation before changing runtime or facet context. The registered core
handoff below supplies the corresponding lifecycle boundary.

Missing evidence, a different binding or invocation, expired authority and budget
or callback stops produce incomplete validation (`accepted: None`). A present
empty list has real evidence and undergoes validation. Complete consumption runs
all effective datatype rules, cardinality restrictions and inherited/local typed
enumerations, then intersects local facets using the original lexical text and
retained list count. Original `003` can match typed enumeration `3` while failing
attribute-local lexical `values=3`. Source-less evidence cannot satisfy a required
rule candidate. No preparer, tokenizer, converter or serializer executes during
consumption; bare typed values have no ingress through this API.

`AttributeDatatypePhase::Pretyped` retains the sealed evidence alongside the
fresh datatype report; facets remain a separate phase in the combined report.
Datatype item reports retain occurrence indices and ranges into completed rule,
enumeration and cardinality reports, plus the stopped item index when applicable.
The corresponding sealed token span supplies the original decoded occurrence;
duplicate values do not collapse and document offsets are not fabricated.
Neither report confers authority to another invocation or caches acceptance.
Receipt inspection spends input visits, including one visit for an empty receipt.
The complete scheduled rule/input work is conservatively charged even if a later
callback stops. Comparisons and completed, stopped-rule and local-facet diagnostics
spend their actual allowances; a facet limit with hidden partial diagnostics spends
the remaining diagnostic allowance. A reservation excludes callback re-entry and
unwinding spends the reservation. Final invocation/control/query checks prevent
acceptance after cancellation or closure during validation.

### Implemented native core handoff and publication lifetime

`CompiledAttributeDatatype::prepare` receives explicit original lexical input and
a live `AttributeDatatypeContext`. Successful issuance returns a
`NativeAttributePreparation` handle with no acceptance verdict. The embedding
passes it as `AttributeDatatypeValue::Prepared` to the registered consumer, along
with the current original source, facet inputs, operation and context. The core
container retains an opaque process-local engine payload and has no CEM-QL `Item`
dependency. Its public payload constructor is not authority: CEM-QL downcasts to
its private issuer type and checks the exact binding, sealed evidence and live
invocation. A string, typed vector, copied report or different issuer stays
incomplete; the consumer never falls back to preparation or conversion.

The engine retains the exact original source tree and attribute handle, preserving
the restricted candidate view, lexical text and source maps. Equal node IDs in
another owner do not match. Source-less input remains source-less and cannot gain
candidate access at consumption; required candidates remain incomplete. Literal
defaults retain their original declaration owner and `@default` attribute.
Core context metadata is bounded before cloning or invoking a preparer, and source
metadata is bounded before additional retention. Receipts share the original
preparation/validation budgets, including repeated calls through cloned core handles.

`AttributeDatatypeContext` is the host-owned context/grant epoch. Its clones share
one private identity; `advance` closes that identity before creating a new one,
and `close` or dropping all context owners expires its leases. Hosts must advance
the epoch when external context roots or effective grants change; those external
mutations are not inferred from public names or source IDs. Each consumption also
checks the actual source owner, attribute contents, element name, complete local
attribute-value map and exact `OperationControl` instance. A mismatch permanently
closes the original invocation. Reusing a numeric operation ID cannot revive it,
and cancelled/completed controls remain stopped. Final checks after native
callbacks prevent a handle or acceptance from escaping a context/publication change.

Ready model replacement retires preparations of replaced registered contracts.
Package staging and rejected or incomplete overrides preserve the old publication
and its live preparations. Package and explicit datatype override commits retire
old preparation authority only after all fallible checks pass, including when a
caller still retains the old model. A newly committed consumer requires fresh
preparation. Standalone descriptor/lexical APIs retain their existing explicit
contracts and do not gain prepared authority from an old report.

No native handle or context identity enters CEMB, package artifacts, worker data or
reload metadata. Reloaded source must establish current capabilities and prepare
anew. Existing lexical and authorized-node paths remain available; no source-format
syntax or implicit pretyped fallback is introduced.


## Implemented original attribute-constraint retention

Implemented 2026-10-08. Successful literal/native attribute type bindings now retain
the source attribute's local constraint metadata and all original field handles.
The projection preserves the original declaration/source spans and the authored
native-type readiness flag; obtaining a descriptor does not activate the attribute.
Effective local metadata follows existing last-authored-slot precedence, with all
original occurrences still available for attribution. Unresolved effective names
or dynamic constraint values leave binding pending. Namespaced fields are rejected
by this consumer instead of being collapsed into unqualified facet names. Missing
or empty declaration names cannot produce a usable binding.

Constraint retention is independent of facet applicability and execution. The
explicit inherited facet profile below supplies specialized URI/path semantics;
scalar representation and authored datatype names do not choose that behavior.

Scenarios for later design verification: resolving `@type` cannot discard a pending
`@pattern`; a foreign namespaced `pattern` cannot become a local facet; reused
attribute declarations keep their original constraints and owners; binding alone
cannot clear native-type readiness or make local constraints optional.


## Adopted and implemented inherited facet profiles

Adopted and implemented 2026-10-08. The host explicitly registers a facet profile
against an original datatype declaration and scope, with an implementation identity
and a representation-compatible facet family. Derived descriptors inherit that
same registration. Duplicate or unrelated registrations fail; an explicitly
unavailable profile leaves compilation pending. A descriptor without a profile
can support typed validation, but cannot compile this local facet adapter.
Ordinary `Ready` selection cannot replace a base profile. Explicit checked
replacement follows the contract below.

The local adapter compiles applicability and syntax using the shipped validators.
It checks scalar domain admission and local URI/path/string/numeric restrictions;
local whitespace handling does not rewrite the original input or prepared value.
Attribute-local `values` retain their lexical comparison, independently of datatype
enumerations' registered equality. Lists use the supplied prepared sequence count
without another tokenization pass. Native node sequences use counts without scalar
extraction and preserve existing attribute singleton defaults. Original declaration,
constraint fields, source spans and registration identity remain available.

Model and input bytes and diagnostic counts have explicit limits; runtime control
is checked around validation. A local facet verdict covers only local constraints.
Overall acceptance still requires completed datatype preparation/validation and all
inherited restrictions. Callers supply the original input and declaring-scope
diagnostic bindings. The combined explicit adapter below shares phase limits.
Defaults and diagnostic dependency readiness are checked by the explicit declaration
API below and integrated package lifecycle. Neither profile
selection nor local compilation clears the source model's readiness guard.

Scenarios for later design verification: an alias named `uri` gains no profile by
name; a replacement source owner cannot satisfy an old registration; derived local
restrictions intersect the datatype contract; list counts do not retokenize lexical
input; native values are never atomized; interruption cannot publish acceptance.


## Checked inherited facet-profile replacement (adopted and implemented 2026-10-09)

`FacetProfileBinding::CheckedReplacement` explicitly selects a new profile for an
original derived datatype. The compiler retains the effective base registrations
in oldest-first order and appends the selected registration. A derivative without
its own selection inherits that complete sequence. Every registration keeps its
original declaration, scope, implementation identity and family. Equal names or
implementation IDs never merge registrations or authorize substitution. The
existing `facet_profile()` accessor returns the selected registration;
`facet_profiles()` exposes the complete effective sequence.

The selected and all inherited families must describe the datatype's same value
representation. Scalar, whole-list and native-node descriptors use this contract;
a list's item profiles remain attached to its shared item descriptor. A checked
replacement without an inherited base is invalid. A base without a profile is
pending, and explicitly unavailable ancestors block compilation. Ordinary `Ready`
on a derived datatype remains invalid. No new authored syntax, serialized evidence
or implicit authority from datatype override grants is introduced.

### Local contract compatibility

`compile_facets` compiles the original attribute model under **every** effective
family. All must admit the authored fields and their syntax. The returned
`AttributeFacetContract`, including direct callers of `contract()`, owns this
complete intersection. It cannot expose a selected-only validation shortcut.

This deliberately admits only the common local-field vocabulary. For example,
URI-to-string replacement with no specialized fields retains the URI admission
check. With `uriHosts`, the string profile cannot compile the local contract;
that restriction is never dropped. Two URI profiles both honor `uriHosts`, and
two path profiles both honor `pathExtensions`. A string-to-URI replacement can
narrow domain admission when the complete local model is supported by both.
Routing selected fields to individual profiles is a separate extension requiring
an explicit ownership contract.

At consumption, profiles validate the same original lexical text or prepared
sequence count, oldest first. Each profile's established whitespace handling is
local; no normalized value is fed to another profile or written over preparation.
Stop at the first rejection or incomplete result. Acceptance requires every
profile to accept and the independent datatype phase to accept. Lists do not
retokenize or add item constraints; native nodes are counted without atomization.
Diagnostics retain the consuming attribute's original source and declaring-scope
bindings. Profile registration owners remain separately inspectable.

### Limits and lifecycle

The datatype compiler charges profile retention against its shared traversal work
budget before copying a sequence. Local compilation preflights aggregate model
accounting: each profile charges the original model's accounted bytes, with a
minimum of one byte per profile. Runtime applies that same aggregate model bound
and preflights original lexical byte length times profile count against
`FacetLimits::max_input_bytes`. Checked multiplication overflow is a limit failure.
Thus even an empty lexical input or native-node sequence has a bounded profile
count. Single-profile limits keep their existing meaning.

Runtime control is checked before the chain and around each profile. A successful
local profile has no diagnostics; the first rejected profile spends the shared
diagnostic allowance remaining after datatype validation and ends the chain.
Exhausted limits and cancellation stay incomplete and cannot publish acceptance.

Defaults and lexical, sealed pretyped and native-node consumers all execute the
same composite contract. Sealed preparation retains typed values, not a cached
facet verdict; each consumption revalidates every profile. A committed attribute
contract replacement retires prior preparation authority. Invalid local fields,
failed defaults and unavailable profiles preserve the last active package and
its receipts. Authorized dependency rebinding rebuilds the sequence from the new
effective base; unlisted edges and old immutable descriptors keep original owners.

## Typed-only external attribute admission (adopted and implemented 2026-10-09)

External producers can supply immutable primitive scalar values or ordered scalar
lists without claiming that those values came from any authored spelling.
Admission requires an original host registration of `FacetFamily::TypedScalar`
or `FacetFamily::TypedList` with the exact primitive representation. Missing
`pattern`/`length` fields, a familiar datatype name, or a successful typed rule
does not authorize this path. Existing lexical profiles and sealed preparation
keep their original contracts.

### Explicit omission and dependency contract

Only effective `Scalar` and `List` datatypes can opt in. The compiler requires
every inherited profile, effective base and list item descriptor to admit typed
ingress. Scalar items retain their exact descriptor; whole-list restrictions
still intersect. An absent base/item profile, a lexical/grammar/reference kind,
any inherited or selected preparer, or any tokenizer prevents typed-only
compilation. Checked profile replacement cannot mix typed and lexical families.
An unavailable registration remains pending. Original source/scope checks and
bounded dependency rebinding are unchanged.

The omission contract is explicit:

| Concern | Typed-only behavior |
| --- | --- |
| Original spelling, separators, normalization and token spans | None are asserted or synthesized. No preparer, tokenizer, converter or serializer runs during ingress. |
| Shipped lexical domain checks | Not supplied by the profile, including URI/path/media/name syntax. A tagged primitive is not lookup or navigation authority. Required value-domain restrictions belong in registered typed rules. |
| Scalar local facets | No authored local value restrictions are admitted. String pattern/length/content, whitespace, numeric bounds/digits, URI, path and media fields all fail compilation. Numeric spelling facets are not reinterpreted as typed comparisons. |
| Attribute-local `values` | Rejected because it compares lexical spellings. Datatype enumerations retain their original registered typed equality and constant owners. |
| List local facets | Only `itemCount`, `minItems` and `maxItems`, evaluated against the supplied sequence count. No item parsing or retokenization. |
| Literal defaults | Rejected, including empty defaults. No implicit lexical preparation or synthesized typed default. |
| Metadata and diagnostics | Original name/type/readiness/source metadata and diagnostic selectors remain available; declaring-scope diagnostic dependencies still require readiness. |

Local applicability uses a whitelist: after removing admitted metadata and list
counts, any remaining nondefault model field is invalid. Future fields therefore
fail closed. Typed and lexical `FacetInput` variants are disjoint; a direct local
facet verdict still requires the independent datatype phase for overall acceptance.

### Producer and value authority

`ExternalTypedProducer::new` is an explicit native host grant for one exact
`BoundAttributeFacets`. Its name is inspection metadata, bounded to 4096 bytes;
equal names do not merge grants. Clones share revocation. `close` expires all values
from that producer; creating another producer with the same name cannot revive
old values. Standalone hosts own the lifetime of these explicit grants.

`produce` takes an explicitly present owned sequence and checks cardinality,
representation, immutable storage, operation control and limits. Scalar input
must contain exactly one item. A present empty list differs from absent input.
Order and duplicates are preserved. Primitive atoms and the engine's private
immutable scalar adapter are admitted; arbitrary native views, nodes, public
representation tags and token-view substitutes cannot assert immutable storage.
Wide integers retain their private representation and original owner. Decimal
storage must contain optional sign, digits and at most one decimal point with at
least one digit; whitespace/exponents are not accepted or normalized. Double
values retain IEEE bits, including signed zero, infinities and NaNs. Additional
numeric constraints remain the typed datatype's responsibility.

An `ExternalTypedValue` has no public constructor, mutation, codec or cached
acceptance. Its source maps, when present, are retained provenance metadata, not
proof of lexical derivation. It contains no token spans or preparation receipt.
Host-supplied immutable data may be reused in another fresh live operation/context;
every consumption independently checks current authority and validates again.

### Consumption, budgets and core integration

`BoundAttributeFacets::validate_external` checks the exact binding and live
producer, then runs all effective typed rules, enumerations, cardinalities and
local count facets. Candidate requirements remain effective. The producer grants
no candidate navigation; the host supplies current authorized candidates.
Rejection is `Some(false)`; malformed, absent, unrelated, expired, cancelled or
incomplete work never becomes acceptance. Revocation and control are checked
again after callbacks, so a callback cannot publish after closing its producer.

Production and consumption bound value counts and aggregate immutable text storage
using `PreparationLimits::max_output_values`, `validation.max_input_values` and
`max_lexical_bytes` (a byte-storage cap here, not lexical evidence). Retained source
metadata shares a cumulative 1 MiB cap before any view is atomized. Consumption
checks its own limits before cloning values; rules, comparisons, input visits,
query execution and diagnostics share the existing invocation budgets. Local
profiles retain aggregate model limits. Each external consumption is a fresh
validation, unlike a sealed preparation invocation's cumulative receipt budget.

An active core `CompiledAttributeDatatype::typed_admission()` exposes an opaque
engine capability only for an admitted typed-only contract. The host uses
`ExternalTypedProducer::from_native` and passes a value's `native_handle()` through
`AttributeDatatypeValue::ExternalTyped`. CEM-ML never depends on CEM-QL `Item`.
The engine checks private issuer identity; wrapping arbitrary payloads in either
public opaque container cannot mint authority. External handles and lexical
preparation handles are separate and cannot substitute for each other.

Core consumption requires a live `AttributeDatatypeContext`, the current operation
and publication, bounded current input metadata, and exact original source owner
when supplied. The core exposes only the existing restricted attribute candidate.
The source attribute supplies attribution/context; its lexical text is not claimed
as the origin of external data. Context closure during validation or operation
completion prevents acceptance. Advancing to a fresh context permits a new full
validation of still-authorized immutable data.

Typed-only package activation does not require lexical preparation. Lexical input
and lexical preparation requests against that contract remain incomplete. Invalid
defaults/fields and unavailable candidates preserve the active package and its
admissions. Commit retirement expires old core producer capabilities and values;
new contracts reject old binding identities. No capability or external handle is
serialized, reloaded, sent through workers, or reconstructed through JSON/AST
conversion. After reload, a host must obtain current admission and provide data
through the native producer again.

## Implemented explicit attribute validation composition

Implemented 2026-10-08, extended 2026-10-09. Bound attribute facets provide explicit
lexical, sealed pretyped, external typed-only and native-sequence consumer entry
points. Lexical consumption prepares the original input and executes every
effective datatype restriction before applying local
facets to the original lexical text and prepared sequence count. Native consumption
validates the complete supplied node sequence and intersects local cardinality
constraints without scalar extraction, preparation, conversion or descendant
reference expansion. The host supplies already-completed reference selections and
declaring-scope diagnostic bindings.
Sealed pretyped consumption uses the original invocation and fresh validation
without repeating preparation, as described in the provenance contract above.

The combined verdict requires both phases to accept. Rule rejection remains invalid
even without error-severity diagnostics. Pending/unavailable execution, invalid
input, cancellation and exhausted limits leave the result incomplete. The report
retains original inputs and each phase's inspection data; prior datatype acceptance
cannot publish after a failed local phase. A completed lexical rejection exposes no
fabricated typed value for facet execution.

Preparation and datatype validation retain their shared input-visit bounds. Local
facets receive the diagnostic allowance remaining after preparation, rules,
execution diagnostics, enumeration diagnostics and typed cardinality rejections.
Input/model bytes remain bounded and all phases share the caller's lifecycle
control. Local sequence counts do not iterate or re-tokenize the prepared values.

This is an explicit consumer API. It leaves automatic model activation guarded.
Default-value validation and diagnostic dependency readiness are implemented by the
explicit declaration API below. Package replacement is now integrated as described
in the lifecycle contract; sealed pretyped consumption and the registered native
core handoff are implemented with context and publication lifetime checks above.
Scenarios for later design verification: partial acceptance cannot escape after a
later stop; duplicate native targets retain the exact original query views; list
items retain token spans; a base rejection cannot be weakened by local acceptance;
typed rejection reports spend the same diagnostic allowance as rendered messages.


## Implemented explicit attribute declaration readiness

Implemented 2026-10-08. The consumer checks host-supplied diagnostic bindings against
the attribute's original declaring schema handle. The snapshot includes diagnostic
collection and behavior dependency completeness. Equal names or URIs in a replacement
owner do not establish that identity. Declared diagnostic dependencies remain pending
until their snapshot is complete; a complete snapshot with a missing code or wrong
engine family is invalid. No diagnostic dependency means unrelated catalog work need
not block this declaration. The host remains responsible for compiling and associating
the snapshot with the stated original schema.

Literal defaults use the same registered preparation, effective datatype rules and
local facets as supplied values. The original `@default` attribute provides the native
candidate and source attribution, following the adopted original-`@values` constant
contract. No hypothetical consuming attribute or second source tree is manufactured.
An absent default is distinct from an authored empty value. Effective defaults retain
last-authored-slot precedence and list token spans. A literal default for a node
contract is invalid; this path never turns strings into references.

The readiness report retains original declaration/default/schema handles, typed
issues and the complete default-validation phase report. Explicit default rejection
makes the declaration invalid even if its diagnostics are only warnings. Incomplete
preparation/validation or cancellation keeps it pending. The caller's lifecycle
control and limits apply, with a final control check before readiness is returned.

This API checks declaration prerequisites for one supplied lifecycle invocation.
It does not activate a model, bypass use-site validation in another runtime context,
or authorize reusing a report after its inputs change. Automatic binding/activation
and coordinated package replacement are now implemented as described in the
lifecycle contract. Sealed preparation issuance, combined pretyped consumption
and the registered native core handoff with lifecycle checks are implemented.
Scenarios for later design verification: pending diagnostics never choose fallback
behavior; replacement owners cannot satisfy older declarations by name; original
default candidates retain their source maps; incomplete retries preserve the same
source; declaration-time validation does not replace use-site checks.
