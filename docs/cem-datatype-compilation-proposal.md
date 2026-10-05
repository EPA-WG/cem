# General datatype compilation proposal

Status: temporary design draft, awaiting semantic decisions. General datatype
compilation must be designed before enabling native attribute `@type` consumption.
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
   Preserve current collection precedence during migration. Decide duplicate
   datatype-name behavior explicitly before implementing a registry.
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
| `list` | Ordered values satisfying an item datatype contract | Item representation, delimiter/normalization rules and the meaning of list `values` |
| `grammar` | A value checked by a declared grammar consumer | Binding to existing content-model parsers versus a new grammar language |
| `reference` | A symbolic-reference value contract such as a QName | Separate lexical validation from consumer-requested dereferencing; do not imply URL or AST link evaluation |
| Native node contracts | Retained node input consumed by native validation/behaviors | Preserve the existing explicit node contract and decide how it is represented in the datatype registry |

For inherited scalar restrictions, the recommendation is intersection: the base
and every restriction must hold, including locally authored attribute facets.
This agrees with `AttributeValueContract.restrictions`. Conversion occurs once;
restrictions cannot change the final representation. Type-dependent checks wait
while dependencies are pending; independent malformed rules or facet syntax can
still be reported. This recommendation is not yet a general datatype decision.

## First semantic decision: executable rules

Recommended option: bind executable semantics to explicitly registered,
schema-owned rule implementations, using existing native predicates/parsers and
schema behavior capabilities. Preserve shipped prose as descriptions attached
to their known built-in contracts; do not interpret arbitrary prose as code.
Unknown or unavailable implementations cannot activate a datatype. Exact custom
binding syntax and native behavior input contracts need their own specification.

Alternative: define an executable grammar language for `@rule`, migrate the
existing descriptions to that language, and compile it directly. This is a
larger language feature requiring grammar semantics, normalization rules,
resource limits and its own compatibility plan.

This decision should precede implementing custom `kind`/`rule` consumption.
Neither option requires changing the generic CEM-ML reference node semantics.

## Implementation sequence after decisions

The actionable steps and adjacent verification scenarios are in
[todo.md](todo.md#general-datatype-compilation-design).
Start with an inventory of existing declarations and native contract parity,
then establish the descriptor and lexical registry, bind dependency graphs,
implement kind consumers, and finally enable native attribute type consumption.
Keep function reference contracts, child-scope override syntax, and
`cem-element` ID conversion on their existing deferred tracks.
