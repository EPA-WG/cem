# General datatype compilation proposal

Status: temporary design draft. Registered schema-owned implementations,
native `@rule` binding, separate validation/conversion roles, duplicate-name
rejection, intersecting scalar restrictions and ordered typed list conversion
results, rejection of list declaration `values` and explicit rule acceptance
with diagnostics, empty lists unless constrained and a dedicated `node` kind are
adopted. Node rules validate complete target sequences, and candidate requirements
are per registered capability. Derived declarations inherit an omitted kind from
a resolved base, with explicit list kind for item-base semantics. Whitespace-token
`@values` authoring is adopted; richer retained constants are deferred. Validation
descriptors, sequence bounds, tokenizers and explicit native conversion are
implemented. Separate conversion/equality signatures are adopted; equality/query
adapters and remaining kind contracts remain open. Datatype validation uses the adopted
fixed names `value`, `datatype` and `candidate`. Result construction uses ordinary
CEM-QL records under a dedicated contract enforced at the consumer boundary;
candidate cardinality must be visible to compilation. Diagnostics admit existing
native values and checked records with optional original source nodes. Shared
value-contract declarations are adopted for reusable result/diagnostic shapes.
General datatype compilation must be designed before enabling native attribute
`@type` consumption.
The shared value-contract source surface and explicit result consumer are implemented.
Explicit registered validation dispatch and opt-in datatype descriptor/package
readiness integration are implemented below. Automatic native attribute `@type`
remains guarded pending the remaining compiler and migration work. Separate
registered lexical preparation for literal validation inputs is adopted and
implemented below; automatic attribute activation remains open.

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

## Datatype kind contracts and remaining work

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

The following are proposed semantic input roles, not enabled function names or
metamodel `source` literals:

| Role | Proposed typed input | Purpose |
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

Explicit registered native list conversion now uses the descriptor adapter below.
Shipped lexical list conversion still needs its concrete implementations and
migration fixtures. Existing scalar conversion
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
kind inference, dependency resolution and automatic datatype consumer activation
remain guarded; explicit validation registration is available separately.

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

Richer retained enumeration constants remain deferred to a separate future
design item covering original owners, typed scalar admission, references and
compatibility with existing tokens. No repeated `{value | …}` child syntax,
nested quoting or implicit JSON interpretation is adopted. Do not treat the
whole `@values` attribute as a single constant.

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

The actionable steps and adjacent verification scenarios are in
[todo.md](todo.md#general-datatype-compilation-design).
Start with an inventory of existing declarations and native contract parity,
then establish the descriptor and lexical registry, bind dependency graphs,
implement kind consumers, and finally enable native attribute type consumption.
Keep function reference contracts, child-scope override syntax, and
`cem-element` ID conversion on their existing deferred tracks.

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
Native `@base`/`@rule` targets retain their original owners and source attributes.

The host registers a kind, typed representation, accepted cross-kind bases,
optional bounds/tokenizer and optional default validator for an exact original
datatype and lexical scope. This registration does not follow a same-name vendor
declaration. Omitted or identical derived kinds can reuse the resolved base's
contract. A different kind requires an explicit compatibility entry, and every
base restriction must inspect the same representation. Node/scalar mixing is
invalid even with a compatibility entry. An explicit list's `base` selects its
item contract; the initial executable list representation admits scalar items.
Whole-list inheritance and list-of-list/native-node items remain unsupported.

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
Nonliteral and empty vocabularies are invalid; no additional constant syntax is
introduced. Token interpretation returns one scalar in the registered representation,
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
constant retain the original decoded token span and owner. No richer constant
syntax is introduced. Its function returns `datatype-constant-result`; the checked
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
inputs continue directly to validation; node-valued inputs retain their native
consumer path.

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
converters. Whole-list inheritance remains deferred. Preparation never invokes
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
and item preparer. Derived descriptors inherit the original base preparer. Explicit
replacement on a derived declaration is rejected pending a separate compatibility
contract, so registering a child capability cannot widen the base's lexical input
admission. Existing typed and node-valued validation paths bypass preparation.

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
