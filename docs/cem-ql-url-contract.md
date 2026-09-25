# Pure CEM-QL URL family — contract draft

Status: **proposed, awaiting decisions D1–D3** (2026-09-25). This is not an
implemented API or a normative extension. The active checklist remains
[todo.md](todo.md). Do not register functions or generate public reference
claims from this draft before the decisions are accepted.

## Scope already requested

Provide Tier A `url:` functions in `cem:stdlib/url`, evaluated through one
native semantic path for CLI, SSR, WASM and browser use. Inputs and outputs are
CEM-QL values. The functions must not access import maps, resource policy,
node scope, ambient location, network or filesystem. Relative URL input needs
an explicit base. Contextual resolution remains `module_url()`.

Object-URL creation/revocation and callback-specific `forEach` are excluded.
Use ordinary stream iteration instead. JavaScript URL APIs may be comparison
oracles during development, never the production implementation.

Reference baseline: [WHATWG URL snapshot 8e14777](https://url.spec.whatwg.org/commit-snapshots/8e14777cfa145b08a9fb735fe580ec0c366564c3/).
The [API](https://url.spec.whatwg.org/#api),
[origin](https://url.spec.whatwg.org/#concept-url-origin), and
[form encoding](https://url.spec.whatwg.org/#application/x-www-form-urlencoded)
sections define the external semantics to map. Pin selected WPT vectors to a
revision during implementation, with their license and provenance.

## Existing implementation constraints

- [Type](../packages/cem_ql/src/types.rs) already represents `Record`, `Array`,
  `Stream`, `Empty`, and atomic `AnyUri`. The notation below is descriptive;
  it does not introduce a new surface type-alias or union syntax.
- [Item](../packages/cem_ql/src/eval.rs) stores owned record fields in a
  `BTreeMap<String, Vec<Item>>`. An ordinary record cannot preserve authored
  insertion order. Native record views can expose a different enumeration
  order; choosing whichever happens to occur would undermine portability.
- [record:entries](../packages/cem_ql/src/eval/pipeline.rs) enumerates those
  fields. It does not restore the lost authored ordering.
- [The registry](../packages/cem_ql/src/stdlib.rs) has no URL module.
  `cem-ql` has no direct URL dependency. The workspace lock contains `url`
  2.5.8, but that is a candidate dependency, not evidence of conformance.
  Its typed setters alone cannot establish JavaScript setter parity.

## Proposed value contract

`Text` means exactly one string or `anyURI` atomic item. No implicit node
atomization, numeric/boolean stringification or JSON conversion. `String`
means exactly one string item. A malformed argument has a type/cardinality
error even for a function that returns an empty result on invalid URL syntax.
An omitted optional argument differs from an explicitly empty argument stream.
The latter is a cardinality error except where a parameter stream is expected.

`Entry` is exactly `{ name: String, value: String }`.
`Params` is an ordered `stream<Entry>`; zero entries are an empty stream.
Duplicates are retained. These are ordinary CEM-QL control records, not external
document imports or a serialized AST representation.

`Parsed` is one record with `href: anyURI`; string fields `origin`, `protocol`,
`username`, `password`, `host`, `hostname`, `port`, `pathname`, `search`, `hash`;
and `query: Params`. `T?` below means zero or one result item, not a null item.
The `query` field can therefore contain zero or many items without flattening
the surrounding record.

| Function | Arity | Proposed result / argument shape |
| --- | --- | --- |
| `can_parse(input, base?)` | 1–2 | `boolean`; `Text` arguments |
| `href(input, base?)` | 1–2 | `anyURI`; `Text` arguments |
| `parse(input, base?)` | 1–2 | `Parsed?`; `Text` arguments |
| `assemble(parts, base?)` | 1–2 | `anyURI`; one parts record, optional `Text` base; D3 |
| `with_parts(input, parts)` | 2 | `anyURI`; absolute `Text` input and one parts record; D2 |
| `params(init?)` | 0–1 | `Params`; D1 defines constructor alternatives |
| `params_size(params)` | 1 | `integer` |
| `params_entries(params)` | 1 | `Params` |
| `params_keys(params)` / `params_values(params)` | 1 | `stream<string>` |
| `params_get(params, name)` | 2 | `string?` |
| `params_get_all(params, name)` | 2 | `stream<string>` |
| `params_has(params, name, value?)` | 2–3 | `boolean` |
| `params_append(params, name, value)` | 3 | new `Params` |
| `params_delete(params, name, value?)` | 2–3 | new `Params` |
| `params_set(params, name, value)` | 3 | new `Params` |
| `params_sort(params)` | 1 | new `Params` |
| `params_string(params)` | 1 | `string`, without a leading `?` |

All `params` arguments after construction require `Params`; names and values
require `String`. Query operations preserve the input. Wrong fields/types or
malformed pairs must not be silently skipped. Invalid URL syntax returns false
from `can_parse`, no item from `parse`, and a source-mapped error from `href`.
A supplied invalid base is validated even with absolute input.

The existing TODO's duplicate, first-match, stable-sort, optional-value,
percent-encoding and empty-value requirements remain part of the proposal.
Use UTF-16 code-unit comparison for parameter sorting, independent of the
record enumeration rule proposed in D1. URL fields use component-specific
encoding; `query` uses form encoding. Keep `href` authoritative for lossless
serialization: exposed empty `search`/`hash` strings alone do not preserve the
difference between absent and empty delimiters.

## D1 — parameter constructor and record order

**Recommendation:** accept no argument, one `String`, one mapping record with
singleton string values, or a stream of two-item string arrays. Enumerate mapping
keys in Rust string order consistently for owned and native records. Preserve
pair-stream order and duplicates. Do not change the common record model.

A mapping `{ b: "2", a: "1" }` therefore serializes as `a=1&b=2`; an explicitly
ordered pair stream for b then a serializes as `b=2&a=1`. This mapping adaptation
must be documented rather than advertised as JavaScript insertion-order parity.
A record `{ name: "a", value: "1" }` means two mapping keys, not one entry.
`Params` is already the output format; callers pass it directly to query
operations, without routing it back through the overloaded constructor.

Alternative: require insertion-preserving mapping input, which needs a separate
representation or common record-model work. That exceeds a URL-only change.

Decision needed: accept the typed, deterministic mapping adaptation, or require
a representation change before supporting the record constructor?

## D2 — immutable setters and ambiguous parts

**Recommendation:** reproduce browser setter results on a private URL value,
including ignored/inapplicable setters, and emit a nonfatal source-mapped
`cem.ql.url_setter_ignored` warning for a detected invalid/inapplicable setter.
Do not infer rejection merely from unchanged serialization: equal assignments
and normalization can be successful. Return no shared mutable URL handle.

Proposed update order: `protocol`, `host` or `hostname`, `port`, `username`,
`password`, `pathname`, `search` or `query`, then `hash`. Record field order has
no effect. Reject unknown fields, `origin`, overlapping `host` with `hostname`
or `port`, and simultaneous `search` and `query`, even when values agree.
`hostname` with `port` is allowed. `href` is an assembly seed only, not a
`with_parts` field. Omitted fields preserve the input; supplied empty values
still invoke their component operation. Empty `query` clears the query.

Alternative: strict transactional updates that return an error and no URL when
a setter is ignored/inapplicable. This intentionally differs from browser
assignment behavior. Under either policy, argument/parts conflicts are errors;
there is no partial result on a type or conflict error.

Observed with Node's URL implementation during review (illustrations, not
native conformance evidence), starting from `https://example.test:8443/a`:

| Assignment | Observed serialized result |
| --- | --- |
| `port = "70000"` | unchanged |
| `port = "123abc"` | `https://example.test:123/a` |
| `protocol = "mailto:"` | unchanged |
| `host = "other.test"` | `https://other.test:8443/a` |

Decision needed: compatible results plus warnings, or strict failure? Accept
the fixed order and overlap rejection, or choose explicit override precedence?

## D3 — assembly seed and round trips

**Recommendation for the first implementation:** require `parts.href` or an
explicit absolute base. Resolve `href` against the supplied base when present;
otherwise clone the base. Apply the D2 fields after construction, with no
ambient seed. This deliberately limits assembly from a parts-only record.
Opaque URLs can be provided through `href`; no synthetic authority is invented.
Empty parts with a base return the canonical base; without either seed, fail.

Alternative: require parts-only construction in the first version. That needs
an explicit contract for authority versus opaque paths, minimum required fields,
component escaping and delimiter presence before implementation. It must not be
implemented as ad hoc string concatenation or by selecting an arbitrary URL seed.

A full `Parsed` record contains overlapping fields plus read-only `origin` and
is therefore not an accepted update/assembly record under this recommendation.
Use `href(parsed.href)` for a faithful serialization round trip; choose fields
explicitly for updates. Passing `parsed.query` as an update is an intentional
query reserialization, not a promise to retain the original URL spelling.

Decision needed: accept seed-based assembly first, or specify complete parts-only
construction before implementing the family?

## Remaining acceptance work after the decisions

Finalize diagnostics, exact registry/type rules and examples under the selected
policies, including how the type checker reports optional result cardinality.
Retain existing error propagation rather than returning a plausible empty value
for malformed arguments. Proposed error categories are invalid URL/base,
invalid parts, argument type/cardinality, and the D2 setter outcome; names beyond
the D2 proposal remain to be assigned in the diagnostic registry.

Specify the portable origin policy before claiming whole-API parity: recommend
`"null"` for file origins and evaluating blob origins without a host object-URL
store. Treat this as an explicit pure profile and test it across runtimes.
Cover the TODO's Unicode/IDNA, component encoding, default ports, dot segments,
special/non-special schemes, opaque paths, credentials, IPv4/IPv6 and file cases.

Add an actionable fixture checklist before native tests. Start with small
registry/type/evaluation cases and selected pinned WPT vectors. Include query
sorting across supplementary/BMP characters, malformed percent/UTF-8 input,
invalid bases, empty delimiters, setter no-ops and conflicts, and immutable
round trips. Use native tests first; then verify the same semantics through the
existing CLI/SSR/WASM paths. Generate reference tables and executable CEM-QL
examples only for implemented, validated functions.

Documentation-only validation for this draft: local file links and whitespace.
The Node probes above inform the decision examples only. No runtime code,
fixture, dependency or generated artifact changes; no test/build suite required.
