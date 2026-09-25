# Pure CEM-QL URL family — accepted implementation contract

Status: **accepted target; native parameter core implemented, query API pending**
(2026-09-25). The user's
“continue recommended” accepts D1–D3: deterministic mapping order, compatible
setter results with warnings, and seed-based assembly. This document completes
those choices into the implementation contract referenced by [todo.md](todo.md).
Public schema/reference output must continue to describe only implemented APIs.

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
  `cem-ql` has no direct `url` crate dependency. The workspace lock contains `url`
  2.5.8, but that is a candidate dependency, not evidence of conformance.
  Its typed setters alone cannot establish JavaScript setter parity.

## Value contract

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

| Function | Arity | Result / argument shape |
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

The TODO's duplicate, first-match, stable-sort, optional-value,
percent-encoding and empty-value requirements apply.
Use UTF-16 code-unit comparison for parameter sorting, independent of the
record enumeration rule in D1. URL fields use component-specific
encoding; `query` uses form encoding. Keep `href` authoritative for lossless
serialization: exposed empty `search`/`hash` strings alone do not preserve the
difference between absent and empty delimiters.

## D1 — parameter constructor and record order

**Accepted:** accept no argument, one `String`, one mapping record with
singleton string values, or a stream of two-item string arrays. Enumerate mapping
keys in Rust string order consistently for owned and native records. Preserve
pair-stream order and duplicates. Do not change the common record model.

A mapping `{ b: "2", a: "1" }` therefore serializes as `a=1&b=2`; an explicitly
ordered pair stream for b then a serializes as `b=2&a=1`. This mapping adaptation
must be documented rather than advertised as JavaScript insertion-order parity.
A record `{ name: "a", value: "1" }` means two mapping keys, not one entry.
`Params` is already the output format; callers pass it directly to query
operations, without routing it back through the overloaded constructor.

An explicitly empty initializer stream is the empty pair stream and produces
no entries. A singleton two-item array is one pair; an outer array containing
pair arrays is not implicitly flattened. A mixed stream, a pair of the wrong
length, or a mapping with empty/multiple/non-string values is a type error.
Native record views use the same key ordering as owned records. No common
record representation or insertion-order guarantee changes.

## D2 — immutable setters and ambiguous parts

**Accepted:** reproduce browser setter results on a private URL value,
including ignored/inapplicable setters, and emit a nonfatal source-mapped
`cem.ql.url_setter_ignored` warning for a detected invalid/inapplicable setter.
Do not infer rejection merely from unchanged serialization: equal assignments
and normalization can be successful. Return no shared mutable URL handle.

Update order: `protocol`, `host` or `hostname`, `port`, `username`,
`password`, `pathname`, `search` or `query`, then `hash`. Record field order has
no effect. Reject unknown fields, `origin`, overlapping `host` with `hostname`
or `port`, and simultaneous `search` and `query`, even when values agree.
`hostname` with `port` is allowed. `href` is an assembly seed only, not a
`with_parts` field. Omitted fields preserve the input; supplied empty values
still invoke their component operation. Empty `query` clears the query.

Validate all argument shapes and parts conflicts before applying any setter.
There is no result on a type or conflict error. Component values other than
`query` require one string; assembly `href` accepts `Text`. After validation,
apply setters in the fixed order and continue after each warning.

Preserve partial setter effects too. Starting from
`https://example.test:8443/a`, assigning `host = "other.test:70000"` yields
`https://other.test:8443/a`: the hostname changes while the invalid port is
ignored. Emit one `url_setter_ignored` warning naming `host` and the rejected
port subcomponent; do not roll back the hostname. The diagnostic denotes an
ignored part of the request, not necessarily an unchanged URL. Valid prefix
parsing, such as port `"123abc"` producing 123, is successful and does not warn.

A candidate dependency's success/error return alone is insufficient if it
swallows an ignored subcomponent. Detect the relevant parser/setter outcome in
the native adapter; never compare only the before/after serialization.

Observed with Node's URL implementation during review (illustrations, not
native conformance evidence), starting from `https://example.test:8443/a`:

| Assignment | Observed serialized result |
| --- | --- |
| `port = "70000"` | unchanged |
| `port = "123abc"` | `https://example.test:123/a` |
| `protocol = "mailto:"` | unchanged |
| `host = "other.test"` | `https://other.test:8443/a` |


## D3 — assembly seed and round trips

**Accepted for the first implementation:** require `parts.href` or an
explicit absolute base. If `href` is present, parse it with the supplied base, or as an absolute
URL when no base is supplied. If `href` is absent, clone the explicit base.
Validate a supplied base even when `href` is absolute. Apply the D2 fields after construction, with no
ambient seed. This deliberately limits assembly from a parts-only record.
Opaque URLs can be provided through `href`; no synthetic authority is invented.
Empty parts with a base return the canonical base; without either seed, fail.

Parts-only construction without either seed is outside this first version.
It must not be emulated with an arbitrary seed or ad hoc concatenation.

A full `Parsed` record contains overlapping fields plus read-only `origin` and
is therefore not an accepted update/assembly record under this contract.
Use `href(parsed.href)` for a faithful serialization round trip; choose fields
explicitly for updates. Passing `parsed.query` as an update is an intentional
query reserialization, not a promise to retain the original URL spelling.

## Query operations

Follow the [URLSearchParams algorithms](https://url.spec.whatwg.org/#interface-urlsearchparams)
with immutable results. `entries`, `keys`, and `values` preserve list order;
`size` counts pairs. `get` selects the first match; `get_all` selects all.
`has` and `delete` match names, or name/value pairs when a value is supplied.
`append` adds at the end. `set` replaces the first matching value and removes
later matches, or appends when absent. Sorting is stable by UTF-16 name units.
String construction strips one leading `?`, splits form data, and decodes
`+` as space. Missing `=` means an empty value. Serialize spaces as `+` and
encode literal percent signs. A complete URL string is still form-data input.

No callback function or mutable URLSearchParams object is introduced. Inputs
already containing `Entry` records go directly to query functions; `params`
does not guess that a mapping record was intended to represent one entry.

## Native types and diagnostics

Register `url` as a default module alias, with identical behavior through an
explicit import alias. Do not introduce bare helpers that could shadow other
modules. Type validation must dispatch by resolved module/function identity,
not by spelling of the author's prefix or a similarly named user function.

Use existing atomic/record/stream types. `Parsed?` and `string?` are represented
conservatively as `Stream<Record<...>>` and `Stream<String>` in static inference;
the evaluator enforces zero-or-one results. The current lattice has no separate
optional cardinality type. Do not widen that common model for this family.
Reject provably incompatible static arguments; defer unknown types and stream
cardinality to runtime validation. This permits feeding a possibly-singleton
stream into a scalar argument without pretending its cardinality is proven.
Validate singleton cardinality, exact entry fields, pair lengths and parts
conflicts in the evaluator before producing output. Return concrete scalar
result types where the function guarantees one item, and concrete entry/string
stream types for query operations; do not register every result as `Any`.

| Code | Severity | Outcome |
| --- | --- | --- |
| `cem.ql.type_error` | error | Wrong atomic/record/array shape, cardinality, pair length or entry fields; no result |
| `cem.ql.url_invalid` | error | URL parse failure in `href`, `assemble` or `with_parts`; no result |
| `cem.ql.url_base_invalid` | error | Supplied base fails absolute parsing for `href` or `assemble`; no result |
| `cem.ql.url_parts_invalid` | error | Unknown/read-only/conflicting parts, or missing assembly seed; no result |
| `cem.ql.url_setter_ignored` | warning | Inapplicable/invalid setter or ignored subcomponent; return the resulting URL and continue |

`can_parse` and `parse` suppress only new URL/base syntax failures, returning
false or no item respectively. They do not suppress argument errors or an
upstream error. Preserve existing argument diagnostics and errors in evaluation
order. Static argument rejection prevents evaluation; runtime checks cover
unknown values, not a second emission of the same static failure.

For newly produced failures, validate argument shapes in argument order,
then parts keys/conflicts in deterministic key order, then the base, then the
URL seed/input. Report the first fatal validation failure with no output.
Emit at most one setter warning per supplied field, in setter order, naming
all rejected subcomponents of that field. Preserve the query source range and
source-map context; diagnostic messages name the argument/field and reason,
without copying credentials or the entire URL.

Nonfatal parser validation conditions that still yield a URL do not become
`url_invalid`. URL syntax parsing is not a resource-policy check. Query decoding
uses the referenced form algorithm's malformed-input handling, not strict URL
validation or a generic decoder that rejects recoverable data.

## Portable origin and encoding profile

The pure profile uses the standard's origin algorithm with no blob-URL store.
File origins serialize as `"null"`. A blob path can supply an HTTP/HTTPS origin;
file and other opaque origins serialize as `"null"`. These are origin strings,
not reusable origin identities. No host environment lookup is permitted.
See [the origin algorithm](https://url.spec.whatwg.org/#concept-url-origin).

Native strings contain Unicode scalar values. Do not add a JavaScript UTF-16
object representation; retain the explicit UTF-16 comparison rule for parameter
sorting. Use the pinned standard's IDNA, IPv4/IPv6, special/non-special scheme,
opaque-path, credential, default-port, dot-segment and component-encoding
algorithms. File parsing must be platform independent and must not consult the
host filesystem. Candidate Rust libraries must be checked against these cases;
sharing a dependency across platforms does not by itself prove parity.

## Implementation sequence and focused acceptance

1. Add native tests first for the immutable parameter core and its exact typed
   input boundary. Implement that core without host services or new value types.
2. Add parse/serialize/base and origin fixtures against a pinned native dependency
   and selected WPT vectors. Document any discrepancy before choosing a fix;
   do not silently substitute the embedding platform's URL implementation.
3. Add assembly/update fixtures, including partial effects and warning detail,
   using the native setter adapter. Cover every rejected/allowed parts pairing.
4. Wire registry, aliases, inferred types, diagnostics and evaluation. Test
   imported aliases and user functions with colliding local names, diagnostic
   propagation, immutable input values and source locations.
5. Generate reference tables and the executable CEM-QL query/result matrix;
   validate that implemented functions match this contract. Verify the same
   cases through CLI, SSR and WASM/browser entry points.

Each stage needs an actionable fixture checklist before tests are added. Native
fixture inputs remain typed CEM values, not a JSON-based internal handoff. WPT
JSON is an explicitly named test-data input boundary with pinned provenance and
license, not a runtime representation. Keep copied vectors separate from authored
CEM fixtures. Run focused native tests first; broaden checks only when common
modules change or failures require it.

Required regression cases include:

- empty constructor and explicit empty pair stream; one pair versus an outer
  array; mapping order versus pair order; mixed/malformed inputs;
- duplicate names, missing versus empty values, append/delete/set position,
  optional-value matching, UTF-16/BMP/supplementary stable sorting;
- form spaces/pluses, literal percent signs, malformed percent/UTF-8 sequences,
  and a complete URL supplied as form input;
- explicit invalid bases even with absolute input, opaque relative bases,
  `href` idempotence, empty `?`/`#`, and query reserialization;
- ignored, normalized, equal, prefix-parsed and partially applied setters;
  fixed update order, conflicting keys and diagnostics before any result;
- IDNA, host IP forms, credentials, default ports, dot segments, opaque origins,
  file and blob behavior, and no dependency on ambient location or module maps.

## Validation of the contract-only milestone

The contract-only milestone validated local links, whitespace and every
function/arity against the checklist while keeping implementation items open.
Illustrative Node probes confirmed the partial-host and form-decoding examples;
they were not native CEM-QL conformance tests. That milestone changed no runtime,
dependency, fixture or generated artifact. Native implementation evidence follows.


## URL-PARAMS implementation evidence

Completed 2026-09-25: the native
[`UrlParams` core](../packages/cem_ql/src/stdlib/url_params.rs) and its
[typed-input fixtures](../packages/cem_ql/tests/url_params.rs) implement stage 1.
The initial focused test build fails on the missing core import; after
implementation all nine focused tests pass. Tests cover constructors, native
views, malformed cardinality/shape, duplicate handling, form decoding and
encoding, immutable edits, query versus mapping sort order, and typed entry
round trips. Node-like values are not atomized. Errors contain shape information
without copying argument payloads; evaluator source/diagnostic attachment stays
in URL-INTEGRATION.

The core uses `form_urlencoded` 1.2.2, already present in the workspace lock,
through a direct cem-ql dependency. No common record/value representation,
query registry or evaluator behavior changes. Whole-URL parsing, setter behavior,
WPT provenance and cross-runtime entry-point parity remain unimplemented stages;
the passing parameter tests do not claim those capabilities.

Reproduce focused coverage with:

```sh
cargo test -p cem-ql --test url_params --target-dir dist/target/cem_ql
```

The package-native regression target `yarn nx run cem_ql:test` reaches
**116 passing unit tests (4 ignored)** and **2 passing attribute tests**, then
stops at `select_boolean_attributes_use_presence_with_exact_dom_strings`.
Its `group.label` and `group.options` queries report unknown pipeline steps.
A clean detached worktree at baseline commit `093fcb40` reproduces the same
failure with the single-test command below. This is a pre-existing package-gate
failure, not a passing full suite. No select/runtime correction is included.

```sh
cargo test -p cem-ql --test attribute_strings select_boolean_attributes_use_presence_with_exact_dom_strings --target-dir /home/suns/cem/dist/target/cem_ql
```

Evidence logs: `/tmp/cem-url-params-{red,green,final,native,baseline}.log`.
The nine focused parameter tests, formatting and whitespace checks pass.
README/contract local links resolve. No browser or workspace-wide suite ran:
this core has no evaluator or browser entry-point wiring yet. Investigate the
baseline select fixture separately before treating the native package gate as
green; URL-PARSE remains the next URL-family implementation stage.
