# Pure CEM-QL URL family — accepted implementation contract

Status: **parse/assembly query API implemented; parameter query API pending**
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
- [The registry](../packages/cem_ql/src/stdlib.rs) exposes five URL parse/assembly
  functions. Native URL cores use the scoped Rust `url` 2.5.8 dependency.
  This is not evidence of full conformance; three file/IDNA parse cases remain.
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


### Select fixture attribution and correction

Resolved 2026-09-25 with fixture-only changes. The choice capability's
[`renderSlices`](../packages/cem-elements/src/lib/choice-select-capability.ts)
always supplies `groups` as a collection. The native boolean-attribute fixture
supplied mode and behavior flags but omitted that collection's host binding.
`seed_declaration_defaults` in the native renderer therefore seeded the empty
`groups` declaration as one scalar null item and projected it into the slice.
The loop then attempted record-field reads on that item. No change to loop,
field-read, declaration-default or component semantics is needed.

A minimal named-template/group loop reproduces the missing-binding failure.
Supplying matching host and data-document collections renders empty groups and
a populated group with a nested option without diagnostics. The original
select fixture now supplies explicit empty `groups` in both locations. Its
existing dropdown/listbox and missing/empty/false/present-string assertions
remain intact. Both tests live in
[`attribute_strings.rs`](../packages/cem_ql/tests/attribute_strings.rs).

All **4 focused attribute tests pass** with:

```sh
cargo test -p cem-ql --test attribute_strings --target-dir dist/target/cem_ql
```

Evidence: `/tmp/cem-select-attribution.log` (reproduction) and
`/tmp/cem-select-final.log` (passing correction). Only the fixture and these
notes/checklist changed. Whitespace and local-link checks pass. No common
module changed, so no full native, browser or workspace-wide suite was run.
This closes the attributed failure, not a claim that the previously interrupted
package suite has completed. Next is URL-PARSE with native tests first.


## URL-PARSE candidate evidence

On 2026-09-25, the native `url` 2.5.8 candidate was tested against
[51 selected WPT vectors](../packages/cem_ql/fixtures/url/README.md), pinned to
revision `c48d58747e1f211527fb695fd60548a997fae617` with source SHA-256 and
unchanged BSD license. All selected objects were compared with the pinned source.
The dependency is dev-only; no production module, registry or evaluator changed.

The initial conformance test fails: eight cases have 11 differences. These cover
file drive normalization and IPv6-host preservation, nested/non-HTTP blob origins,
and empty punycode labels. Forty-three cases match all supplied expectations.
The final three focused tests characterize the exact candidate gaps and cover
additional explicit-base, delimiter, query and pure-origin behavior. Their
passing result **does not mean the candidate conforms**. Upstream expectations
are unchanged; a separate difference file retains the adoption blockers.

Evidence logs: `/tmp/cem-url-parse-candidate.log` (initial failure) and
`/tmp/cem-url-parse-characterization.log` (three passing characterization tests).
Only tests, pinned data/license, a dev dependency and documentation changed.
Formatting, whitespace and pinned-source integrity checks pass. No full package,
workspace or browser tests were invoked.

Initial investigation decision (superseded by the Rust decision below): compare
maintained upstream fixes and replacement parsers before adoption. The
accepted contract is unchanged. Four origin cases need the already specified
pure-profile handling, but drive/host and IDNA differences require parser-level
evidence; do not assume post-serialization rewriting can recover lost host
information or rejected input. URL-PARSE remains open until that strategy and
its conformance evidence are complete. No compatibility fix is included here.

### Maintained alternative investigation

The [isolated reproducible probe](../packages/cem_ql/fixtures/url/parser-probe/README.md)
compares current upstream `url` at a pinned revision (44/51 selected cases match)
with Ada 4.0.0 (51/51 native matches). SDK 34 permits the combined browser-WASM
build, but residual WASI imports block standalone execution in both debug and
release. No production dependency changed. The subsequent user decision below
closes the toolchain choice. Full conformance, setters, and CEM integration
remain open.

### Rust decision and deferred compatibility work — 2026-09-25

Keep Rust; do not adopt Ada or introduce C++/WASI SDK build requirements. The
[eight known gaps and importance assessment](wishlist.md#rust-url-compatibility-gaps)
remain explicit compatibility debt. The selected `url` version is still dev-only;
this documentation change does not implement a parser or authorize a fork.
Proceed with Rust implementation, prioritizing the bounded pure-origin adapter.
Deferred parser differences retain their original WPT expectations and must be
reported as limitations rather than presented as full WHATWG conformance.
The accepted semantics remain the target; moving debt to the wishlist does not
make the missing behavior implemented. The Ada probe is historical evidence,
not an active toolchain recommendation or required verification task.

### Native origin adapter completed — 2026-09-25

[`serialize_origin`](../packages/cem_ql/src/stdlib/url_origin.rs) implements the
pure origin profile on a parsed Rust URL. Direct HTTP/HTTPS/FTP/WS/WSS URLs
retain their normalized tuple origins; file and other opaque origins serialize
as `null`. Blob origins inspect exactly one parsed inner URL and accept only
HTTP/HTTPS. Nested blobs, FTP/WS/WSS inner URLs, malformed or relative inner
URLs and opaque inner schemes return `null`, without host lookup or mutation.

The pinned `url` 2.5.8 dependency moves from dev-only to runtime for this core.
No parser fork, C++ dependency, query registration or evaluator change is added.
Three new [native tests](../packages/cem_ql/tests/url_origin.rs) cover the profile,
input immutability, and all 11 blob-origin expectations in the selected WPT set.
Two regressions fail with direct dependency-origin delegation and pass with the
adapter. All 15 focused URL tests pass, including the unchanged raw candidate
characterization. The four origin gaps are fixed through the adapter; the four
file/IDNA parser gaps remain documented in the wishlist. Next is the explicit-base
parse/serialize core; query integration and full conformance remain unproven.

Validation: `cargo test -p cem-ql --test url_origin --test url_params --test
url_parse_candidate --target-dir dist/target/cem_ql` passes (15 tests), and
`yarn nx run cem_ql:build:wasm` succeeds. The WASM build verifies compilation,
not execution parity of the unregistered API. Formatting and whitespace checks
pass; no full package test suite or workspace-wide task ran.

### Native parse/serialize core completed — 2026-09-25

The [typed Rust core](../packages/cem_ql/src/stdlib/url.rs) implements `can_parse`,
`href` and `parse` over CEM-QL item streams. It accepts singleton string/anyURI
atoms, including native atomic views, while rejecting non-atomic views even if
they expose an atom. Omitted bases are distinct from empty argument streams.
All argument shapes are checked in argument order before validating the base,
then input syntax. An invalid supplied base fails even for absolute input.

`href` returns an anyURI atom or a classified error; `parse` returns an optional
control record, and `can_parse` a boolean. The latter two suppress syntax errors
only. Error codes/messages omit input payloads; future evaluator integration
must attach source ranges and preserve existing argument diagnostics. Parsed
records use the pure-origin adapter, retain authoritative href delimiters, and
contain ordered query entry streams built through the existing parameter core.
No runtime JSON conversion, ambient base, host lookup or resource load occurs.

Seven new [tests](../packages/cem_ql/tests/url_parse.rs) cover strict typed
boundaries, explicit/opaque/invalid bases, canonical fields, href idempotence,
query decoding/duplicates, recoverable encoding, origin handling and error codes.
All 51 selected WPT cases now pass through the core: 47 match, and four retain
exactly the seven deferred file/IDNA differences. That test characterizes the
known gaps; it does not claim full conformance or alter upstream expectations.

All 22 focused URL tests pass. The URL-PARSE native stage is complete with the
accepted deferred compatibility debt. Next is URL-PARTS: seeded assembly and
immutable setters with ignored/partial-effect evidence. Query registry, static
types, source-mapped evaluator diagnostics and execution parity remain pending.

Validation: the focused `url_parse`, `url_origin`, `url_params` and
`url_parse_candidate` test binaries pass, as does `yarn nx run cem_ql:build:wasm`.
The build verifies WASM compilation, not query execution parity. No full test
suite or workspace-wide task ran; shared evaluator/type behavior is unchanged.

### URL-PARTS setter evidence and accepted repair direction

The [pinned setter probe](../packages/cem_ql/fixtures/url/SETTERS.md) compares 277
WPT component cases through native seed parsing and `url` 2.5.8 setters. It
finds 22 differing cases (36 fields), including port loss, hostless/file path
serialization and opaque trailing-space handling. Additional authored probes
show that raw return values cannot supply D2 warning classification, including
partial host success and equal file-protocol rejection. Four passing tests
characterize those findings; they do not establish setter conformance.

The user chose to fix the Rust setter path before exposing updates; bounded
adapter progress and the next parser-level decision are recorded below. The previous parse-gap
exception is not automatically extended. The complete list and importance
assessment are in [the wishlist](wishlist.md#rust-url-setter-compatibility-gaps).
No production/common module changed; no global task or full test suite ran.

### Bounded port and hostless-path fixes — 2026-09-25

The [native setter primitives](../packages/cem_ql/src/stdlib/url_setters.rs)
fix whitespace-only port preservation and hostless hierarchical pathname
serialization. Applicability/outcome reporting distinguishes an ignored port
or opaque pathname from equal/normalized successful assignments. A truly empty
port still clears it. Hostless paths retain the required `/.` guard and shed a
stale guard when no longer needed, preserving query/fragment and URL identity
through reparsing. These mutators act on a future parts adapter's private clone;
no query API or evaluator warning is registered yet.

All 31 focused URL tests pass. The adapted WPT matrix matches 260/277, leaving
17 cases with exactly 30 differences; the raw candidate baseline stays unchanged.
Five cases (port[26], pathname[24–27]) are closed at the native adapter layer.

File pathname[21–23] cannot use the same repair strategy: the pinned parser
collapses leading empty file segments even when parsing the correct expected
serialization. Stop before adopting a dependency patch. Recommend owning a
scoped Rust parser fix; alternatively explicitly defer file setter support.
The remaining cases and their importance stay in the wishlist. Assembly,
fixed-order multi-field updates and source-mapped warning integration remain open.

The package Nx WASM build passes; no full test suite or global task ran.

### Scoped file-path parser patch completed — 2026-09-25

The user accepted maintaining a scoped Rust patch. The [vendored crate](../vendor/url/CEM-PATCH.md)
preserves leading empty file segments and corrects path-start handling for
empty/localhost authorities. Simply removing trimming doubled root separators;
upstream `issue_197` and the existing href round-trip assertions caught that
regression before the complete patch passed. Runtime changes are confined to
three sections of the dependency's `parser.rs`, with a reviewable source diff.

CEM-QL uses private `cem-url` 2.5.8 through a path dependency, retaining the
`url` Rust library API. The distinct package name avoids an Nx graph collision
with registry `url`; explicit CEM-QL cache inputs include vendored source.
CEM-ML still resolves registry `url`; there is no workspace-wide Cargo patch.
Raw candidate tests explicitly use `url_unpatched` and retain their original
expectations. The new fork is excluded from the main Cargo workspace.

File setter cases pathname[21–23] and legacy drive parse case 138 now pass.
The native core matches 48/51 selected parse cases and 263/277 setters. All
31 focused CEM tests, 66 upstream unit tests and the historical dependency WPT
harness pass. The latter removed only 23 now-passing expected-failure entries;
no upstream expected outputs changed. This remains partial conformance.

Next is opaque trailing-space preservation and partial-host/protocol outcome
classification. Assembly/update registration and source-mapped warnings remain
open. Publishing CEM-QL requires a deliberate distribution route for private
`cem-url`; the local path dependency is sufficient for workspace builds only.

The CEM-QL Nx WASM build passes with the private parser dependency. This is
compilation verification; registered query execution parity remains pending.
No full workspace/package test suite or global task ran.

### Opaque spaces and setter outcomes completed — 2026-09-25

The private Rust parser now encodes the last opaque-path space before a query
or fragment delimiter. Clearing either component preserves its path content.
This fixes search[10–13] and hash[16–19] without changing CEM's pinned WPT data.
Eight stale bundled dependency cases were refreshed from that pinned source;
two historical unit assertions now expect preservation. The complete runtime
diff and fixture provenance are in [CEM-PATCH.md](../vendor/url/CEM-PATCH.md).

The native host adapter strips only the standard ASCII tabs/newlines, respects
IPv6 brackets and authority terminators, and detects rejected nonempty port
subcomponents on a private probe. Accepted hostname changes survive a
`Ignored("host.port")` outcome; valid prefixes, omitted/empty ports and equal
assignments do not produce false warnings. Scheme guards now permit eligible
file conversions and equal file assignments while retaining invalid transition
rejection. These are native outcomes, not yet source-mapped query warnings.

All 34 focused CEM URL tests, 66 dependency unit tests and the refreshed
historical WPT harness pass. The selected matrix matches 271/277 setter cases;
six cases retain exactly 12 differences. Parse coverage stays 48/51. Raw
registry characterization remains unchanged. Next: remaining hostname/path
serialization fixes, with empty-punycode behavior still in the wishlist.

The package Nx WASM build passes; no full/global test suite ran. Shared
evaluator/type behavior and CEM-ML registry dependency remain unchanged.


## Native assembly and immutable updates completed

The typed core in `stdlib/url_parts.rs` implements `assemble` and
`with_parts` using the accepted D2/D3 validation and setter order. It accepts
ordinary or native record views, rejects duplicate native field names, requires
explicit seeds, distinguishes omitted fields from empty values, and validates
query entry streams through the existing parameter core. Caller items are
never mutated; results contain an anyURI and ordered warning metadata.

Warnings name the supplied field and any rejected subcomponent, including
partial host.port rejection without rollback. They carry no credentials or
input URL values. The evaluator must still attach source ranges/maps and
preserve existing diagnostics; these APIs are not registered query functions.

Nine new tests cover the 45 component field pairings, seed/base precedence,
typed values, empty assignments, normalized success, native enumeration order,
query duplicates, ignored opaque setters and partial effects. All 46 focused
URL tests and the CEM-QL Nx WASM build pass. The pinned setter matrix remains
275/277; the two deferred empty-punycode cases and remaining parse gaps stay
in the wishlist. No common evaluator/type source or parser dependency changed;
no full/global suite ran. Next: URL registry/type/evaluation integration and
source-mapped diagnostic fixtures.

## Parse/assembly query integration

The registry now exposes `can_parse`, `parse`, `href`, `assemble` and
`with_parts` through default `url:` and imported `cem:stdlib/url` aliases.
Static checking rejects incompatible argument types, defers stream cardinality
to the typed core and returns boolean, anyURI or a typed optional parsed-record
stream. URL-specific checks follow registered signatures; user declarations
with the same names do not inherit them or dispatch to the builtin.

Evaluation uses the complete call source range and embedding source-map stack.
Argument failures stop subsequent argument evaluation and cannot be suppressed
by parse/can_parse. Reports are retained once on the evaluator context, in
argument/setter order. Invalid URL/base/parts calls return no items, while
ignored setters retain the resulting anyURI and warning details.

Seven native query tests cover aliases, types, cardinality, user-function
isolation, full embedding maps, partial host effects and upstream diagnostics.
The Node WASM fixture `tests/url-query-wasm.mjs` exercises the same registered
Rust path through the explicit JSON query API boundary. Browser/CLI/SSR parity
and generated schema reference output remain checklist work.
Next: register the implemented parameter core as `url:params*`, including
precise result types, validation, diagnostics and query-level fixtures.

Validation: `yarn nx run cem_ql:test` passes 781 tests (nine ignored),
`yarn nx run cem_ql:build:wasm` passes, and
`node packages/cem_ql/tests/url-query-wasm.mjs` passes.
The package suite was required because this step changes shared registry,
type-checking, lowering and evaluator dispatch. No workspace-wide task ran.
The registry setup/function-count assertions now expect 19/81 instead of 18/76.
