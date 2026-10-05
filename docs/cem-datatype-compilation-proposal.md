# General datatype compilation proposal

Status: temporary design draft. Registered schema-owned implementations,
native `@rule` binding, separate validation/conversion roles, duplicate-name
rejection, intersecting scalar restrictions and ordered typed list conversion
results are adopted. Exact signatures and
remaining kind contracts need decisions. General datatype compilation must be
designed before enabling native attribute `@type` consumption.
This draft does not adopt a new executable grammar or enable datatype references.

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

## Proposed kind contracts

| Kind | Proposed meaning | Remaining design work |
| --- | --- | --- |
| `scalar` | One value with an explicit primitive conversion/validation contract and restrictions | Built-in registry binding, normalization and custom implementations |
| `lexical` | A value checked by a declared lexical predicate | Executable `rule` binding and compatibility with shipped descriptions |
| `list` | Ordered typed values satisfying an item datatype contract, with authored lexical source retained | Delimiter/normalization rules and the meaning of list `values` |
| `grammar` | A value checked by a declared grammar consumer | Binding to existing content-model parsers versus a new grammar language |
| `reference` | A symbolic-reference value contract such as a QName | Separate lexical validation from consumer-requested dereferencing; do not imply URL or AST link evaluation |
| Native node contracts | Retained node input consumed by native validation/behaviors | Preserve the existing explicit node contract and decide how it is represented in the datatype registry |

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

## Next decision: list-level values

The general compiler needs an explicit meaning for `{type @kind=list @values=...}`.
Item restrictions already belong to the item datatype selected by `base`; a native
list rule can validate the complete ordered sequence. Existing attribute-local
`values` behavior remains a separate compatibility surface.

Recommended first contract: reject `values` on a list datatype declaration.
Declare allowed item values on its item datatype, and use a registered list rule
for sequence restrictions. This avoids giving one lexical field two meanings and
leaves a future sequence-enumeration syntax to explicit design.

Alternative: make list datatype `values` a shorthand for an item vocabulary,
requiring every item to match an entry in addition to its base contract. This is
more concise, but must specify typed equality, lexical parsing and normalization.
Whole-sequence enumeration would still need a distinct contract.

General empty-list admission and custom tokenizers remain explicit follow-up
work; shipped name-list contracts continue to reject empty values.

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
