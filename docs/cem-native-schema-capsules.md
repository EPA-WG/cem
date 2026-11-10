# Portable native schema contracts for component applications

Status: the native-only structural `CEMSC1` exporter/verifier and explicit
fresh-host node receiver linker are implemented. They restore a bounded shared
model and bind its use to a concrete native host and invocation. A native-only
`CEMQV1` capsule also verifies a closed subset of compiled validator queries and
registers them against separately supplied checked original declarations.
Native `CEMQD1` now carries those declarations and binds query use to a concrete
host scope, context epoch and invocation. Native `CEMNR1` requirements now link
exact registered validators, tokenizers, scalar lexical preparers, converters,
scalar equality and list serializers to those host lifetimes. Native constant
interpreters, facet families, shared list-item orchestration and canonical native
source-result profiles now have explicit checked bindings too. The native-only
`CEMAPPD2` dependency envelope packages and inertly verifies compiled
declarations and typed capsules with a bounded declared dependency graph.
Native named-node assembly now stages exact receiver occurrences and query/native
bindings against one fresh host, with a checked in-process replacement slot.
Native `CEMDT1` now carries original local datatype sources and exact literal
base/list-item dependency fields. A checked fresh-host adapter compiles these
sources through the shared compiler with explicit implementation registrations.
This is not executable/behavior closure proof or named receiver admission.
Datatype roots now issue guarded process-local preparation evidence and consume
it with fresh validation and one cumulative work allowance. Retired tokenizer
owners also invalidate retained evidence in the shared attribute path.
Native `CEMDP1` separately pins intended datatype dependency fields and verified
DT1/QD1/NR1 inputs, including passive original references. These are source-backed
build assertions, not checked host selections, crossing grants or executable closure.
A native fresh-host adapter now checks those selections against actual source
scopes, original aliases, explicit exports and directed grants. Its local inspection
result remains separate from datatype compilation and receiver publication.
A native primitive compiler now renews selection within the actual checked
invocation and keeps cross-source scalar/list descriptors private for root-only
typed validation. Imported query/native rules now bind explicit owners and use
original source contexts; preparation/tokenizer hooks remain separate gates.
Native `CEMDM1` now carries flat dependency-bearing datatype modules with stable
module IDs, original sources, shared datatype/leaf checks and bounded acyclic
module/declaration orders. Its immutable metadata now has explicit fresh-host
selection/compilation entry points sharing the existing checked resolver and
compiler. Existing CEMDP1 wire semantics remain unchanged.
Native `CEMOP1` separately groups exact CEMNR1 operation/constant dependencies
by original datatype, with checked roles, representations and whole-source
agreement. Its verified records remain inert; module-compiler integration is
a separate gate.
Native `CEMOS1` separately retains dependency-bearing operation sources and
constant-reference slots. It preserves original contexts through the shared
dependency snapshot, without widening CEMNR1/CEMOP1 or resolving selectors.
Explicit native entry points now bind these requirements to actual-host operation
and tokenizer owners using the existing weak native guards and result adapters.
Native `CEMOP2` now groups exact CEMOS1 pins through the shared profile validator,
with independent whole-source agreement before occurrence mapping. These are
still inert source requirements, not installed module/receiver capabilities.
Native `CEMDM2` now assembles original datatype modules, CEMOP2 profiles and
intended constant-reference targets, with explicit module imports and bounded
graph checks. Its explicit actual-host inspection now renews datatype/constant
selections against original scopes and per-hop grants. Native CEMDM2 compilation
now renews those selections inside the shared compiler and installs exact,
explicitly owned CEMOS1 profiles. Guarded synchronous root preparation, conversion
and list serialization now share its actual source contexts and owner guards.
The [CEMAPPD3 native installation](#native-assembly-application-installation)
now stages exact graph/root compilation owners and atomically replaces guarded
native root invocations. Mixed receiver/runtime publication remains a separate
gate; CEMDM1 is unchanged.
The [mixed receiver declaration contract](#mixed-receiver-declaration-contracts)
now separately pins node capsules, closed scalar facets and assembly-root demands
in CEMMRC1. Its verifier remains inert. The
[original-receiver binder](#native-original-mixed-receiver-binding) now provides
explicit guarded field consumption. The
[complete application publication](#versioned-complete-application-publication)
now composes these declarations with exact registry demand and live native owner
guards. WASM transport/generation enrollment and
compiled-component receiver admission remain **unimplemented**.
The deployed `CEMAPPD1` format remains scalar-only. Its
existing wire version must not acquire executable semantics implicitly.

This extends the [component profile](cem-components-wasm-profile.md) and
[application declaration contract](cem-components-shopping-cart-profile.md).
Implementation and fixture gates are in
[the profile checklist](todo.md#component-application-wasm-profile-separate-target).

The separately versioned [CEMAPPD3 envelope](#versioned-assembly-aware-application-envelope)
now adds exact module/profile/operation-source inventories and explicit application
roots, with whole-source agreement and bounded native verification. Native root
installation, scoped mixed field binding and native application publication are
implemented; worker/fallback transport and admission remain separate gates.

## Why a separate contract is required

`CompiledDatatypeContract` is deliberately supplied by an explicit native host
compiler, never deserialized. `ExecutableDatatype` retains original declaration
owners, dependency bindings, registered validators/tokenizers, converters,
preparation, serializers, equality and enumeration implementations. A
`SchemaDocumentModel` also contains live executable attribute contracts and
function bindings. Neither is a wire record.

Serializing `AttributeModel` does not preserve those capabilities. In particular,
clearing `native_type_pending`, naming a type `node`, or supplying a content type
cannot turn a facet record into a ready executable/node contract. A hash proves
integrity relative to a trusted pin; it does not establish host authority or a
registration. Existing callback traits, preparation handles, typed-admission
tokens and context leases must retain their in-process-only contracts.

## Proposed wire boundary

The native structural capsule uses `CEMSC1`; the separate native dependency
envelope now uses `CEMAPPD2` / `cem-application-dependencies/2`. It is an inert
package verifier, not an available component loader or WASM package export.
Existing component runtimes refuse it. Do not reinterpret a v1 scalar model,
attach ignored executable fields to it, or silently route it to a general schema
parser.

The binary is a bounded, explicitly versioned native representation. JSON may
carry a named control sidecar of hashes, identifiers and requirements; schema
trees, typed values, queries and validator programs never pass through JSON or
JavaScript records. The build and reload paths share native typed structures and
verification routines. Unknown fields, unsupported operations and noncanonical
or trailing data refuse before publication.

The capsule must carry:

| Record | Required identity and meaning |
| --- | --- |
| Compatibility | Capsule format, native IR/compiler ABI and exact selected runtime/implementation registry identity; package version alone is insufficient. |
| Source owners | Original logical schema URI, physical source/base URI, source hash and retained native source/provenance records. Include lexical namespace bindings and original declaration/field occurrences needed by shared validation. |
| Contract entries | Expanded type name, declaration and scope occurrence, representation, cardinality, effective restrictions and exact base/item/validator dependencies. Names and in-process pointer IDs cannot be the lookup authority. |
| Native programs | Reviewed compiled structural/query instructions, result/input signatures, original diagnostic attribution and every static native-function requirement. No source string to compile as a fallback. |
| Implementation requirements | Exact kind, implementation ABI/module hash/export and source occurrence for each validator, tokenizer, preparation, conversion, equality, constant interpreter, facet profile and serializer. An identifier alone grants nothing. |
| Dependency closure | Pinned source, contract and implementation dependencies, ordered where semantics require it. Every reachable requirement must resolve exactly; duplicate/conflicting identities or unused executable records refuse. |

Export must preserve the original source arena's required projection and mappings,
not just copy display spans into an unrelated reconstructed declaration. Native
reload creates new owners and establishes an explicit occurrence mapping to
them; old `SchemaDeclarationNode::identity()` strings and pointer equality cannot
survive as authority. Repeated source occurrences remain distinguishable.

## Build, verify, link, publish

1. **Build:** start with a complete native compilation and its original retained
   schema owners. Export only a reviewed portable subset. Inspect every selected
   dependency and effective inherited restriction. Any pending selection,
   unsupported callback or required context prevents that contract's export;
   dropping the unsupported part is forbidden.
2. **Verify:** check the independently trusted application pin, compatibility,
   whole capsule and per-record hashes, sizes, native graph structure, source
   occurrence mapping, instruction validity and exact dependency closure. This
   produces inert verified records, not an `ExecutableDatatype`, permission or
   published owner. Decoding performs no fetch, compilation or callback.
3. **Link:** an explicit compiler/linker owned by the fresh native runtime binds
   those records to reviewed shared implementations and creates new executable
   snapshots. Validate original declaration/scope, signatures, current context,
   directed grants and candidate requirements through the existing native
   binding machinery. This is fresh native construction, not deserialization of
   a callback or live token. No matching registration means unavailable, never
   successful string validation or a name-based builtin fallback.
4. **Publish:** revalidate generation/admission, ownership inventory and context
   epochs, then atomically publish the declaration and its complete contract
   owners. Missing requirements, cancellation or changed policy release staged
   resources and preserve the last committed UI. No partial type registry or
   extra declaration owner may remain visible after refusal.

A shared declarative demand receipt may request a pinned optional module before
linking. It cannot authorize its activation or publish an incomplete declaration.
The loader must distinguish unavailable, loading, ready and failed/offline
states. Do not add application/component JavaScript or change existing full
runtime synchronous APIs to conceal loading.

## First executable subset: structural nodes

Implement the native structural node subset first. It is typed-only: inputs are
original retained native nodes/authorized forests with expanded names and source
identity, not serialized XML text or a JSON tree. A compiled structural plan must
use the shared retained validation/content-model machinery and diagnostic
attribution. Do not create a parallel component-profile schema interpreter.

The first subset supports only the structural operations the exporter and shared
validator can both prove portable: expanded-name selection, required/allowed
attributes using supported scalar contracts, and bounded child/cardinality
rules. Explicitly enumerate accepted operations in the native format. Dynamic
schema expressions, custom parser contracts, external context reads and
executable behaviors stay unsupported until their own adapters pass. A source
schema that requires any of them cannot be exported as a weaker structure-only
contract.

The node adapter must retain candidate/source owners, original reference
occurrences and directed crossing policies. It must distinguish incomplete from
completed rejection and completed acceptance. Receiver-local restrictions
intersect the named base and never widen it. Defaults and lexical preparations
must not manufacture a node-valued input. Separate provenance for the contract
and candidate must survive both diagnostics and exported results.

After structural parity, add pure compiled query validators with verified
instruction/signature and native function closure. Native callback/tokenizer
support requires exact linked implementations and fresh explicit registrations.
Never serialize an `Arc<dyn ...>`, callback address, registration index, grant,
`NativeAttributePreparation` or `NativeAttributeTypedAdmission`. A generic list
contract cannot infer the whitespace tokenizer; checked replacement must preserve
all inherited tokenizer admission checks and original token spans. Preparation,
conversion, validation and serialization remain distinct capabilities.

## Bounds and lifecycle

Keep the current 1 MiB application binary limit, 1 MiB control sidecar limit,
64 named types, 64 KiB accounted per contract, 1 MiB aggregate contracts, 16
scalar restrictions and 64-level decoding limit. The future implementation must
account shared source/program records once in the aggregate and charge every
reachable dependency to the consuming contract; sharing cannot evade per-type
limits. Existing template inspection remains capped at 100,000 nodes.

For the first structural subset, cap native source/program/dependency records at
16,384 each, graph depth at 64 and expanded verification/validation visits at
100,000 per operation. These are design ceilings to implement and test, not
claims about a current capsule decoder. Start with an acyclic dependency closure;
recursive schema contracts need a later explicit recursion/work policy. Validate
indices, lengths, arithmetic and allocation bounds before following references
or allocating claimed collections. Poll the shared operation control during
long native loops. Nested query/function work consumes a shared invocation
budget and cancellation scope; it cannot reset the counters per dependency.

Immutable verified programs may be cached with leases under the complete app,
capsule, compiler/runtime and implementation identities. Context-dependent linked
executables and prepared values are owned by a live generation/context epoch.
They cannot be reused merely because source text is unchanged. Disposal retires
their preparations and releases dependencies; stale results never publish.

Recovery checkpoints retain portable deployment identities and already-admitted
input state under the existing privacy policy, not live callback objects or
cached verdicts. Fresh generations reacquire verified deployment bytes and link
new owners against their own registrations/context. An unavailable offline module
blocks that capability; no source fetch, full-runtime substitution or stale
owner reuse is allowed. Entered passwords/OTP still block recovery. The 4 MiB
checkpoint budget and five-second handoff deadlines remain unchanged.

## Required proof before enabling admission

| Stage | Acceptance evidence |
| --- | --- |
| Native structural export/reload | Independent full-native and reloaded validation agree for accepted/rejected/incomplete inputs, inherited/local restrictions, namespaces, reference occurrences and original candidate/contract source diagnostics. Export refuses unsupported behaviors and incomplete compilations. |
| Verification | Correctly repinned unsupported programs still refuse. Cover mixed ABI/registry identities, missing/conflicting/unused dependencies, dangling references/cycles, wrong source occurrence, malformed instructions and byte/record/depth/work limits. |
| Fresh linking | Missing/wrong registrations, tokenizer replacement, changed context/grants, cancellation and stale generations refuse without publishing an owner. Test callbacks in-process; do not put them on the wire. |
| WASM/worker/fallback | Separate heaps match native results; no foreign handles cross. Owner inventories return to baseline, failure preserves existing owners, and valid renders work after refusal. No source/compiler/full-engine fetch appears. |
| Recovery/privacy | Recreate implementations and contexts in a new generation; reject old preparations/admissions and stale publications. Keep entered-credential refusal and existing byte/deadline assertions. |
| Size/closure | Report actual optimized binary/transfer costs and linked implementation evidence. A new binary envelope or moved source pack does not prove parsers or validators are absent. |

## Implemented native structural subset

[`cem_ml::schema::structural_capsule`](../packages/cem_ml/src/schema/structural_capsule.rs)
is available on native Rust targets. `export` accepts an original
`LexicallyScopedDocument`, original source identity made from its bytes, the
logical schema URI, an independently selected registry/build hash and operation
control. The trusted build must supply the matching capture and byte identity;
the exporter does not reparse bytes to establish that association.

Build inspects the entire original arena and compiles it through
`compile_schema_document_model_from_retained`. The portable projection must
equal the selected native element/attribute models, including every field
contract. Unsupported source fields are refused before compilation can ignore
them. Import/base selection, dynamic slots, parser contracts, named executable
types, defaults, custom behaviors/diagnostics, conditionals and wildcard child
policies are outside this first subset. Unknown or unreferenced declarations
cannot disappear during projection.

The binary contains a flat typed CEM source arena, passive lexical/namespace
metadata, compiled element/attribute/count records and sorted exact dependency
pins. It does not use the debug CEMB codec, JSON trees or embedded source text.
Original arena indices identify source occurrences within the capsule; reload
creates a new allocation and restores captured lookups against that allocation.
An old owner cannot use the fresh capture. The source's physical hierarchical
URI, raw-byte BLAKE3 hash/length and local source ID are distinct from its logical
schema URI. Native source maps, expanded declaration names, ordered owning
edges, comments, whitespace and explicit content boundaries are preserved.
Source frames must fit that original input; foreign resource boundaries refuse.
The shared parser's reserved source ID 0 for document-level builder trivia is
preserved with its original ranges. That exception cannot authorize a schema
declaration or field, a foreign source ID or an external resource boundary.

`verify(bytes, trustedPin, control)` checks the whole binary hash, native ABI,
engine version, expected registry hash, logical schema and original source
identity. An allocation-free MessagePack preflight bounds collection lengths,
byte spans, depth and work before typed decoding. Canonical re-encoding refuses
unknown nested fields, alternate shapes and trailing bytes. Verification checks
owning source edges, reachability, occurrence roles, supported record policies
and exact dependency targets/hashes, without parsing source or compiling a
schema. Changing an integrity pin does not enable unsupported operations.

Every element is an explicit exported root. Required/optional attribute and
direct child declarations must resolve inside this single schema; unused
attribute definitions and recursive or dangling child dependencies refuse.
Supported scalar fields use string, integer, decimal/number and boolean bases
with the reviewed scalar facets. Child rules support exact/minimum/maximum total
element counts through the existing shared `FieldContract` model. Name matching,
facet conversion and cardinality evaluation use shared validation semantics;
the capsule creates no namespace or reference-crossing permission.

This first format limits the entire binary to 64 KiB and the original source
coordinate extent to 1 MiB. It retains the design ceilings of 64 combined
element/attribute definitions, 16 count rules per element, 16,384 records/edges,
64 levels and one 100,000-visit budget across each verification operation.
Conservative retained accounting charges the source owner to every consuming
root, bounds each root plus reachable contracts to 64 KiB and the aggregate to
1 MiB. This accounting is a retention bound, not a measurement of peak heap or
allocator overhead. Source/model verification and dependency walks poll the
shared operation control; expanded DAG paths cannot reset the budget.

The returned object owns an inert `SchemaDocumentModel`, retained native source
and passive capture. It has no callback/preparation registry, grant, generation
owner or serialization implementation. Native callers can explicitly supply
that model to existing validation hosts. Component `open`, workers and recovery
have no new entry point. WASM binaries and the full/default backend are unchanged;
no parser-omission or reduced linked-code claim follows from this native step.

The [literal cart schema fixture](../packages/cem_ml/fixtures/structural-cart.cem)
has independent native compilation/reload comparisons. The
[shared-host integration test](../packages/cem_ql/tests/structural_capsule.rs)
compares complete/incomplete and accepted/rejected results under fresh hosts,
with explicit cross-owner grants, repeated reference placements and original
candidate diagnostic URIs/ranges. Compiled-component receiver transport,
inherited/imported schema composition, worker/fallback query ownership and exact
native implementation registration remain gates before deployment admission.

## Native compiled validator query subset

[`cem_ql::query_validator_capsule`](../packages/cem_ql/src/query_validator_capsule.rs)
provides native `export` and `verify` entry points for `CEMQV1` programs, also
available as Rust adapters under the opt-in `query-source-runtime` WASM feature.
Build compiles one checked inline validation function through the existing query
compiler. The binary carries its typed instruction tables, original source maps,
fixed role bindings and exact reviewed function requirements. It contains no
query source to parse, JSON AST, callback or live registration. The older general
query artifact's source-recompilation path is not used by this format.

The independently trusted pin covers the whole binary, original source identity,
declaration fingerprint and selected registry/build hash. The fingerprint binds
the schema, behavior, function and body occurrences, their original headers and
source maps, aliases, body hash, datatype/value representation, candidate
requirement and schema-owned result name. Matching bytes do not grant access to
any source or register a runtime implementation.
The trusted build associates original byte identity with its native declaration
capture, as in structural export; the exporter does not reparse source bytes to
prove that association.

The first instruction subset accepts string/integer/boolean/null literals,
static records, arrays/sequences, comparisons and ordinary binary operations,
non-reference unary operations, conditionals and unqualified field navigation.
Every compiled instruction must be supported, reachable and postordered with
valid tables, fixed input bindings and original query-relative provenance.
Native lowering that introduces another operation or helper is refused rather
than rewritten. Dynamic calls, lambdas/loops, external type/template/state or
resource access, reference traversal, receiver methods and effect-bearing
functions remain outside this subset.

The reviewed function set is sequence `first`, `last`, `count`, `take`, `drop`
and string `length`, `lower`, `upper`, `contains`. Requirements pin each exact
module/name/arity, input/result contract and selected implementation ABI. The
verifier recomputes the complete set across all branches; missing, extra,
duplicate or changed requirements refuse even after repinning the binary.
Execution uses the existing selected registry and value helpers with no general
dispatcher fallback. This is a semantic closure, not evidence that the full
evaluator or optional implementations were removed from a binary.

Bounds are 64 KiB per program binary, 4,096 instructions, 16 KiB original query
text at build time, 64 graph levels, 32 type levels and one 100,000-visit budget
through shape preflight and instruction verification. The original source extent
is at most 1 MiB. Structural and query capsules share the native MessagePack
shape checker, which bounds claimed allocations before typed decoding;
canonical re-encoding refuses ignored or trailing fields. These are wire and
traversal limits, not a measurement of peak heap or total retained source cost.

`DatatypeValidationRegistry::register_verified_query` is an explicit native host
action. The host must independently supply a checked `DatatypeBehaviorContract`
from the original retained declarations, their trusted `SourceIdentity` and its
actual reviewed function registry. Matching registry hashes cannot supply a
missing function. The host can support additional entries; the linked program
dispatches only its verified requirements.
The same byte identity on a fresh owner can match; changed occurrences, headers,
body, signature, aliases or provenance cannot. Registration neither parses nor
compiles a query. It retains the newly supplied owners through the ordinary
registry; duplicate/failed registration preserves existing entries. The native
`CEMQD1` wrapper below adds portable declarations and host binding. Physical
worker/fallback ownership enrollment remains required before application or
recovery admission.

Validation keeps the shared input checks, scoped native node views, required
candidate rules and schema-owned result adapter. Scalar, list and node signatures
remain distinct. Malformed result records remain incomplete. No cached verdict
is installed. A verified query conservatively charges its full expanded graph
cost to the enclosing query budget before evaluation. The native `validator-work`
ceiling is the smaller of 100,000 and 64 times the effective queue size; rules in
the same invocation share it and preserve exhaustion. Existing cancellation and
original diagnostic attribution remain in force.

This native step adds no component/app JavaScript, WASM export, receiver-envelope
version, checkpoint content or recovery grant. Entered passwords/OTP still block
recovery; the 4 MiB checkpoint budget and five-second handoff deadlines remain.

## Native query declaration sources and host lifetimes

[`query_validator_capsule::declarations`](../packages/cem_ql/src/query_validator_capsule/declarations.rs)
exports and verifies `CEMQD1`: a pinned `CEMQV1` program plus its original source
arena and passive lexical metadata. The independent pin covers the complete
envelope and the original program/source identities. Verification uses the
shared native source codec with its text-bearing profile; structural `CEMSC1`
keeps its existing whitespace-only profile and wire representation.

Source records preserve original arena indices, ordered owning edges, literal
attributes, text/raw text, comments, explicit boundaries, source ranges and
physical URI. Passive names and namespace/schema snapshots restore against a
new allocation; old owner handles cannot read the fresh capture. Namespace
provenance is bounded against the same original source as node provenance.
Empty pending-binding maps are passive metadata, while unresolved declarations
or nonempty pending bindings refuse. Dynamic references/value nodes, typed
preludes, source diagnostics and unsupported node records refuse.

The source must contain one top-level schema and the one selected inline query
body. Reload checks its original behavior/function declarations through the
shared `DatatypeBehaviorContract` source checker and matches the program's
occurrences, headers, signature, aliases, body hash and provenance. The retained
body text is original source data; no tokenizer, query parser/lowerer, source
fetch or evaluator runs during verification. This does not compile an entire
effective datatype or grant execution to other declarations in the source.

The complete envelope, including program and source, is at most 64 KiB. Source
graphs retain the 16,384-record/64-level limits and 1 MiB coordinate extent.
Outer preflight, nested program verification, source graph/provenance checks and
declaration inspection share one 100,000-visit allowance and operation control.
These limits do not measure peak heap or the memory of an embedding's owners.

[`FreshStructuralHost`'s query adapter](../packages/cem_ql/src/structural_receiver/query.rs)
requires explicit registration of the exact restored source tree in a ready
scope owned by that concrete host. `link_query` checks a simple original named
type's kind and source membership, actual host functions and registry pin, then
privately retains the ordinary bound validation rule. It is a validator binding,
not complete type/base/facet/tokenizer or named component-receiver admission.

`query_invocation` obtains the scope and policy from that actual host record and
establishes an enclosing shared query budget. `validate_query` accepts only a
matching live binding/invocation pair, preserving input node views and their
existing restrictions. Context changes, new scopes or new directed grants expire
both receipts. Repeating an existing grant does not expire them. Closing/dropping
the host, explicit retirement, foreign scopes and old-generation receipts refuse.
Checks run before and after validation. `replace_query` stages the new binding
first and atomically retires the old one only after successful checks; failed
staging leaves the previous binding usable. No source query is replayed to link.
Invocations retain the selected query scope, policy, diagnostics and budget;
unavailable host callbacks, module resolvers, ambient bindings and context items
are not inherited or kept alive after host disposal.

This is native host lifetime evidence. It adds no WASM entry point, worker or
owned-fallback endpoint, public checkpoint state, permission or publication
receipt. Physical endpoint/inventory enrollment, fresh-heap transport and
credential/privacy acceptance for a new transport remain deployment gates.
Existing Login recovery still refuses entered passwords/OTP and keeps the
4 MiB checkpoint budget and five-second handoff deadlines.

## Portable query lifecycle owners

[`FreshQuerySource`](../packages/cem_ql/src/structural_receiver/query/owners.rs)
adds an explicit original source owner to the native query adapter. After all
source scopes/contexts/grants are established, `host.query_source(declaration,
scope)` checks the actual registered tree, registry pin and original host epoch.
It retains the declaration's original source, lexical capture and verified
program; declarations stored by value inside dependency/module/assembly owners
share those same original components. No declaration, AST, program or host is
reconstructed to acquire ownership. CEMQV1/CEMQD1 verification remains passive,
with its existing format, limits and provenance checks.

`link_query_source(&source, datatype, functions, adapter)` retains a weak link to
that explicit source owner, the ordinary bound rule and its actual selected
registry snapshot. Dropping the last caller-held source owner or retiring it
fences dependent bindings. The binding still accounts for its retained inert
source/program data until it actually drops. Equal pins or fresh verification
of equal bytes cannot replace the original registered source or a foreign host/
generation. `replace_query_source` stages and checks before retiring the previous
binding; missing functions, exhausted owner limits or stale sources preserve the
previous usable binding. Existing `link_query`/`replace_query` remain convenience
APIs and retain an implicit source owner for the binding's original lifetime.

`query_invocation` retains the actual scope, context and enclosing shared work
budget. `retire` fences the request without erasing its retained state. One
original request cannot reenter validation while already executing; a refused
nested call does not poison the outer call. The original native entry remains
active across execution/callbacks. Binding/source/request retirement, context or
generation changes and cancellation are checked after callbacks before output
escapes. Existing budget exhaustion and pending failures are not reset.

For `new_in_generation` hosts, all three owner roles use the original linked
generation ledger and its existing v2 census categories:

| Category | Query ownership included |
| --- | --- |
| Compilation | Original verified query source, passive lexical capture and compiled program. |
| Registry | Query/native validator bindings and imported datatype rule registry snapshots. |
| Preparation | Original query request context and shared invocation budget. |

These are role categories, not counts of individual concrete Rust types. The
six-kind manifest and existing wire formats remain unchanged. Roots remain counted
after retirement or host loss until actual destruction; the existing 256-root/
256-entry limits and native drain/freeze guards still apply. Unmanaged legacy
hosts retain their prior behavior. Raw inert inspection aliases are outside
managed execution authority, not proof of an empty process heap.

Imported query-rule bindings use the same source owners and registry guards;
native validator bindings use the same registry/request accounting. Datatype
compilation context availability checks those original owners, including during
preparation and result validation. No bound rule or copied source hash silently
revives a retired owner.

The [query fixtures](../packages/cem_ql/tests/query_validator_capsule/generation.rs)
cover retained roots, source loss, host epochs, generation closure/loss, atomic
replacement, owner limits and fresh native source reimport. The
[callback fixtures](../packages/cem_ql/tests/native_registrations/query_owners.rs)
cover real registered validator execution, draining, cancellation, request reentry
and late-result refusal; imported-rule coverage borrows the original native
assembly and source components.

These adapters are also available as an opt-in WASM query core. The source-only handoff below supplies a
bounded passive transport record. Physical query-owner endpoints, live binding
transport, production fresh-WASM reimport/relinking and query-specific worker/
owned-fallback checkpoint handoff remain gates before admission. There is no new
production WASM export, application/component JavaScript, credential transfer or effect
replay. Automatic Studio recovery remains off; entered passwords/OTP block
recovery until explicitly cleared, with unchanged checkpoint/deadline budgets.

## Query source handoff

[`CEMQH1`](../packages/cem_ql/src/query_validator_capsule/handoff.rs) is a separate
native, source-only transport contract. `host.capture_query_handoff(binding)`
requires the actual live pure-query binding, original registered source and
current host context/generation. It reads the bound datatype occurrence and
result/diagnostic adapter names from that binding, and preserves its verified
CEMQD1 byte image. A foreign, retired or unavailable source/binding is refused;
native callback bindings are refused even when their result signatures agree.
Capture holds native admission across serialization and rechecks the original
owner before returning bytes.

The packet consists of the seven-byte `CEMQH1\n` magic, a four-byte little-endian
metadata length, canonical MessagePack metadata and the unchanged CEMQD1 bytes.
Metadata carries the ABI, exact ML/QL versions, datatype occurrence and adapter
contract names. Its limit is 4 KiB with bounded collection-length preflight and
eight nesting levels; the original declaration keeps its 64 KiB limit. The whole packet is
bounded to 69,643 bytes. Verification shares the declaration/program's 100,000
work-check budget, cancellation and source/provenance validation; it checks
framing and collection lengths before typed decoding, requires canonical
metadata and refuses trailing data. Original CEMQV1/CEMQD1 formats are unchanged.
Each verified declaration retains one immutable, at-most-64-KiB wire image;
owners share it rather than keeping a chain of previous generations.

`handoff::verify(bytes, expected_pin, control)` checks an independently supplied
outer packet hash and original declaration/program/source pins. It restores
fresh passive native CEM arenas and typed instructions. The selected occurrence
must still be an eligible original datatype with the declared result contract.
No query source recompilation, host creation, native registration, context,
grant or invocation occurs during verification.

For relinking, the embedding explicitly registers that exact restored
declaration's source and lexical capture with a fresh host, chooses its scope
policy and context, then calls `link_query_handoff` with fresh reviewed functions
and a checked result adapter. Adapter names must agree with the capture; ordinary
registration checks their validation shape. These names are requirements, not
portable schema implementations or proof of source-backed result-profile code
closure. Missing functions, mismatched adapters, an equal-but-different source
allocation, a foreign host or unavailable context fail without publishing a
binding. Failed owner allocation drops staged leases. A successful binding
owns a new source/registry lifetime; the caller must explicitly create a fresh
invocation. Old requests, budgets and bindings never become valid on the new
host, and native root/drain accounting remains in force.

Evaluated values/candidates, retained results, callbacks, runtime context/grants
and invocation budgets never enter this record. A packet is a source dependency,
not an ownership receipt, credential policy decision or recovery authorization.
The enclosing checkpoint transaction must still refuse entered passwords/OTPs,
drain requests, prove complete physical ownership, account for all packet bytes
within 4 MiB and finish within five seconds. Copying or repinning a packet cannot
prove any of those conditions. Automatic Studio recovery remains off.

The [native fixtures](../packages/cem_ql/tests/query_validator_capsule/handoff.rs)
exercise independent source reimport, original diagnostic provenance, old-handle
refusal, malformed/oversized packets, payload exclusion, stale owners,
cancellation and failed allocation. The opt-in WASM adapter below shares this
source contract for direct pure-query bindings. Imported-rule/code-owner transport, production
worker/fallback wiring, physical shutdown/replacement and credential acceptance
remain in the [active TODO](todo.md#component-application-wasm-profile-separate-target).
No transport endpoint or component admission is enabled by CEMQH1.

## Opt-in query source WASM adapters

The `query-source-runtime` Cargo feature exposes the existing source verifier,
`FreshStructuralHost` query core, source/binding/request owners, selected evaluator
and conservative validator work budget to Rust WASM embeddings. It is disabled
by default. Its [provisioned byte endpoints](#native-provisioned-query-endpoints)
require original native bootstrap authority; the feature grants no renderer
admission or new application profile. It can compile alongside the full runtime or the
separate `component-runtime` prototype.

This shares the original host and typed CEM/IR paths. It does not substitute a
record-shaped source, parallel evaluator or synthetic scope. The ML feature
shares passive original source snapshots and bounded binary preflight.
`native_source::SourceIdentity` is re-exported at the previous
`structural_capsule::SourceIdentity` native path; source/pin serialization is
unchanged. Original structural receiver installation, imported datatype assembly
and CEMAPPD4 publication remain native-only. The opt-in core now also admits
explicit original native-validator code owners through the
[rule/profile transport](#imported-rule-and-custom-profile-transport), without JS
callback installation or default runtime admission. The WASM query core keeps the actual context epochs, weak source
authority, native entry/root accounting, post-call checks and cumulative request
budget; it does not restart an exhausted request.

Run the isolated acceptance probe with:

```sh
NX_DAEMON=false yarn nx run cem_ql:test:query-source-wasm --skipNxCache
```

The target first authors and checks CEMQH1 through the native Rust fixture,
writing binary source and independent deployment pins under `dist`. It then
builds only the test example with `component-runtime,query-source-runtime` and
uses pinned wasm-bindgen glue. The example supplies an explicit trusted fixture
host, original linked generation, reviewed functions and result adapter from the
shipped schema. It reimports the binary declaration, registers its original
lexical capture, links a new binding and creates a new request. Its adapter setup
parses the trusted shipped schema; the transported query is never reparsed or
recompiled. This fixture is not evidence of a schema-free minimal binary.

The [Node probe](../packages/cem_ql/tests/query-source-wasm.mjs) uses two distinct
WASM memories and a real worker. It checks original diagnostics, unavailable
functions, malformed bytes, cumulative budget exhaustion, cancellation, changed
contexts, cross-host requests, source-only capture and owner counts through
retirement and actual drop. A five-second original scoped drain remains blocked
by retained query owners and closes ordinary admission. Scoped inventory/sealing
is not the full native retirement census or permission to replace a physical
generation. The probe uses test-only exports from the example; production full
and component entry points receive none of these exports.

The separate [shared resource acceptance](#shared-query-resource-transport-and-checkpoint-handoff)
joins actual physical participants to complete checkpoint/journal handoff. The
source-only probe does not issue checkpoint receipts, activate automatic Studio
recovery or copy evaluated credentials into source packets. Entered passwords/OTPs still block
recovery until explicitly cleared; 4 MiB checkpoints and five-second recovery
deadlines remain unchanged. These integration gates stay in the active TODO.

## Shared query resource transport and checkpoint handoff

[`query-source-resource`](../packages/cem_ql/runtime/query-source-resource.mjs)
provides internal shared host controls for original owned WASM and worker
sources. A local endpoint retains the original native backend supplied by its
physical owner; a worker endpoint uses a private MessagePort created/transferred
by that trusted owner. Registration checks the actual original Memory/Worker
participant. Owner tokens are private map keys, never native IDs recovered from
metadata. Copied tokens and foreign endpoints fail before native work.
The producer's `current` witness must include its original ownership/epoch and
invalidation policy; arbitrary native aliases are outside this managed contract.

The worker protocol has only source open, capture and original-owner release.
It copies bounded binary packets and independent deployment pins; JavaScript
never decodes a CEM AST, verifies a schema or evaluates query instructions.
Packet and UTF-8 pin limits are 69,643 and 16,384 bytes. Pending endpoint work is
bounded to 16 operations and ownership to 64 handles. Original sequence/action
matches, cancellation and at-most-five-second deadlines fence results. Unknown,
cancelled or late replies never reopen a failed endpoint or supply retirement
authority. Explicit cleanup remains separate: `cleanupCurrent` can allow release
of an existing original owner behind closed ordinary admission, without permitting
new imports or capture. A failed transport remains fenced even if cleanup later
settles. The complete native census must still prove that native owners dropped.

`QuerySourceResource.open` owns a stable source route. Its recovery policy callback
is separate from ordinary native work, so entered credentials can block recovery
while the application remains usable. `prepareRecovery` checks that policy before
and after original native capture, compares exact source bytes and prepares a
read-only witness. Group capture requires this witness; policy changes, source
changes, disposal or missing peers fence release and allocation. Source packets
exclude evaluated inputs, candidates, results and request budgets.

The resource joins the existing `registerNativeGenerationResource` transaction.
Its packet and UTF-8 pin bytes are charged through `retainedBytes` alongside all
committed checkpoint bytes, within the existing aggregate 4 MiB ceiling. Counts
must be safe nonnegative integers; capture and allocation both check limits.
An exact private group attestation rejects older mixed-vendor constructors that
cannot account for those bytes, before any cleanup. This does not enlarge the
budget or infer ownership from a serialized size. Existing resource adapters
without declared replay bytes retain their previous accounting; independent
package/cache budgets are unchanged.

Resource release waits for the original native/worker acknowledgment within the
existing checkpoint deadline. The physical owner still closes/drains admission,
collects the complete v2 native census and holds its original native retirement
permit through disposal. A native-sealed `CemCommittedCheckpointStore.handoff()`
must remain current, including its terminal external-operation journal. Unknown
outcomes require reconciliation before retirement. No operation is replayed by
source reimport. A copied checkpoint/receipt is not an original capability.

Fresh preparation resolves the target through its original endpoint registry and
checks its participant, actual physical identity and allocation runtime ID. It
reimports the source through the fresh native backend, stages its original owner
and publishes the stable route only inside the shared binding transaction.
Policy changes after allocation, foreign targets, stale checkpoints or failed
preparation revoke the entire fresh group. Old native aliases stay retired.

Run `NX_DAEMON=false yarn nx run cem_ql:test:query-resource-wasm --skipNxCache`.
The [acceptance fixture](../packages/cem_ql/tests/query-resource-wasm.mjs) builds
the test-only full-runtime query probe and uses the production owned factory,
actual WASM query owners, private worker byte transport, complete 36-registry
census, native-sealed checkpoints and the exact shared checkpoint store. Both
fallback and worker modes cover a successful handoff, stale checkpoint refusal
before allocation, credentials entered after allocation and foreign-target
refusal. Native diagnostics are compared across fresh reimport. Source-route
checkpoint islands in this fixture are inert transport data; no DOM owner or
UI state is restored from them. Protocol unit doubles test additional timeout,
cancellation, copied-owner and budget cases without claiming native evidence.

The module and types are shipped with the shared runtime assets, with an internal
`cem-elements` re-export. No Studio/component UI behavior is added. The
[native-provisioned endpoints](#native-provisioned-query-endpoints) supply the
original native backend when explicitly installed. The [source-backed execution owners](#source-backed-query-execution-owners)
now guard direct pure-query bindings in these probes. The separate
[rule/profile transport](#imported-rule-and-custom-profile-transport) covers
imported/native rule leaves and custom profiles. Application bootstrap generation remains
admission gates in the [active TODO](todo.md#component-application-wasm-profile-separate-target).
Automatic Studio recovery stays off. Entered passwords/OTPs still block recovery
until explicitly cleared, with unchanged 4 MiB/five-second recovery policies.

## Source-backed query execution owners

[`query_validator_capsule::profiles`](../packages/cem_ql/src/query_validator_capsule/profiles.rs)
builds a result adapter from original retained CEM sources and lexical captures.
The tree, capture and schema selector must share the actual original document.
Source identities, passive references and lexical metadata are checked through
the existing native source snapshot contract. No trimmed schema, synthetic scope
or JSON AST handoff is introduced.

The profile fingerprint covers the profile ABI, ML/QL versions, selected result
and diagnostic names, ordered original source identities, schema selectors,
namespace bindings and complete source snapshots. Changed same-name schemas
produce different pins. Conflicting claims for one source URI are refused.
Builds accept 1–16 sources, at most 16,384 aggregate AST nodes and 4 MiB of encoded
source/fingerprint data, with cancellation and a one-million-check work ceiling.
The compiled contracts use the shared native `ValueContracts` and
`DatatypeResultAdapter`; verification creates no host, context or registration.
This fingerprint establishes source agreement. It does not attest linked native
implementations, authorize callbacks or prove unused code was removed.

[`structural_receiver::query::execution`](../packages/cem_ql/src/structural_receiver/query/execution.rs)
adds three explicit caller-held owners:

| Owner | Original authority and inventory role |
| --- | --- |
| `FreshQueryCodeOwner` | Borrows the exact original query source and retains the actual reviewed function table narrowed to the verified program's requirements. Missing functions fail. Counts as Registry. |
| `FreshQueryResultProfileOwner` | Retains the verified profile and actual registered source scopes. An independently supplied profile pin must match. Equal source bytes cannot substitute a different original tree. Counts as Compilation. |
| `FreshQueryRegistryOwner` | Borrows that exact source, code owner and profile owner, checking the declared result contract and original host context/generation. Counts as Registry. |

`link_query_owned` retains inert binding data but borrows this registry authority
weakly. Dropping or retiring any required owner, changing its context or closing
its generation fences execution and source capture. Authority is checked before
and after evaluation. `replace_query_owned` stages and checks the replacement
before retiring the predecessor; failure leaves the previous binding usable.
An equal declaration with a newly created source owner cannot borrow another
source owner's code authority. Existing unowned native APIs remain available
for legacy callers; they do not supply the new owned-path evidence.

One managed host/source/code/profile/registry/binding/request session contributes
seven roots: Host 1, Compilation 2, Registry 3 and Preparation 1. Retiring an
owner blocks use without hiding retained memory. Only actual drop releases its
root; a drain cannot seal while those original roots remain. Passive verified
profiles may be retained independently, but confer no execution permission.

The opt-in WASM fixture now requires the explicit deployment sidecar
`cem-query-execution/1` (`QueryExecutionPins`): original CEMQH1 source pins plus an
independently emitted result-profile fingerprint and result/diagnostic names.
CEMQV1, CEMQD1 and CEMQH1 framing is unchanged. The shared JS transport treats
the sidecar as opaque bounded control metadata. It never parses an AST or
derives execution rights from the fingerprint.

Each fresh fixture runtime reconstructs its profile from the original trusted
shipped schema, checks the deployment pin, registers the original captures and
creates new source/code/profile/registry owners before linking. Neither live
owners nor evaluated values cross the worker boundary. This fixture proves
fresh source agreement and original-owner enforcement, not portable custom
schema dependencies or a schema-free minimum binary. Its reviewed function
table is actual native code, but the fixture's build identity is not linked-code
closure evidence.

Both `cem_ql:test:query-source-wasm` and `cem_ql:test:query-resource-wasm` use these
owners. The first checks independent heaps/worker, changed pins, missing code,
retired code/profile/registry, original provenance, budgets and retained roots.
The second joins the renewed owners to complete native census, physical
worker/fallback disposal, sealed checkpoint/journal handoff and credential
refusal. [Native cases](../packages/cem_ql/tests/query_validator_capsule/execution.rs)
also cover foreign original sources, cancellation, atomic replacement and
context/generation fencing.

The [native provisioning API](#native-provisioned-query-endpoints) now supplies
direct-query byte endpoints. The [rule/profile transport](#imported-rule-and-custom-profile-transport)
now adds imported/native rule leaves and custom/multi-source profiles. Generated application bootstraps, linked-code
evidence and CEMAPPD4 transport remain open. No component admission or automatic Studio recovery is enabled. Entered
passwords/OTPs must be explicitly cleared before recovery; the 4 MiB checkpoint
ceiling, five-second deadline and external-operation reconciliation remain intact.

## Native-provisioned query endpoints

[`query_source_endpoint`](../packages/cem_ql/src/query_source_endpoint.rs) is the
shared native implementation for direct pure-query source transport. A trusted
Rust embedding creates the original `QuerySourceEndpoint` with its independent
`QueryExecutionPins`, reviewed selected functions, verified original result-profile
sources, operation control and actual `NativeGeneration`. It loads no implicit
general schema catalog or function registry. Supplied packet/pin metadata must
match that retained deployment; even a valid differently pinned packet cannot
authorize itself. Both result and diagnostic selections must match the profile.

Each open creates an actual fresh host, registers original query/profile captures
and retains new source/code/profile/registry/binding/request owners. The session
borrows its original deployment weakly. Retirement, loss of that owner, cancellation
or a closed generation fences capture and typed native validation. A newly created
equivalent deployment cannot renew an old session. Failed setup releases partial
owners and its capacity reservation. Request budgets remain cumulative; values
and budgets never enter source replay bytes.

The deployment contributes one Registry root, in addition to seven roots for each
session. Its explicit ceiling is 32 live sessions (225 roots including deployment),
within the existing shared 256-root generation ceiling; other native work may
exhaust that shared ceiling earlier. This limit does not enlarge native budgets.
Retirement blocks use while all retained owners stay counted until actual drop.

With `query-source-runtime`, the [WASM adapter](../packages/cem_ql/src/query_source_endpoint/wasm.rs)
exports only `querySourceOpenV1`, `querySourceCaptureV1` and `querySourceReleaseV1`.
No JS provisioning or constructor API is supplied. The trusted Rust bootstrap
installs a weak reference to its original endpoint, and must retain the owner.
Installation verifies the actual linked generation and refuses a second owner or
replacement while old sessions remain. Native registries invalidate inventory
revisions on mutation. Their real deployment/session leases appear in the existing
36-registry v2 census; no new counter-only retirement shortcut is introduced.

Open returns a frozen opaque original JS object retained by the native adapter.
Capture/release require that same object in that same heap. Object copies,
structured clones, foreign-heap tokens and released tokens fail; no numeric owner
ID, native pointer or serialized handle crosses the ABI. Private worker wire IDs
continue to select only the worker transport's own retained originals.

`createQuerySourceWasmBackend` in the shared transport calls these native exports
and checks the original Memory through a weak reference. It cannot switch to an
equal or foreign heap and does not keep disposed memory alive. Local endpoints
and private worker ports retain their existing physical participant, timeout,
opaque owner, byte-budget and recovery-policy checks. The helper performs no
AST parsing, schema validation or query evaluation.
The embedding's original `current` witness must cover deployment/source epochs
and policy invalidation, as required by the shared transport contract; Memory
identity alone is not a recovery-currentness proof. Original cleanup may use
`cleanupCurrent` without reopening ordinary work.

Release and native uninstall permit original cleanup behind closed ordinary
admission. The embedding must remove/drop its original deployment as well as
release sessions before the complete census can be empty and retirement sealed.
Fresh allocation requires a newly provisioned original native endpoint before
shared source reimport and atomic binding publication. Source pins and checkpoint
bytes cannot provide that provisioning authority.

The native and WASM acceptance targets exercise this implementation. Only the
test embedding has `probeProvision`, which supplies the original trusted fixture
profile and code table; it is not a library or default application provisioning
export. Its schema parse remains a fixture dependency, not a minimum-runtime
measurement. Default full/component builds receive none of the new opt-in byte
exports, and no Studio/reduced-component admission is enabled. Application-specific
bootstrap generation, full consumer/application transport and linked-code
evidence remain in the active TODO. The rule/profile transport below adds separate
opt-in endpoints. Credential refusal, sealed
external journals, 4 MiB checkpoints and five-second recovery deadlines remain
required; automatic Studio recovery stays off.

## Imported rule and custom profile transport

Two separate binary formats extend source transport without changing CEMQD1,
CEMNR1, CEMQH1 or `cem-query-execution/1`:

| Format | Passive payload and independent pin |
| --- | --- |
| `CEMRP1` / `cem.query-result-profile-sources/1` | Complete original result-profile source snapshots, identities, schema selectors, namespace bindings and selected result/diagnostic contracts; a packet hash plus the existing profile fingerprint. |
| `CEMRH1` / `cem.rule-source/1` | An intact original CEMQD1 query or CEMNR1 native-validator leaf, its CEMRP1 profile, exact optional local datatype selector and direct/imported origin; a packet hash plus independent leaf/profile pins. |

[`profiles::transport`](../packages/cem_ql/src/query_validator_capsule/profiles/transport.rs)
restores complete original CEM graphs and lexical metadata, then recompiles
through native `ValueContracts` and `DatatypeResultAdapter`. It invokes no source
parser, schema catalog, callback or query evaluator. Repeated source claims must
agree on their whole snapshot and identity; restored selectors from the same
source share one fresh arena. Changed same-name profiles, missing contracts and
incompatible result shapes fail. Profile compilation keeps its ordinary recursive
value-contract checks and bounds; no new dependency resolver or scope is invented.

Profile packets accept at most 16 sources, 16,384 aggregate source nodes, 4 MiB
of encoded bytes, depth 64 and one million bounded verification checks. Aggregate
nodes are checked before any source restoration. Original source graph, lexical,
range and reference limits remain in force. Canonical encoding, independent
packet/profile hashes, ML/QL versions, source identity and cancellation are
checked. Trailing/noncanonical, truncated and oversized data refuse admission.

[`rule_source_transport`](../packages/cem_ql/src/rule_source_transport.rs) frames
bounded canonical metadata and unchanged leaf/profile byte images. Metadata is
at most 4 KiB; original leaf limits remain 64 KiB, with a 4 MiB whole-packet
ceiling. Sources sharing a URI across leaf/profile must agree completely.
Imported origin names the exact original artifact hash and carries no datatype
selector. No runtime handle, consumer expression result, context, grant, callback,
input value or request budget is serialized.

`FreshNativeRuleCodeOwner` retains actual Rust callback code with the complete
implementation ID/ABI/module/export/signature and its actual generation/control.
It contributes a Registry root. `register_native_validator_owner` installs only
a weak original-owner reference; an equal identity registered with a different
callback cannot substitute it. Missing, foreign, dropped, retired or cancelled
code owners fence work, including a callback that retires itself before returning
success. Identity agreement is not a linked-binary/code-omission attestation.

Imported native APIs add `QueryProfile` and `NativeProfile` registrations with an
original `FreshQueryResultProfileOwner`; native registrations additionally require
the original code owner actually installed on that host. The rule binding borrows
these authorities weakly. Legacy registrations still work but cannot fabricate
a transport profile from matching contract names. `capture_datatype_dependency_rule`
checks the live original binding, exact module/source/behavior, current profile
and native owner before and after export.

`link_transported_dependency_rule` checks the exact imported origin and leaf pin
against the freshly verified CEMDP1 input. It uses that input's actual original
source arena and scope, plus the packet's actual registered fresh profile. The
existing compiler still resolves and verifies the consumer's original rule field,
directed crossing, kind and dependency closure. Foreign profile allocations,
unavailable code and failed binding publish no replacement. Native fixtures
validate the new consumer node and behavior identities, then prove original
profile loss fences subsequent capture.

[`RuleSourceEndpoint`](../packages/cem_ql/src/rule_source_endpoint.rs) supplies the
separate opt-in runtime path. Its original Rust deployment owner retains trusted
packet pins and query functions, borrowing native code authority weakly. Opening
reimports the complete profile and leaf, registers original captures and creates
a guarded `FreshTransportRuleBinding`. It loads no implicit schema catalog.
`ruleSourceOpenV1`, `ruleSourceCaptureV1` and `ruleSourceReleaseV1` use original
opaque object tokens, with Rust-only installation into the actual linked runtime.
The shared worker/local adapters retain their original participants and Memory.

A direct leaf with an actual local datatype supports native validation with the
existing cumulative invocation budget. An imported behavior may have no local
datatype. It remains a guarded, recapturable rule registry; standalone validation
refuses instead of manufacturing a consumer. Complete imported consumer/receiver
graph installation in WASM belongs to CEMAPPD4 transport. This distinction keeps
the already working native imported compiler separate from later app admission.

The deployment contributes a Registry root; native code adds its own Registry
root. Hosts, sources, profiles, registries and optional invocations retain their
actual ledger roles. Endpoints admit at most 32 live sessions under the unchanged
256-root ceiling. Retired owners remain counted until actual drop. Session release
and original deployment uninstall remain possible behind closed admission; code
and deployment owners must both drop before the complete census can seal.

`createRuleSourceWasmBackend` selects this explicit ABI. Trusted endpoints may
declare `maxSourceBytes` up to 4 MiB; legacy query endpoints retain their 69,643-byte
default. The worker server validates its own native backend's limit. JS only
copies bounded bytes and control pins; it performs no AST conversion or validation.
Packet and UTF-8 pin bytes still join all checkpoints in the same aggregate 4 MiB
recovery budget, so a maximum-sized packet cannot bypass checkpoint overhead.
Original epoch/privacy witnesses, five-second deadlines, terminal journals and
atomic fresh binding publication remain required.

The component probe checks custom two-source profiles and direct/imported
query/native leaves in independent heaps and a real worker. The full probe covers
40 physical handoff scenarios: the legacy query and four new variants, each on
worker/fallback with successful replay, stale checkpoint, late credential policy
and foreign-target cases. Imported leaf fixtures restore rule registrations, not
consumer DOM or UI. Entered passwords/OTPs still block recovery until explicitly
cleared. Default builds and automatic Studio recovery remain unchanged. Generated
application bootstraps, complete CEMAPPD4 consumer transport, live reduced-control
integration and linked-code evidence remain in the active TODO.

## Exact native validator and tokenizer requirements

[`cem_ql::native_registrations`](../packages/cem_ql/src/native_registrations.rs)
exports and verifies `CEMNR1` requirements natively and under the opt-in
`query-source-runtime` Rust WASM feature. Each record carries an
implementation ID, kind-specific ABI, module BLAKE3 hash, export name, passive
signature, original declaration occurrences and a native source snapshot. The
whole binary, source identity and registry hash must match independently supplied
trusted pins. Verification uses canonical bounded MessagePack and reconstructs
the original native source/namespace capture; it does not deserialize a callback,
parse a query, evaluate an expression, fetch a resource or infer a tokenizer.
Only passive signature types gain serialization; executable contracts and live
context/preparation receipts remain in-process.

The requirement is limited to 64 KiB, 16,384 MessagePack/source placements and
64 levels. Shared source restoration also enforces its 1 MiB original source
extent limit. One 100,000-check operation budget covers preflight, source restoration
and declaration checking. Cancellation refuses verification. Unsupported dynamic
source forms, foreign declaration members, incompatible versions, invalid
ABI/signature combinations and noncanonical/trailing bytes refuse even when a caller repins the
outer bytes. Behavior QName aliases come from their original attribute lexical
scopes; conflicting aliases cannot be flattened into a guessed binding.

The embedding explicitly calls `FreshStructuralHost::register_native_validator`
or `register_native_tokenizer` with both the exact descriptor and the actual
implementation. Matching all descriptor fields is required when linking; an ID
alone grants nothing. The descriptor-to-code association is the trusted native
embedding's assertion, not binary attestation or proof of Rust symbol omission.
Missing implementations return `NativeLinkError::Unavailable`; incompatible
metadata, foreign source owners, expired hosts and duplicates have distinct
errors. Register/remove operations advance the context epoch on success.
A refused duplicate leaves existing bindings usable. Prepare registrations,
source scopes, contexts and grants before creating invocation receipts.

`link_native_validator` checks the exact registered source tree/scope and original
datatype declaration. It uses the ordinary native validation registry, result
adapter and `FreshQueryBinding`/invocation guards. The initial subset requires
the canonical accepted/diagnostics record result; legacy diagnostics-only result
policies remain unsupported here. `replace_native_validator` stages the new
binding before atomically retiring the previous one, preserving a live previous
binding when staging fails. Unavailable callback execution remains incomplete
and never becomes an accepted or rejected validation verdict.

`link_native_tokenizer` returns an opaque `FreshNativeTokenizer`. Keep it alive
while using the compiled selection; dropping or retiring it expires that
selection. `install` accepts only the exact original list declaration/scope and
preserves the pinned `Ready` or `CheckedReplacement` policy. The existing datatype
compiler still enforces inherited tokenizer admission, passes the original
decoded input to each tokenizer, checks byte spans/separators and compares
replacement spans. No whitespace/default tokenizer is inferred from a name.
Availability checks surround calls and separator handling; loss of availability
is `TokenizationError::Unavailable` and preparation/conversion unavailability,
rather than a lexical rejection. Existing limits and operation control apply.
Linking this selection is not complete datatype/preparation admission: ordinary
compiler registrations and in-process prepared-token policies remain required.
No compiled datatype or prepared proof is exported by this adapter.

Actual implementations belong to the fresh host. Bound proxies hold weak
implementation/context/active-binding references, so retaining a compiled type
or retired binding cannot retain an old callback after host disposal. Context
changes, new registrations, removal, explicit retirement, cancellation and
host closure invalidate use. Checking before and after execution prevents a
callback that loses availability from publishing a verdict.

Related independently verified requirements initially have separate source
allocations. `VerifiedNativeRequirement::share_source` explicitly maps one onto
the other's source only when the complete verified source snapshot hash, source
identity and registry pin match, then rechecks the selected original declaration.
This allows a base and derived list requirement to feed the same ordinary
compiler without equating owners by name or URI alone.

The [native registration fixtures](../packages/cem_ql/tests/native_registrations.rs)
cover exact/missing registrations, repinned incompatible records, owner
mismatches, lifecycle and callback release, original UTF-8 spans, inherited
checked replacement, limits and unavailable outcomes. This adds no production
JavaScript, application envelope or WASM export. Actual worker/fallback generation
transport and complete compiled-component datatype admission remain separate
gates. Entered credentials still block recovery; the 4 MiB checkpoint
budget, five-second deadlines and disabled automatic whole-Studio recovery remain.

## Native preparation, conversion, equality and list export

`native_registrations::export_operation` extends `CEMNR1` with an exact original
datatype occurrence, an `OperationMode` and a kind-specific passive signature.
It uses the same bounded source snapshot, canonical encoding, independently
pinned metadata and inert verifier described above. Older readers refuse the
new variants; existing validator/tokenizer encodings retain their semantics.

| Requirement | Explicit fresh-host registration | Shared compiler selection |
| --- | --- | --- |
| Scalar lexical preparation | `register_native_preparer` with `NativeLexicalPreparer` | `PreparationBinding::Ready` or `CheckedReplacement` |
| Conversion | `register_native_converter` with `NativeDatatypeConverter` | `ConverterBinding::Ready` |
| Scalar equality | `register_native_equality` with `NativeScalarEquality` | `EqualityBinding::Ready` |
| Typed-list lexical export | `register_native_list_serializer` with `NativeListSerializer` | `ListSerializerBinding::Ready` |

`link_native_operation` checks the exact implementation descriptor and registered
source allocation, then returns an opaque `FreshNativeOperation`. Keep the owner
alive while using its installed selections. `install` requires the exact original
datatype/scope and feeds ordinary `DatatypeImplementations` methods; refused
foreign sources and duplicate selections leave the registry unchanged. It does
not publish or replace a complete compiled datatype. The descriptor-to-code
association remains an explicit trusted embedding assertion.

Preparation pins `Ready` or `CheckedReplacement`. A known unsupported `Ready`
selection on a derived type, a checked selection without a base, or a scalar
preparer on a whole-list base refuses during verification. The shared compiler
still resolves inherited types, checks actual representation compatibility and
preserves the original preparation chain. Every checked step receives the same
original lexical input/candidate and must produce compatible typed output;
inherited rejection and the cumulative preparation budget remain effective.
Signatures and a verified source plan are not a complete compiled-type proof.

Callbacks receive the ordinary native operation call and caller-supplied runtime;
capsules do not restore an ambient runtime, grants or context. Shared ingress
checks required candidates before callbacks run. Preparation and conversion
validate typed output and retain their separate rejected, pending, unavailable
and failed states. Validation never invokes conversion or lexical export
implicitly. Equality runs through the existing enumeration/constant machinery,
preserving original constant owners. List serialization validates effective
typed restrictions first, retains order/duplicates and applies existing output
and diagnostic budgets. It does not export an AST or invent lexical input.

All four proxy kinds use weak implementation/context/active-binding references
and check liveness before and after a call. Registration/removal epochs,
retirement, owner disposal, context changes and cancellation expire use. A
callback that cancels its native host while returning success yields unavailable,
without publishing a prepared value, converted value, equality verdict or text.
Keeping a compiled descriptor cannot retain native code after its host is dropped.

The [operation fixtures](../packages/cem_ql/tests/native_registrations/operations.rs)
exercise these boundaries through real compiled descriptors, plus repinned
incompatible records, metadata/owner mismatch, invalid output shapes, candidate
requirements, checked replacement and weak ownership. This is the direct typed
native-trait subset. The additional native contracts below use the same ordinary
compiler and preserve explicit selection. No prepared/admission token,
callback, worker owner or permission is serialized. WASM/envelope transport and
atomic whole-generation publication remain separate implementation gates.

## Constants, facets, list orchestration and source-result profiles

`CEMNR1` additionally carries `Constant`, `FacetProfile` and `ListItems`
signatures. `register_native_constant_interpreter` supplies an actual native
constant interpreter. `register_native_facet_profile` explicitly supplies the
selected shared `FacetFamily`; `register_list_item_preparation` explicitly selects
the shared list-item algorithm and item representation. The family/representation
must match the pinned signature. These shared algorithms are not inferred from
a datatype name, source declaration or a requirement alone. Their code/hash
association remains a trusted embedding assertion, not binary attestation.

Facet and list-item modes pin `Ready` or `CheckedReplacement` against the original
datatype occurrence. Known incompatible modes/kinds refuse in the inert verifier;
the ordinary compiler resolves the remaining base/representation dependencies.
Facet replacement intersects every retained family, so unsupported local fields
cannot disappear under a less restrictive replacement. List-item orchestration
uses the explicitly selected tokenizer and effective item preparer. Original
decoded spans, duplicates, inherited admission and cumulative budgets remain
unchanged. Missing tokenizer/item preparation stays unavailable; even an empty
list cannot manufacture the orchestration capability.

The shared in-process `DatatypeAvailability` restriction adds no admission or
portable authority. Additional guards intersect; they cannot replace an earlier
refusal. Installed native selections retain weak guards. Compiler
readiness, datatype/facet validation, declaration readiness, sealed preparation
and external typed producers check applicable availability. Preparation checks
the complete inherited orchestration chain before and after execution, including
empty output. Conversion, equality and constant handling check through result
adaptation; a lazy native record that retires its owner while its fields are read
cannot publish a value or verdict. List export also checks its selected owner
at completion. Existing caller-control stops retain precedence over unavailable
states. Unguarded ordinary full-runtime registrations keep their existing behavior.

Constant interpretation still happens explicitly during native compilation under
its existing budget. The compiler preserves original token/constant owners and
validates interpreted values before retaining them. Completed immutable constants
are compiler data, not cached validation verdicts or portable prepared tokens;
retiring the interpreter prevents new compilation through that registration.

`export_source_operation` additionally pins an original native behavior occurrence
for conversion, equality or constant interpretation. The datatype and behavior
must be members of the same original schema/source in this first subset.
Verification rebuilds the shared `ConversionBehaviorContract`,
`EqualityBehaviorContract` or `ConstantBehaviorContract`, checks roles, QName
bindings, primitive identity and canonical result profile, and admits only a
native implementation. It performs no query compilation, evaluation or fetch.
Canonical profiles are `schema:datatype-conversion-result`,
`schema:datatype-equality-result` and `schema:datatype-constant-result`.

The fresh host explicitly registers the corresponding `SourceDatatypeConverter`,
`SourceScalarEquality` or `SourceConstantInterpreter`, then calls
`link_native_source_operation` with a checked `NativeResultAdapter`. The existing
`from_source` registrars validate the adapter/profile match. Result schemas and
adapters are supplied afresh by the trusted embedding; live `ValueContracts`,
callbacks and registrations are not deserialized. Typed shape/status checks,
original diagnostic sources, pending/unavailable execution and diagnostic/value
limits use the ordinary result adapters. A record containing a pending-like tag
is an invalid result, not completed success or a loading signal.

The [closure fixtures](../packages/cem_ql/tests/native_registrations/closures.rs)
cover real compiler/attribute consumers, sealed/typed producer retirement,
original constants and spans, late adapter retirement, result errors and
repinned incompatible records. All requirements retain the existing 64 KiB
wire, 100,000-check and source limits. Large source arenas refuse; independent
source modules are explicit, and source sharing still requires the complete
verified snapshot identity. No limit is raised or original arena silently pruned.

This completes these native capability adapters, not whole application admission.
Custom result QNames, cross-source/imported behavior closures and compiled-query
operation profiles beyond the existing validator subset remain outside this
source-result format. Full APIs retain those capabilities. The native envelope
below verifies the declared dependency graph; complete semantic/native code
closure and physical worker/fallback ownership must still be established before
component use. Credential refusal, disabled automatic whole-Studio recovery,
the 4 MiB checkpoint budget and five-second
deadlines remain unchanged. No production JavaScript or WASM endpoint is added.

## Bounded native application dependency envelope

[`cem_ql::application_envelope`](../packages/cem_ql/src/application_envelope.rs)
implements native-only `CEMAPPD2`. `EnvelopeBuilder` accepts compiled declaration
artifacts and `ContractArtifact::{structural,query,native}` inputs. These retain
existing `CEMSC1`, original-source `CEMQD1` and `CEMNR1` bytes and typed pins;
raw `CEMQV1` without its original declaration source is not a package input.
`build` validates before returning bytes and an `ApplicationEnvelopePin`.
`verify` produces an inert `VerifiedApplicationEnvelope` with typed capsule
accessors, declaration metadata, compiled templates, the selected registry plan,
receiver requirements and a deterministic dependency-first order.

The independent trusted pin includes the whole package hash, manifest hash and
selected registry hash. The manifest pins exact native ML/QL versions, compiled
IR requirements and registry recipe, component roots, explicit standalone
contract roots and every declared dependency. Opaque native blobs are stored
once by content hash using MessagePack binary records. The existing declaration
metadata JSON is an explicit control sidecar; source trees, queries, schema
models and typed values never become JSON ASTs or JavaScript records.

The verifier recomputes static component references and named receiver names
from compiled templates through the shared declaration/IR inventory. It checks
that every named receiver maps to a verified structural capsule with the exact
schema URI. Static component references, receiver bindings and explicit contract
dependencies must resolve completely from the selected roots. Missing records,
unreachable components/contracts/blobs, conflicting identities, cycles and
unsupported dynamic elements, resources, providers, payloads or behaviors
refuse. This first package subset also refuses recursive component cycles.
Standalone query/native/structural requirements must be explicit contract roots
or reachable dependencies; adding unused assets is not allowed.

Declared contract dependency edges are trusted build assertions, not an inferred
proof of every executable or native implementation dependency. The IR registry
recipe retains the shared inventory's limits too. Consequently
`native_link_closure_verified()` always returns `false`. A matching registry hash
is an integrity identity, not evidence that code is linked, a callback is
registered, a scope is authorized or an optional module is active. Verification
performs no fetch, source compilation, evaluation, callback registration or
runtime publication. It creates fresh inert owners; it does not touch a live
host or the last committed UI.

Identical native requirement source identities share a reconstructed arena only
after the existing verifier confirms the complete source snapshot matches.
Conflicting snapshots with the same claimed identity refuse. Exact native
implementation IDs must agree on the full implementation identity, including
ABI, module hash, export and signature. Independent envelope verification builds
fresh owners. Failed component insertion preserves builder staging, and failed
verification cannot publish a partial registry or replace prior verified owners.

| Bound | Native package limit |
| --- | --- |
| Encoded package and total unique blob bytes | Each at most 4 MiB, including package overhead for the encoded limit. This is independent of the unchanged recovery checkpoint budget. |
| Components / contracts / unique blobs | 32 / 64 / 128. |
| Individual template / declaration metadata | 1 MiB / 64 KiB. Inner capsule formats retain their existing smaller limits. |
| Named receivers | At most 64 per component, with exact required-name agreement. This subset carries structural node requirements only. |
| Declared graph | At most 1,024 distinct edges and dependency depth 32, with complete reachability and no cycles. |
| Outer binary preflight | At most 32,768 MessagePack records and 64 nesting levels, canonical encoding, no unknown fields, trailing data or alternate blob encoding. |
| Verification work | 100,000 outer checks with cancellation; each of the at most 64 nested capsules additionally retains its own existing verifier budget and the same operation control. This is not a single 100,000-check budget across all nested capsules. |

These bounds limit accepted inputs; they do not establish a measured peak heap
budget. Source arenas are never pruned to make a package fit. The
[integration fixtures](../packages/cem_ql/tests/application_envelope.rs) and
[wire fixtures](../packages/cem_ql/src/application_envelope/tests.rs) cover exact
pins, all three capsule kinds, deterministic packing, source/implementation
conflicts, cancellation, malformed and repinned records, closure failures,
limits and unchanged legacy refusal.

The native assembly below now checks literal named-node receiver occurrences.
Remaining gates are scalar/executable named contract composition, runtime
admission/publication, and actual reduced WASM/worker/fallback transport with
physical generation, inventory and checkpoint ownership. Current WASM loaders
continue using scalar `CEMAPPD1` and refuse `CEMAPPD2`; no ABI or endpoint was
added. Entered passwords/one-time codes still block recovery, automatic
whole-Studio recovery remains disabled, and the 4 MiB checkpoint budget and
five-second deadlines remain unchanged.

## Versioned assembly-aware application envelope

[`application_envelope::v3`](../packages/cem_ql/src/application_envelope/v3.rs)
implements native-only `CEMAPPD3` / `cem-application-dependencies/3`. It combines
one exactly pinned CEMAPPD2 application with explicitly named application datatype
roots and flat CEMDM2 assembly inventories. The CEMAPPD2 application keeps its
existing compiled declarations, structural named receivers, original-source
query/native contracts and selected registry recipe. Its original binary and
trusted pin are checked by its unchanged verifier. No old envelope is relabelled
or given executable receiver semantics.

`EnvelopeBuilder::new` takes the built CEMAPPD2 application. `add_assembly` stages
a built CEMDM2 graph, its exact DT1/QD1/NR1 dependency artifacts and its CEMOP2
profile artifacts with exact CEMOS1 sources. Every insertion is transactional:
a refused insertion preserves the prior builder inventory. `build` takes a map
of application root names to `AssemblyRoot { assembly, datatype }` and checks the
complete envelope before returning bytes. Each selected original datatype must
be exported by that assembly's root module. Root names are bounded explicit
application identifiers; they do not bind component receiver names.

The separately typed trusted pin includes package, manifest and selected registry
hashes. The canonical manifest pins exact ML/QL versions, the base application,
all assemblies, legacy leaves, profiles, operation sources and their associations.
Opaque binary artifacts are deduplicated by content hash across assemblies.
The shared nested verifiers independently check the exact CEMDM2/CEMOP2/CEMOS1
pins, original occurrences, source/profile agreement, imports, constant target
order, graph closure and each format's existing bounds. No extra or missing
assembly input, profile source, inventory record or blob is accepted. Every
assembly must be selected by at least one application root.

One shared original-source ledger checks complete snapshots and source identities
across all assemblies, their leaves/profiles, and the base application's
structural/query/native contracts. Equal URIs with different source identities or
different complete snapshots refuse, including conflicting snapshots bearing the
same claimed source hash. A second ledger requires a native implementation ID to
agree on its complete ABI/module/export/signature identity across both CEMNR1
and CEMOS1 records. Neither a matching hash nor a common URI supplies ownership.
The ordinary native source snapshot/verification path remains the only source
handoff; no JSON AST, source-text reparse or synthetic resolver is introduced.

`verify` returns an inert `VerifiedApplicationEnvelope` exposing the verified
base application, explicit roots, typed assembly graphs and a deterministic
leaf/source-before-profile-before-assembly inventory order. The base application
and each graph retain their own nested dependency orders. Profile requirements
map into their own assembly's original source arena after complete agreement.
Independent graphs and verification calls retain independent owners; byte
packing and source agreement do not create cross-graph runtime aliases or grants.
`selections_verified()`, `executable_closure_verified()` and
`native_link_closure_verified()` remain `false`.

| Bound | CEMAPPD3 limit |
| --- | --- |
| Encoded envelope / total unique outer blob bytes | Each at most 4 MiB, including encoded overhead; the separately checked CEMAPPD2 application and nested formats retain their own limits. |
| Assemblies / profiles / legacy leaves / CEMOS1 sources | 8 / 32 / 64 / 64 distinct artifacts. |
| Explicit application roots | 64; each alias has 1–64 ASCII letters, digits, hyphens or underscores. |
| Restored assembly modules | 64 across all graphs, counting repeated graph occurrences even when source bytes agree. |
| Expanded artifact associations | At most 256 assembly-to-leaf/profile associations and 256 profile-to-source associations. |
| Outer binary records | At most 192 unique blobs, 32,768 MessagePack records and 64 nesting levels; canonical binary encoding, no unknown fields or trailing data. The artifact counts imply at most 169 distinct required outer blobs. |
| Verification work | 100,000 outer checks and a separate shared 100,000-check source/implementation ledger, with the same caller cancellation control. Nested application, assembly, profile and source verifiers retain their independent existing budgets. |

The byte and count limits are admission limits, not a measured peak heap bound.
Repeated dependency use counts against the expanded inventory and nested format
budgets even when the outer bytes are stored once. Snapshots are not pruned to fit.
[Native integration fixtures](../packages/cem_ql/tests/application_envelope/versioned.rs)
and [wire fixtures](../packages/cem_ql/src/application_envelope/v3/tests.rs) cover
roundtrip/fresh owners, shared profile/operation bytes, legacy datatype leaves,
structural/query contracts, cross-source and implementation conflict, exact pins,
malformed or repinned records, cancellation, limits and failed staging.

This is a package verification boundary. It does not compile datatypes, link
callbacks, resolve a live reference, evaluate a source, install roots or publish
component permissions. The separately invoked native installation below now binds
explicit actual-host compilations; mixed node/scalar/executable named receivers,
physical-generation worker/fallback transport and checkpoint ownership remain
separate gates. Existing CEMAPPD1 and
CEMAPPD2 readers and component admission refuse CEMAPPD3. Entered credentials
still block recovery until cleared, automatic whole-Studio recovery remains off,
and the 4 MiB checkpoint and five-second deadline policies remain unchanged.

## Native assembly application installation

[`structural_receiver::application::v3`](../packages/cem_ql/src/structural_receiver/application/v3.rs)
adds an explicitly invoked CEMAPPD3 `stage`, `LinkedApplication`,
`InstalledDatatypeRoot` and `ApplicationSlot`. The verified envelope now exposes
`application_owner`, `assembly_owner` and `assembly_ids` so the embedding can
prepare source scopes and compilations from the exact original checked arenas.
The existing borrowed metadata accessors and wire bytes are unchanged. These
inert owner accessors grant no execution or receiver permission.

Before staging, the embedding establishes actual source/lexical scopes,
readiness, directed crossing grants, native registrations, weak rule/operation
owners and ordinary CEMDM2 compilations. `stage` takes the actual host, verified
envelope, existing structural receiver source/binding requests and an exact map
from assembly hashes to caller-held `Arc<FreshDependencyDatatypeCompilation>`
owners. Missing/extra graph bindings refuse. Every selected root must have a ready
compiled descriptor on the exact graph owner supplied by that envelope. A
separately restored graph with identical pins cannot substitute for that source
arena; matching pointers alone are insufficient without the host context,
registry, original descriptor and live weak owners.

Staging checks the existing CEMAPPD2 application through its unchanged native
link path and creates all root guards before exposing the result. It neither
changes a source context or host registry nor compiles, evaluates a selector,
fetches a resource or invokes a user callback. It uses the compilation already
prepared by the embedding; it does not freeze mutable providers or claim a new
source-freshness proof. Changed inputs still require the ordinary renewed
selection/compilation path.

One installation-wide guard covers every named root, including roots from
multiple assemblies. Every access and invocation checks the actual host/context,
base application guards, exact original graph/root descriptors and all required
compilation/rule/operation lifetimes. A missing or retired peer owner makes
retained sibling roots unavailable too. The guard borrows compilation and graph
owners weakly and never retains a native host or callback. The embedding must
keep its compilation and implementation owners alive; passing a temporary last
owner to staging cannot authorize a subsequent publication.

`ApplicationSlot::publish` repeats these checks immediately before one synchronous
native slot replacement. Failed staging/publication preserves the previous slot.
Dropping a staged candidate, clearing/replacing a slot, or explicitly retiring
an installed root revokes that installation's shared root guard. Cloned roots
observe retirement, while caller-held compilations and native registrations
remain usable by other installations. Old structural receiver models retire
with the old base application through its existing guards. Retained CEMAPPD2
models keep those per-receiver contracts; named datatype roots are not grafted
into their attribute models.

`InstalledDatatypeRoot::{validate,prepare_lexical,convert,serialize_list}` delegates
to the ordinary guarded compilation entry point with the selected original root,
input and limits. The original restricted operation/rule contexts, diagnostics,
spans, cumulative/query budgets and missing-capability behavior remain intact.
Each invocation borrows an owner only for the call and rechecks installation-wide
liveness before returning. Clearing/replacing the installation during a callback
withholds late output. Busy compilations refuse nested installed-root calls,
staging and publication; an existing installation becomes usable again when its
ordinary invocation finishes. No callback is replayed to establish readiness.

The checked CEMAPPD3 inventory bounds remain: eight graph owners, at most 64 named
roots and 64 restored graph modules, plus all existing base-application and nested
format limits. Staging adds at most one root guard and one weak ownership entry
per alias. It does not raise a heap/checkpoint limit or establish a measured
resident-memory budget. The
[installation fixtures](../packages/cem_ql/tests/support/datatype_assembly_installation.rs)
reuse ordinary inherited scalar/list preparation, conversion, serialization,
rule and source-context fixtures. They cover exact source owners, root-only
access, callback-free staging, transactional replacement, weak ownership,
cancellation, reentry, peer loss across assemblies, late reports and structural
model compatibility.

This slot installs native root invocations only. It is not a component registry,
physical-generation transaction, portable receipt, mixed receiver contract or
WASM/worker/fallback endpoint. CEMAPPD1/CEMAPPD2 admission remains unchanged and
still refuses CEMAPPD3; the envelope's selection/executable/native-link closure
flags remain false. Entered credentials continue to block recovery until
explicitly cleared; automatic whole-Studio recovery stays disabled, with 4 MiB
checkpoints and five-second deadlines.

## Mixed receiver declaration contracts

[`application_envelope::mixed`](../packages/cem_ql/src/application_envelope/mixed.rs)
implements native-only `CEMMRC1` / `cem.native-mixed-receivers/1`. This standalone
contract packages one compiled declaration and its named node, closed scalar and
executable-root requirements against an independently verified CEMAPPD3 package.
It does not append the declaration to that package or install it in a runtime.
Existing CEMAPPD1/CEMAPPD2/CEMAPPD3 readers and admission rules remain unchanged.

`build` takes the built declaration, an `Arc<VerifiedApplicationEnvelope>` from
the CEMAPPD3 verifier, an alias map and the caller's operation control. `verify`
checks the bytes against a separately typed `MixedReceiverPin` and the supplied
verified parent. The pin covers package hash, manifest hash, parent application
hash and registry hash. The canonical manifest also retains the complete parent
pin, exact ML/QL versions, compiled template/declaration hashes, shared native
requirement-inventory hash and every typed receiver requirement. Template and
existing declaration JSON metadata are opaque binary blobs; the JSON remains an
explicit control sidecar, never a schema/data/AST handoff.

The receiver variants preserve distinct meanings:

| Variant | Checked requirement |
| --- | --- |
| `Node(CapsulePin)` | An exact SC1 structural capsule already present in the parent application. Its complete schema/source/registry pin must match; a hash pointing at another artifact kind refuses. |
| `Scalar(NamedReceiverType)` | The existing CEMAPPD1 closed scalar-facet policy, reused without changing it. Alias, schema/source pins, original source maps, restrictions and supported primitive facets are retained. Nodes, lists, content-type contracts, pending native types and unsupported model policies refuse. |
| `Executable { root, target }` | An exact named root and `AssemblyRoot` target from the parent, selecting an original exported datatype in its CEMDM2 graph. Parent verification already checks exact CEMOP2/CEMOS1 dependencies and source agreement. This record carries no callback, descriptor, prepared value, implementation registration or execution grant. |

Closed scalar records remain trusted build-supplied facet projections. Their
source URI/hash assertions must agree with other scalar records and the parent's
native source identities. This is not an independent reconstruction or proof of
the scalar schema's complete original AST. Native SC1/module/profile sources
retain their existing complete-source verification in the parent; no source
arena is fabricated to turn a projection into an owner. Comparing the original
receiving occurrences and live compiler contracts remains part of explicit host
binding before publication.

The verifier uses the shared native IR inventory and original compiled template.
Every named type must have exactly one map entry and be used; multiple distinct
attributes may use the same alias. All receiver directives must be top-level
literal `attribute` elements with unique nonempty names. This wire subset admits
only `name`, `type`, `itemCount`, `minItems` and `maxItems` fields and no children.
It retains exact field text for subsequent original-occurrence comparison,
including primitive receiver occurrences. Dynamic/nested/qualified directives,
duplicates, defaults, local extra facets, unknown policies and missing/unused
aliases refuse. The closed named scalar profile still retains its supported
schema-owned facets. Existing full/source declaration APIs retain their broader
capabilities.

Literal cardinalities reuse the SC1 count-envelope validator in
[`cem_ml::schema::structural_capsule`](../packages/cem_ml/src/schema/structural_capsule.rs).
Explicit counts use its existing bounded integer policy (0–100,000); contradictory
minimum/maximum/exact fields, malformed values and overflow refuse. Absent local
count fields are preserved as absent. This validates metadata consistency, not a
runtime value, token sequence, input-node capture or navigation permission. The
SC1 wire and ordinary full schema/runtime count rules are unchanged.

Static component dependencies must already exist in the parent, and the new
public tag must not shadow any parent declaration. Unknown construction,
resources/providers and other unresolved inventory gaps refuse. The verified
object exposes the exact shared `requirements()` inventory and component
dependencies so later composition can prepare their demand. A matching registry
hash is integrity metadata; verification does not extend the parent's registry
recipe, prove that additional functions are linked, activate an optional module
or authorize resource access.

`VerifiedMixedReceiverDeclaration` exposes the declaration, compiled artifact,
receiver records, original field occurrences, structural capsule owners and
assembly graph owners. Equal pinned parents can be independently verified into
fresh original source owners; the mixed contract binds to the supplied owner's
arenas. Such metadata sharing is not a cross-host selection or installation
receipt. `runtime_admission_verified()` and `executable_closure_verified()` remain
`false`; verification parses no source text and invokes no user code.

| Bound | Mixed declaration limit |
| --- | --- |
| Complete encoded CEMMRC1 package | 1 MiB, including metadata, template and manifest overhead. |
| Declaration metadata | 64 KiB. |
| Named aliases / all receiver occurrences | 64 / 64; primitive occurrences count toward the latter limit. Alias and attribute names have at most 256 ASCII letters/digits, hyphens, underscores or colons; built-in/reserved type names cannot be aliases. |
| Closed scalar projections | Existing D1 limit: 64 KiB accounted bytes and 16 restriction models per record; at most 1 MiB aggregate accounted scalar bytes, with the stricter complete encoded limit still applying. |
| Outer preflight / work | 32,768 MessagePack records, depth 64, canonical encoding, no unknown/trailing/alternate blob shapes; 100,000 caller-controlled outer checks. Shared IR inventory and parent formats retain their independent limits. |
| Parent package | Existing independently verified CEMAPPD3 bounds, including its 4 MiB encoded limit; it is referenced rather than duplicated in the mixed contract. |

These are input and metadata limits, not a measured peak heap or recovery budget.
[Native integration fixtures](../packages/cem_ql/tests/application_envelope/mixed.rs)
and [wire fixtures](../packages/cem_ql/src/application_envelope/mixed/tests.rs)
cover all variants, scalar rendering parity, exact dependency/alias/occurrence
agreement, source claims, cardinalities, limits, malformed/repinned inputs,
cancellation, fresh owners and unchanged legacy refusal.

The [native binder](#native-original-mixed-receiver-binding) now connects all
three variants to original receiving declarations and live actual-host guards.
The [CEMAPPD4 publication](#versioned-complete-application-publication) now composes
these declarations with a recomputed registry demand and common native guard.
Mixed rendering, generation-bound worker/fallback
transport and checkpoint ownership are not enabled by this format. Entered
credentials continue to block recovery until explicitly cleared. Automatic
whole-Studio recovery remains disabled, with 4 MiB checkpoints and five-second
deadlines. No application/component JavaScript is introduced.

## Native original mixed receiver binding

[`structural_receiver::application::mixed`](../packages/cem_ql/src/structural_receiver/application/mixed.rs)
adds native `stage`, `LinkedMixedReceivers` and `MixedReceiverSlot`. It binds a
verified CEMMRC1 declaration to the actual borrowed `FreshStructuralHost`, an
original registered receiving capture, exact scalar source projections and
caller-held CEMAPPD3 installed roots keyed by exported root name. Source scopes,
lexical contexts, directed grants and native implementation owners must already
exist. Staging renews original receiving type selectors through the ordinary
resolver; those expressions can execute registered query functions. It does not
prepare values, run datatype validation callbacks, register a component or make
the enclosing source schema ready.

Every occurrence must select its original attribute declaration under the
original schema's `attributes` collection. Names and local count text must agree
exactly with the verified declaration. Missing/extra input keys, unregistered
source copies, unsupported fields and changed scalar projections refuse. The bounded
source vocabulary permits literal schema fields and one original native
reference in an executable `type` slot; it does not fabricate a QName reference
or import an AST through JSON. Host registration retains the original captured
names, namespaces and lexical scopes.

| Receiver | Scoped consumption |
| --- | --- |
| Structural node | The existing `StructuralNodeReceiver` links the original literal node field to its exact SC1 capsule. A private field-only snapshot feeds the shared native attribute-reference validator, including actual input grants, complete-selection checks, local cardinality and structural validation. Other fields' pending states are untouched, and the snapshot cannot escape as whole-input schema authority. |
| Closed scalar or primitive | Registered original scalar attribute models must equal the pinned projection, including source maps and authored restriction order. The shared closed scalar converter intersects these models with original local facets and checks local cardinality separately. Typed scalar inspection data is revalidated; it carries no preparation authority. |
| Executable scalar/list | The shared attribute-datatype binder resolves the original type slot through a borrowed adapter over the actual host. Only the exact exported root in the exact parent/graph installation may bind. The first profile permits local count fields; other local executable facets refuse. Typed ingress requires the compiler's explicit typed-only profile, including its dependency checks. |

Scalar source URI/hash/length remain trusted native embedding assertions, coupled
to actual registered source arenas and exact model comparisons. This API does not
claim to reconstruct the original bytes from a model or derive registration
authority from a hash. Structural field snapshots likewise confer no general
schema readiness or whole-document acceptance. Node cardinalities retain the
existing `StructuralNodeReceiver` ceiling of 64, even where wire metadata permits
a larger bound; unsupported bindings refuse instead of weakening that policy.

Executable lexical preparation uses the existing sealed datatype invocation and
its shared remaining work budget. Native preparation and validation borrow the
compiled roots' original rule/operation contexts. The sealed receipt is private
to its invocation, original input arena and attribute occurrence. Consumption
validates prepared values without replaying tokenization/preparation, intersects
local count checks and charges local diagnostics to the same allowance. Token
span inspection also checks the original issuer; missing, foreign, copied,
closed or retired receipts cannot authorize use.

One group guard covers every receiver and required root. Consumption and
publication borrow the actual host exclusively and renew the original executable
receiving selectors; results are checked again before return. Read-only
`is_available`/`current` checks report owner/epoch liveness, not a frozen selector
or invocation receipt. The group also rejects busy access. Missing peer roots,
retired native operations/rules, host epoch changes, closure and cancellation
invalidate the whole group. Weak links retain no reusable compilation or native
callback owner. The synchronous slot replaces all mixed guards together only
after candidate validation; failed candidates preserve the previous installation.
Clearing, dropping or replacing a group retires its receipts and private node
guards without retiring the reusable parent roots or compilation. Callback
retirement or changed selections withhold late results.

The [native fixtures](../packages/cem_ql/tests/support/mixed_receiver_binding.rs)
exercise all three kinds, primitive fields, local facet/count intersections,
explicit typed admission, native preparation spans and budgets, mutable
selectors, exact original sources, grants, weak owners and atomic replacement.
The inert CEMMRC1 verifier, legacy formats, renderer and worker/WASM endpoints
remain unchanged. The [CEMAPPD4 publication](#versioned-complete-application-publication)
now composes these native bindings under a common registry/owner guard.
Generation-bound transport/checkpoint enrollment and linked-code/deployment
attestation remain subsequent gates.
Entered credentials still block recovery until explicitly cleared; automatic
whole-Studio recovery remains off, with 4 MiB checkpoints and five-second
deadlines.

## Versioned complete application publication

[`application_envelope::v4`](../packages/cem_ql/src/application_envelope/v4.rs)
implements native-only `CEMAPPD4` / `cem-application-dependencies/4`. It composes
one independently verified CEMAPPD3 parent, one or more CEMMRC1 declarations and
explicit component roots. The outer package stores the original native blobs
once by content hash. Its typed pin covers the package, manifest and parent
registry identity. The canonical manifest pins native ML/QL versions, the entire
parent pin, each mixed declaration pin, component roots, recomputed requirements
and combined registry demand. Legacy readers retain their original formats and
refuse CEMAPPD4.

Verification restores the parent once and verifies every mixed declaration
against that exact original owner. It does not independently restore a parent
for each child, reinterpret older formats or fabricate schema/query sources.
The existing bounded dependency-order implementation checks the complete static
component graph: duplicate/shadowed tags, missing or unreachable components,
unknown roots and cycles refuse. Source assertions must agree across parent
native sources, declaration source identities and closed scalar projections.
The parent's complete native snapshot/implementation agreement is preserved;
declaration/scalar hashes remain trusted build assertions, not reconstructed
original byte proofs or source permissions.

The [combined demand](../packages/cem_ql/src/application_envelope/v4/demand.rs)
is recomputed from every checked original template through the shared IR
inventory and selected-registry planner. It also includes the checked function
requirements of portable validator programs in both parent standalone contracts
and assembly leaves. The exact union, original requirements pin, template-plan
pin and reviewed execution-unit recipe form
`cem-application-registry-demand/1`. Its JSON projection is an explicit build
control sidecar of identifiers and hashes, never an AST/DOM/data handoff. The
parent registry recipe remains unchanged; a matching registry identity does not
implicitly extend its function selection or prove linked-code omission.

| Boundary | Limit |
| --- | --- |
| Complete encoded outer package | 4 MiB, including original parent and mixed blobs plus manifest overhead. |
| Mixed declarations / all component declarations | 32 / 64. Existing parent limits still apply. |
| Original mixed receiving occurrences | 256 total; the existing per-declaration 64 limit is unchanged. |
| Component graph | Shared dependency policy: at most 1,024 edges and depth 32, with exact reachability from explicit roots. |
| Outer binary/control work | 32,768 MessagePack records, depth 64, canonical records, exact blob inventory and 100,000 caller-controlled checks. Each nested format retains its own bounds. |
| Requirements / registry control sidecar | At most 1 MiB each. These are metadata/input bounds, not heap or checkpoint measurements. |

CEMMRC1 continues to permit static references only to parent components. Each
added mixed declaration must therefore be a selected component root in this
version; sibling references require their own future declaration contract.
Unknown/dynamic/provider/resource policies retain the existing refusal rules.
The original receiving schemas, data scopes, directed grants and executable
registrations are supplied by the trusted actual embedding for native linking;
this package does not yet transport those live inputs into another heap.

[`structural_receiver::application::publication`](../packages/cem_ql/src/structural_receiver/application/publication.rs)
adds a caller-held `RegistryOwner`, `MixedSource`, `LinkedApplication` and atomic
`ApplicationSlot`. `link_registry` checks the exact derived function set and
stamps the original package instance and actual host epoch. Missing/extra
entries or an equal-pinned independent package cannot substitute that owner.
The native embedding retains the registry and reusable compilation/operation/
rule owners; the publication keeps weak authority links to them.

`stage` requires complete parent source/binding/compilation inputs and an exact
mixed-source map. It reuses CEMAPPD3 staging and original mixed receiver linking,
then enrolls every mixed group in one publication guard. That guard checks the
actual host, exact registry owner, parent installation, all required native code
owners and every peer field group. Source selectors still run on the real host
through the ordinary resolver. Staging changes no host registry or last
committed slot. As with the preceding native APIs, a prepared staged application
can be inspected/validated before choosing to publish it.

Every field/preparation/receipt operation renews the whole publication's original
receiving selectors. The native API also wraps exported-root validation,
preparation, conversion and list serialization, parent structural input
validation and a synchronous borrowed-registry callback. It does not expose raw
parent installation/root handles or whole-schema models as unchecked publication
authority. A common busy guard rejects reentry, including peer/readiness access.
Read-only availability remains liveness rather than a frozen selector receipt.

Publication checks candidates again before replacing one slot. Failed candidates
preserve the previous installation. Clearing, replacing, dropping or retiring a
publication invalidates retained application handles and mixed preparation
receipts, even when an embedding still holds an `Arc` to the old application.
Registry/peer/code-owner loss, host epoch changes, closure and cancellation
invalidate all operations. Post-call checks withhold late results; the registry
callback releases its temporary owner lease before rechecking caller ownership.
Private parent guards retire with this publication while reusable compilations
and separately staged installations remain available. Retained handles keep no
host callback, reusable compilation or caller-held registry owner alive.

This establishes an explicit **live native owner closure** for the selected
publication. It is not an attestation of binary symbol omission, arbitrary
provider code closure, optional-module activation, physical runtime generation
ownership or deployment authority. Inert `native_link_closure_verified()` and
`runtime_admission_verified()` remain `false`. There is no renderer/component
admission, worker/fallback endpoint, checkpoint transport or new production
JavaScript. Those require generation-bound enrollment and fresh-source/owner
handoff before use. Credential refusal, disabled automatic whole-Studio recovery,
the 4 MiB checkpoint budget and five-second deadlines remain unchanged.

The [composition fixtures](../packages/cem_ql/tests/application_envelope/publication.rs),
[wire/bounds fixtures](../packages/cem_ql/src/application_envelope/v4/tests.rs)
and [native lifecycle fixtures](../packages/cem_ql/tests/support/application_publication.rs)
cover original owners, query demand, source conflicts, malformed/repinned inputs,
limits, exact registry selection, peer invalidation, stale receipts, weak
lifetimes, reentry, callback retirement and atomic replacement.

## Original application source handoff

`application_source_transport` adds the separately versioned passive `CEMAH1`
(`cem.application-source/1`) packet. It frames the **unchanged original CEMAPPD4
bytes** and complete original receiver/scalar source snapshots. Each source role
identifies its actual base or mixed component, scalar alias, source/schema URI
and original attribute-node selectors. There is no JSON AST or source-text
reparse. Shared external source claims restore one native arena; original
namespaces, ranges, parent/owning edges and lexical capture metadata survive.

Capture holds the live publication's native admission and busy guard. It checks
exact original tree/capture pointers, scalar identities, host context, registry,
compilation and publication owners before and after copying. Equal bytes or pins
from a foreign capture cannot stand in for an original owner. Source identity
hashes remain trusted build input supplied with those original captures. Native
source agreement spans the existing package's contracts, assemblies, operation
profiles and leaves, plus declaration/scalar hash claims; contradictory shared
URIs refuse. Capture does not evaluate a selector or call runtime code.

The independent packet hash and CEMAPPD4 pin must both match. Verification checks
canonical framing, native versions, complete component/scalar roles and selectors,
source agreement, existing nested package limits and cancellation. Limits are
4 MiB for the **entire** packet, 2,112 external sources/roles, 16,384 aggregate
external nodes checked before restoration, 64 selectors per role, MessagePack
depth 64 and one million preflight visits. These are bounds on this passive
packet, not a new checkpoint allowance or a peak-memory measurement.

`VerifiedApplicationSources::receivers()` returns descriptors over restored
original trees and captures. `matches_publication()` requires those exact arenas
and the actual verified application object in a live native publication. The
embedding still supplies fresh contexts, native callbacks, code/registry and
compilation owners, original input data and directed crossing permissions before
staging/publishing. None of those, runtime expression results, credentials or
live native handles is serialized. Missing or retired owners still refuse.
Native acceptance rebuilds node, scalar, executable and list receivers, recaptures
identical bytes and proves disposal and generation-drain refusal.

The separate **default-off** `application-source-runtime` Cargo feature makes the
existing application/module verifiers, compiler, preparation, receiver contracts
and publication guards available on WASM. It preserves the full/default and
compiled-component targets. The test-only `application_source_probe` reads a
native-emitted packet in independent heaps and a real worker; it creates no
installation authority. Run
`NX_DAEMON=false yarn nx run cem_ql:test:application-source-wasm --skipNxCache`.
The [native-provisioned application endpoint](#native-provisioned-application-installation)
now supplies original bootstrap/code-owner installation separately. Complete
installed worker/fallback session/checkpoint handoff, generated bootstraps,
component admission and linked-code omission remain explicit next gates.
Entered credentials still block recovery; automatic whole-Studio recovery stays
off, with the existing aggregate 4 MiB budget and five-second deadlines.

## Native-provisioned application installation

`application_source_endpoint` adds original native `ApplicationBootstrapOwner`,
`PreparedApplication`, `ApplicationSourceEndpoint` and session owners. The caller
provides actual Rust `ApplicationBootstrap` code and an independent CEMAH1 pin,
actual generation and original operation control. The endpoint borrows that exact
code owner weakly. Matching names or pins cannot replace a dropped, retired or
cancelled original owner. Bootstrap invocation keeps its native admission entry
and code callback alive while releasing the temporary authority lease before
calling embedding code, so callback-time loss of the caller's owner is observed.
Concurrent/reentrant use of that bootstrap refuses while it is busy.

The bootstrap constructs a fresh host over restored original packet trees,
explicit contexts, input sources, code registrations, lexical preparation and
directed crossing grants. It must be a construction operation; source reimport
is not permission to replay external writes or acknowledged actions. The shared
`PreparedApplication::stage` then uses the existing native publication linker,
exact original registry and compilation owners. Typed `RetainedApplicationOwner`
variants retain required rule, operation/tokenizer, query/source/profile/registry
and native-rule code owners; arbitrary JavaScript objects or opaque owner bags
are not accepted. Every supplied owner must be live on the actual host. The
endpoint additionally requires the **same verified source object**, actual
physical native generation and original operation control supplied to bootstrap.
Equal-byte source substitutions and foreign host/control owners refuse.

Publication guards enroll the original endpoint weakly once. The dependency is
checked by root, receiver, registry and preparation/evidence operations before
and after native callbacks. Escaped handles therefore cannot outlive bootstrap
or endpoint retirement. `with_application` renews actual mutable selections
before and after the native operation, under native admission. `capture` checks
the live publication and recaptures original sources. `replace` fully prepares
and publishes a candidate before replacing the prior session; a failed candidate
leaves the prior publication in place under its existing owner checks. Dropping
the session retires its slot,
closes its host and releases real owners. Cleanup remains possible after native
admission closes, while retained handles/owners remain counted until they drop.

The endpoint permits at most **eight sessions**, with the existing shared
256-resource generation ceiling able to refuse larger applications earlier.
Source packets retain their 4 MiB bound and pin metadata its 16 KiB bound. Actual
bootstrap and endpoint owners each hold a counted Registry lease; existing host,
compilation, publication and retained-owner leases cover session resources.
Native census revisions include mutations of opaque session tables. Reentrant
table access uses shared `NativeRegistry::try_borrow_mut` to refuse without
unwinding and invalidate old census observations.

The default-off `application-source-runtime` WASM lifecycle ABI exposes
`applicationSourceOpenV1`, `applicationSourceCaptureV1` and
`applicationSourceReleaseV1`. An original Rust embedding installs/uninstalls the
endpoint against the linked generation; there is no JavaScript provisioning API.
Tokens are original frozen objects in that heap. Native embeddings can use typed
sessions without exporting data objects, raw pointers or numeric owner handles.
Unprovisioned calls and unknown tokens refuse. Default full/component exports
and component admission remain unchanged.

Native tests cover complete mixed consumers and source recapture, invalid and
foreign sources/control, missing code owners, failed replacement, bootstrap and
operation callback retirement, escaped preparation handles, cancellation,
shared session bounds and drain cleanup. The WASM probe covers the passive
reader and refusal without provisioning in separate heaps and a real worker.
The [application handoff acceptance](../packages/cem_ql/tests/application-resource-wasm.mjs)
now installs fresh real owners in workers and owned fallback heaps through the
shared `createApplicationSourceWasmBackend` and existing source resource protocol.
Twelve physical scenarios cover native-sealed journals, complete census, release,
retirement, fresh source binding, original tokens, reentrance, expired owners,
stale/foreign targets, late credential refusal and combined byte budgets.
Run `NX_DAEMON=false yarn nx run cem_ql:test:application-resource-wasm --skipNxCache`.
The fixture restores original native input snapshots as immutable bootstrap data;
the reduced frame API below admits separately supplied native application input.
Credential refusal, manual Studio recovery, aggregate checkpoint/replay budget
and deadlines remain unchanged.

## Source-publication-bound reduced frames

`ApplicationSourceSession::render_island_native`, `project_island_native` and
`render_inputs_native` consume the actual installed CEMAPPD4 publication. The
default-off WASM counterparts are `applicationSourceRenderIslandV1`,
`applicationSourceProjectIslandV1` and `applicationSourceRenderInputsV1`. They
require the same original opaque session token as source capture; the frame
call cannot create code owners, schemas, contexts or execution grants.

Each frame enters the original application owner, renews mixed receivers and
registry selections, and uses the publication's original compiled artifact and
selected registry. Closed scalar receiver contracts include both their original
type definition and declaration-local restrictions. For example, a type allowing
2–5 cannot override an original receiver restriction of 3–4. The source schema
is reviewed directly from the original retained tree; required attributes,
children, field contracts and unsupported instance topology need their explicit
input adapter. Node/executable receiver ingress without such an adapter refuses;
the existing original native validation/root-invocation APIs remain available.

Direct frames decode bounded CEMV input with the shared native codec. Separate
JSON routing metadata names attributes, dataset aliases and slices by native
root index; it never carries an AST or reconstructs data values. Dataset aliases
derive from their actual `data-*` attribute. Attribute routes retain the empty
namespace requirement, including attribute-to-slice routes. Original root source
keys, hierarchical source URIs and source-map frames are required. Invalid
indices, collisions, reserved names, wrong domains or missing sources refuse
atomically. The existing generic prototype input path retains its prior source
policy.
Imported JSON slices retain their native CEM tree structure and use native tree
navigation; routing does not turn them into object-style records. Explicit
island record values continue to use the shared island-domain view.

Islands use the existing qualified v0.2.0 native adapter, including dataset/source
agreement, instance identity, payload review, and explicit state-domain limits.
Projection returns native CEMV output; original HTML slot nodes retain their
XHTML namespace, source URI and source maps. This is a frame operation and does
not store mutable input in the source packet or authorize automatic input replay.

Frames have the existing 1 MiB input/output limits, 16 KiB routing bound, 64 routes,
bounded native value graph and five-second scope tied to the original abort
signal. Capability-bearing declarations, retained styles, nested components,
unresolved requirements and resource imports await their real ownership adapters
and refuse before output. Expired bootstrap/endpoint owners fence render and
projection as well as source capture. Rejected frames leave the publication
usable. The opt-in ports preserve default exports and production admission.

The native fixture covers original scalar restrictions, qualified projected
sources, direct attribute/dataset/slice data, malformed routing, source/namespace
refusals, nested component ownership and owner retirement. The real worker and
fallback handoff acceptance invokes the same ABI before/after fresh rebinding,
compares HTML and checks projected source identity inside native code. It never
exports an AST to JavaScript. Shared declaration/CSS/provider/live-control wiring
remains a separate execution gate. Original node and lexically prepared executable
inputs now have the native-owned adapter below.

## Native-owned receiver frames

`ApplicationSourceSession::register_receiver_input` accepts an original
`RetainedCemNode` from a source already registered on the actual host. It returns
an opaque native `ApplicationReceiverInput` bound to that exact application
publication. Copies restored into another arena, equal-byte sources, foreign
sessions and inputs retained across publication replacement cannot substitute
for it. Each owner holds a real generation resource lease; at most 64 input
owners share a session and the existing 256-resource generation ceiling.

The initial topology profile is one original empty-namespace instance element
with exact named required/optional attributes. Native consuming-name metadata
determines namespace admission. Wrong or unfinished names, duplicate/unknown
attributes, missing required fields, child content and field-contract topology
refuse before rendering. The original model retains its pending datatype slots;
this review does not publish the whole schema as ready.
Receiver names reuse the shared native input policy: runtime roots such as
`datadom`, `instanceID` and `island` cannot be shadowed by original input fields.

Every `render_receiver_native` / `project_receiver_native` call consumes the
original fields again. Node fields use the existing structural receiver and
original lexical reference resolution, directed grants, count limits and target
validation. Selected values remain original retained nodes. Closed scalars use
their original source restrictions. Executable fields create their original
lexical preparation invocation, prepare once, and consume its sealed evidence
through the existing original-root validation and local counts. Missing preparers,
pending/rejected preparation, unsupported lexical slots and expired code refuse.
An unavailable preparer never implies a typed-only input grant; the existing
explicit typed-only producer policy remains separate.

A borrowed native receiver frame supplies the resulting values to the shared
renderer only for its exact original compiled artifact. It has no public
constructor, wire representation or reusable acceptance receipt. Renderer-side
string coercion does not replace the native receiver validation. Native node
owners and scalar/prepared source attribution remain attached to bound values.
The selected registry is still the actual renewed application registry. Calls
retain native admission through output serialization and recheck the source owner
afterward. Native callbacks retain their original operation control; the frame's
five-second scope is checked at receiver boundaries and during rendering. A
16,384-node input arena, 64 fields, 16,384 total bound values, 1 MiB scalar-value
allowance and existing output/graph limits bound the initial adapter.

The opt-in WASM ABI adds `applicationSourceRenderReceiverV1`,
`applicationSourceProjectReceiverV1` and
`applicationSourceReleaseReceiverInputV1`. Only original Rust embedding code can
call `wasm::register_receiver_input` to mint an original frozen input token.
Neither a JavaScript constructor nor a numeric node/scope handle is exposed.
Tokens remain in their exact session and heap; copied tokens refuse. Input
cleanup remains available behind closed admission or expired code, and releasing
the session drops every remaining input owner. Native census revisions and real
resource leases cover these owners without another untracked registry.

Source-only capture returns
`cem.application.receiver_checkpoint.unavailable` while any native input owner
is live, including a stale owner retained after replacement. A separate Rust
adapter captures one current original input, its passive dependency sources and
its original named native node bindings. This profile uses default reference
policies and direct named node references. Capture requires embedding approval
for each source and applies credential checks, source and node limits, and the
remaining workspace checkpoint budget, including the deployment and its pins.
A short-lived, single-use native seal binds the exact bytes and expires when
its operation is cancelled or its origin replaces the input or publication.

The fresh bootstrap registers the decoded source owners, recreates the lexical
contexts and explicitly authorizes cross-scope references. Restoration checks
the deployment, the new generation, and those exact fresh owners before
enrolling a new native input. The wire carries no live scope IDs, grants or
runtime handles.

The opt-in receiver route of the shared source resource charges the capsule,
deployment, and retained journal to the same workspace budget. The original
native session consumes its seal and releases its owners together at retirement.
The journal must name the original runtime, receiver route, deployment and
capsule revision, with terminal outcomes. The shared binding transaction carries
the resulting pin independently of the capsule and supplies the original sealed
journal to the fresh embedding. The fresh native generation checks both digests
before decoding, and the Rust bootstrap recreates the scopes and grants before
the host route can bind. Only the trusted embedding and its private worker
transport handle this pin.

The restored write journal suppresses operation IDs recorded for the receiver
route and writes at or before its accepted event sequence. A later input can
claim a new operation once. The worker and owned fallback fixtures edit both
the receiver and its referenced data, render from the fresh owners, and exercise
consumed writes, stale edits, credential refusal, tampered packets and journals,
and the aggregate byte limit. Typed-only producers and production admission
remain separate gates.

## Native publication generation enrollment

[`cem_ml::runtime_admission::generation`](../packages/cem_ml/src/runtime_admission/generation.rs)
adds an original native `NativeGeneration` bound to an actual `Arc<NativeAdmission>`.
`claim_current` uses the admission object of the linked native runtime instance;
`claim` supports a native embedding that already owns its actual admission object.
Neither accepts a runtime label, serialized identity or caller-supplied census as
authority. One admission object can be claimed once, and ordinary closure is
monotonic. A new wrapper around the old closed object cannot create a fresh
generation. Dropping the original generation owner closes its admission gate.

`runtime_admission::linked_generation()` retains one original generation per
actual linked runtime instance. Every shared native entry and closure uses this
same owner. Native embeddings use that accessor when enrolling hosts in the
linked runtime; claiming an unrelated caller-created admission object does not
identify a JavaScript Worker or Memory. The [physical join](#physical-generation-census-and-checkpoint-handoff)
below connects that linked owner to existing worker/owned-fallback retirement.

`FreshStructuralHost::new_in_generation` binds an empty actual host before source
scope, lexical context, native implementation or compilation registration.
Legacy `new` remains an explicitly unmanaged native API. Host checks reject
closed/lost generations. Ordinary publication/type selection, compilation,
root/field execution, structural input and validation paths retain owned entry
permits across callbacks. These permits use the same native admission lock and
active-entry counter as the existing bridge, including when a callback triggers
closure. Results cannot escape the existing post-call owner checks after that
closure; cleanup and object destruction remain available.

The [native resource ledger](../packages/cem_ml/src/runtime_admission/resources.rs)
records original, noncloneable object leases and count-preserving mutations.
It is not an AST, source export, checkpoint, allocation-size report or complete
heap census. Its `cem-native-enrolled-generation/1` control inventory reports
only counters, closed/frozen state and a per-admission revision:

| Root kind | Actual retained native ownership |
| --- | --- |
| Host | The enrolled host, including its original registered source scopes and native implementation map. |
| Compilation | Original datatype assembly compilations and query source/program owners with their guarded dependencies. |
| Registry | Exact CEMAPPD4 registry selections and query/native/imported-rule binding snapshots. |
| Publication | The common parent/mixed/peer publication guard. |
| Preparation | Original native preparation/query invocations, their contexts/inputs and remaining budgets. |
| Evidence | The private issuer-bound preparation receipt and its retained original values/input. |

There are at most 256 enrolled roots and 256 simultaneous native entries. Resource
IDs and revisions do not wrap into renewed authority. Counts remain live until
the owning objects actually drop: retiring an application, closing a host or
closing an invocation does not pretend that retained native allocations vanished.
Retained `Arc` application handles and evidence keep the census nonempty even
when they can no longer execute. Plain inspection data is not a managed execution
capability; raw/unmanaged aliases and arbitrary direct memory access remain
outside this enrollment contract.

`begin_drain(timeout, control)` closes the original admission object, caps the
attempt at five seconds and returns one original pollable drain. It starts no
background task, serialization, refetch or replay. `poll` reads actual entry and
resource counts. `seal` succeeds only when those enrolled counts are zero and
returns a private witness bound to the original drain/generation and exact local
revision. Foreign, cancelled, expired, dropped or superseded drains cannot become
current proofs. A native cleanup entry or resource mutation invalidates a prior
witness even when counts return to zero. Failed attempts leave ordinary admission
closed; cleanup can finish and a new original drain can be prepared.

This witness is scoped enrollment evidence and grants no replacement authority.
The legacy complete command census remains unchanged; physical linked runtimes
use the separately versioned complete census and original joined permit below.
An enrolled gate refuses the legacy freeze path. Live enrolled roots also block
closure validation under the native lock, so an incomplete zero-root DTO cannot
bypass those owners. A scoped inventory cannot replace the complete census or
original embedding/checkpoint checks.

The [core fixtures](../packages/cem_ml/tests/runtime_generation.rs) cover owned
work, exact retained roots, limits, duplicate claims/drains, count-preserving
cleanup, forged empty census rejection, cancellation, expiry and owner loss.
The [publication fixtures](../packages/cem_ql/tests/support/application_generation.rs)
cover callback-time closure during execution and original type resolution,
retained retired handles, dropped owners, stale receipts and fresh native source
reimport/recompilation. They use separate actual native admission objects; they
do not impersonate physical JavaScript runtimes using labels. Same pinned bytes
on a fresh host do not admit the old compilation or receipt.

No metadata reconstructs live handles, no original source is fabricated and no
native tree passes through JSON. Fresh-heap source/registry-owner transport,
portable query-owner adapters and component admission remain gates.
Link-closure/admission flags stay false.
Entered credentials still block recovery until explicitly cleared; automatic
whole-Studio recovery stays off, with 4 MiB checkpoints and five-second deadlines.
Component/theme implementations and declarative UI ownership are unchanged.

## Physical generation census and checkpoint handoff

Each full linked WASM instance now retains its original native generation before
admitting its first shared entry. Workers use their own linked instance; owned
fallback factories close over one instance and Memory each. Fresh allocation
initializes a new native owner. The shared original-participant registry still
uses actual Worker/Memory identity, never a runtime label or serialized receipt.

`commandNativeResourceInventory` emits schema version 2 and profile
`cem-ml-cem-ql-generation-v2`: the complete 30-registry base manifest plus
`enrolled-hosts`, `enrolled-compilations`, `enrolled-registries`,
`enrolled-publications`, `enrolled-preparations` and `enrolled-evidence`.
These additional counts come from the original admission object's live ledger.
Missing/unknown/duplicate entries, unapproved discardable roots, overflow and
incomplete base manifests refuse collection. V1 readers refuse this distinct
profile; legacy native collectors and v1 DTO parsing retain their prior contract.
The managed parser checks the exact v2 manifest. Counters remain control metadata,
never an AST, source export or portable authority.

At final native preparation, the owning transport collects the complete native
census again, starts an original bounded drain, seals its exact local revision
and joins both revisions under the admission lock. The private
`GenerationRetirementPermit` freezes ordinary entries and cleanup only while all
actual enrolled roots, active work and complete-census blockers are zero. Foreign
or stale witnesses, native changes, cancellation and deadlines refuse. The permit
retains its original drain; dropping a temporary preparation object does not
discard that proof. Cancellation/expiry never reopen cleanup automatically;
only dropping the original permit does so, while ordinary admission stays closed.

Bounded lifecycle checks retain cancellation, deadlines and previously recorded
failures without attempting ordinary work-memory admission. This keeps drain/
retirement available under hard memory pressure using recovery headroom. It
admits no source parsing, rendering or new work; live roots still block the
complete census. Prior failures are never cleared to obtain a permit.

The original joined permit retains the five-field native scalar confirmation.
Rendering workers additionally attest the v2 profile read from their fresh
native census; host adapters refuse older worker acknowledgments. Managed command
retirement also requires an original coordinated v2 census. Legacy v1 snapshots
remain readable for inspection and cannot authorize this physical retirement.
The adapters' host paths still require complete original embedding/publication/source coverage,
retained sealed checkpoint/journal bytes and final currentness checks before
physical termination or dropping owned references. Native confirmation is
rechecked after fallback ownership callbacks. Reentrant disposal/abort is fenced;
synchronous callbacks cannot consume the remaining host deadline and then retire.
Failed or late preparation preserves host bytes, grants no replacement receipt
and follows existing bounded abort/fencing rules. No command or external effect
is replayed. Legacy fallback globals still cannot claim owned disposal.

This joins physical lifecycle infrastructure; CEMAPPD4 and its structural receiver
remain native-only. New envelope/query-owner transport, original source/code/
registry reimport on fresh WASM hosts and reduced component admission are still
separate gates. No application-installation endpoint is added. Credential policy,
4 MiB whole-workspace checkpoints, five-second deadlines and disabled automatic
Studio recovery remain unchanged.

See [native join fixtures](../packages/cem_ml/tests/generation_retirement.rs),
[real independent-memory checks](../packages/cem_ql/tests/owned-generation-wasm.mjs),
[command worker/handoff checks](../packages/cem-ml-cli-npm/tests/native-retirement.test.mjs)
and [shared host deadline checks](../packages/cem-elements/src/lib/internal/runtime-support/checkpoint-domain.spec.ts).

## Native application receiver assembly

[`structural_receiver::application`](../packages/cem_ql/src/structural_receiver/application.rs)
adds `stage`, `LinkedApplication` and `ApplicationSlot`. Staging takes a verified
CEMAPPD2 envelope, the actual fresh native host, explicit
`ComponentReceiverSource` records and typed `ContractBinding` requests. The
embedding must establish all source/data scopes, contexts, directed grants and
native registrations first. Staging does not change the host registry/context,
fetch modules, parse source text, execute queries or invoke native callbacks.
No source tree, callback, live token or binding crosses JSON or the wire.

The first assembly subset binds top-level literal named-node directives. Each
compiled attribute name, type alias and local `itemCount`/`minItems`/`maxItems`
field must match its selected original native receiver occurrence. The native
source uses literal `node`/`schema:node`; its selected SC1 capsule supplies the
checked structural contract. It must be the actual tree registered on this host,
under its original schema/attribute parent, with no extra receiver attributes.
Relabelled or reconstructed copies, changed cardinality, duplicate/nested
receiver directives, dynamic fields, defaults and unsupported policies refuse.
Multiple fields and components keep their own original occurrence owners and
merge checked contracts into complete per-component receiver models.

Every QD1 and NR1 dependency needs exactly one matching link request; missing,
extra or wrong-role requests refuse. Structural capsules remain inert until
selected by a receiver. Query/validator requests supply exact source scopes,
datatype occurrences, function selections and result adapters. Tokenizer,
operation and source-result requests call their existing shared native linkers.
Actual implementation identities/registrations and current source scopes are
checked there. The assembly retains their liveness guards and exposes checked
receipts. Operation/tokenizer receipts still require explicit installation on
the exact original datatype through the ordinary compiler; linking is not
complete executable datatype construction or native code dependency proof.

`ApplicationSlot::publish` rechecks the candidate's actual context and retained
node/query/native guards immediately before one synchronous native slot replacement.
A failed stage or publication preserves the prior slot; partially staged owners
are disposed. Replacing/clearing a slot or abandoning a staged assembly retires
its models and guards, including cloned models held by another consumer. Closing
or changing the host context, cancellation and retired node/query/native bindings
make checked access unavailable. Host lifetime is separate: the assembly keeps
a weak context lease and cannot keep native code or an old host alive.
This slot is not a component registry, physical-generation transaction, rendering
commit, optional-module activation or recovery/publication permission.

Assembly retains the envelope's 32-component/64-contract limits and adds at most
64 receiver occurrences per component, 256 total, 16,384 nodes per receiving
source tree and 65,536 nodes across distinct receiving tree owners overall.
Reusing the same retained tree counts once for this in-process bound, without
acquiring authority from pointer identity. Compiled receiver inspection keeps 100,000-node/64-depth limits per
component and the same operation control. Existing nested linker/validator
budgets remain independent; this is not a measured peak heap or a raised recovery
budget. [Assembly fixtures](../packages/cem_ql/tests/application_envelope/assembly.rs)
and [boundary fixtures](../packages/cem_ql/src/structural_receiver/application/occurrences/tests.rs)
cover native validation, source/occurrence refusal, all-or-nothing replacement,
stale/foreign/cancelled candidates, dependency registration and cleanup.

CEMAPPD2 still carries structural named receivers only. A scalar/executable
receiver substitution refuses rather than clearing pending state or widening a
node contract. Portable scalar/executable receiver composition and exact complete
type dependency closure need their own versioned contracts before admission.
Deployed CEMAPPD1, WASM/worker/fallback endpoints and runtime component admission
are unchanged. Entered credentials still block recovery; automatic whole-Studio
recovery remains disabled, with 4 MiB checkpoints and five-second deadlines.

## Native datatype source closure and compilation

[`cem_ql::datatype_source_capsule`](../packages/cem_ql/src/datatype_source_capsule.rs)
implements native-only `CEMDT1` / `cem.native-datatype-sources/1`.
`export`/`verify` preserve the complete original `NativeSourceSnapshot`, trusted
source/registry pins, explicitly selected root type occurrences and their exact
local literal dependency graph. The graph is recomputed through the shared
`DatatypeSource::plan`: inherited bases, list items and inherited lists retain
original attribute occurrences and targets. The verifier never manufactures
reference nodes, parses source text, evaluates queries or registers code.
Reload creates new owners; source URIs, native IDs and hashes alone supply no
runtime scope or implementation authority.

This first profile accepts one original schema's unqualified local type names
and scalar, lexical, list or inherited source kinds. Missing/cyclic/duplicate
names, qualified/imported dependencies, native references, type-child constants,
inline query bodies and unsupported kinds refuse. All original source records
remain intact; unselected type declarations are passive source data, not exports
or implicitly registered implementations. Unknown semantic fields cannot become
ready through transport: the shared compiler still owns field/facet admission,
kind compatibility, effective restrictions and actual implementation checks.
`executable_closure_verified()` always returns `false`. Literal source closure
is neither complete behavior/executable closure nor linked native code proof.

[`FreshStructuralHost::compile_datatype_sources`](../packages/cem_ql/src/structural_receiver/datatypes.rs)
requires the actual source tree and a ready scope on that concrete fresh host,
matching registry identity, current operation control and explicit native
`DatatypeImplementations`/`DatatypeValidationRegistry` inputs. An isolated
compiler scope preserves the actual scope's traversal policy and binds only the
verified same-source literal targets. It uses ordinary `compile_datatypes` and
changes no registry/context on the caller's host. Missing registrations,
unsupported fields, unavailable validation, incomplete compilation, cancellation
or foreign/pending scopes refuse; prior compiled owners remain usable.
Compilation does not prepare lexical input or evaluate source queries.

The resulting `FreshDatatypeCompilation` keeps executable snapshots private.
`validate` accepts only explicitly exported roots and delegates to the ordinary
native scalar/list validator, retaining inherited cardinality, item checks,
stops and original source attribution. Dependency types do not become implicit
entrypoints. Actual host/source/context and control are checked before and after
validation, so a stopped or stale invocation cannot return a completed result.
Retiring the compilation or closing/changing its host refuses subsequent use.
The wrapper exposes inert source metadata and the guarded preparation API below,
not cloned executable types, raw prepared tokens or a portable permission receipt.

`share_native_source` permits explicit NR1 operation installation on the same
original source owner only after source/registry identities and the **complete**
verified snapshot agree. A different arena claiming the same source hash refuses.
Existing operation linkers/installers retain exact declaration, registration and
lifetime checks; copying or relabelling a datatype cannot acquire the binding.
Typed validation does not imply lexical preparation or conversion readiness.

The format is capped at 64 KiB, 32 exported roots, 64 local type declarations
(including unselected declarations) and dependency depth 32. Native source arenas
retain their 16,384-node bound; outer MessagePack preflight allows 16,384 records
and 64 nesting levels, with canonical encoding and 100,000 verification checks.
The wire limit can require fewer types; source arenas are never pruned and no
budget is raised to make them fit. Compiler/validation work additionally uses
the actual host policy and existing shared limits, not a reset or enlarged
verification allowance. These are admission bounds, not measured peak heap.
[Integration fixtures](../packages/cem_ql/tests/datatype_source_capsule.rs) and
[wire/boundary fixtures](../packages/cem_ql/src/datatype_source_capsule/tests.rs)
cover reload, roots, native compilation, source sharing, ownership, control,
repinned incompatibilities and limits.

Guarded root preparation is implemented below. Remaining work includes complete
behavior/query/imported-source dependency contracts, a new application receiver
format for mixed node/scalar/executable bindings, and atomic receiver installation
with actual worker/fallback generation and checkpoint ownership. CEMAPPD1 and
CEMAPPD2 are unchanged; existing runtimes do not admit CEMDT1 or scalar/executable
receiver substitutions. No WASM endpoint or component/application JavaScript was
added. Entered credentials still block recovery; automatic whole-Studio recovery
remains disabled, with 4 MiB checkpoints and five-second deadlines unchanged.

## Inert datatype dependency assertions

[`datatype_dependency_capsule`](../packages/cem_ql/src/datatype_dependency_capsule.rs)
adds native-only `CEMDP1` / `cem.native-datatype-dependencies/1`, separate from
CEMDT1 and existing application formats. `export` accepts the original capture,
source/registry identity, root occurrences, a field-to-target map and explicit
`DependencyArtifact` inputs. `verify` requires the independently pinned bytes and
the exact supplied artifact set. It does not fetch modules, parse query source,
register implementations, grant access or compile executable datatypes.

The shared datatype source plan determines every reachable base, whole-list,
list-item and native validation-rule dependency. The capsule retains each original
attribute occurrence and, for native values, its original reference occurrence,
expression, lexical containing-node index and source/namespace snapshots. Targets
are either local datatype occurrences, explicitly exported CEMDT1 roots, original
QD1 validator declarations or NR1 validator requirements. Exact nested pins and
registry identities must agree with independently verified artifacts. Missing,
unused or duplicate artifacts, wrong dependency roles/artifact categories and
missing/extra field bindings refuse.
Literal local names must match their selected declaration; imported literal forms
must carry a qualified name with the expected target local name. Literal rules
remain descriptive and acquire no executable target.

**Intended targets are trusted build assertions.** Inert verification cannot prove
that a retained native reference or imported prefix actually selects its asserted
target. Original namespace snapshots are retained, without replacing them with a
consumer's aliases. `selections_verified()` and `executable_closure_verified()`
both remain false, including for a correctly pinned package. The next linker must
resolve the original field through actual fresh source scopes, source policies
and directed crossing grants, compare the exact original selected target and
retain readiness/ownership guards before compilation. This format grants no
permission to reuse an independently allocated source or publish a receiver.

The new [dependency source snapshot](../packages/cem_ml/src/schema/native_source/dependencies.rs)
is versioned separately and shares the existing source graph, span and lexical
metadata validation. It permits unevaluated reference records and a single native
reference in an attribute value. A reference's context must be its actual lexical
container, including the containing element for an attribute value. Retained
runtime target lists, missing lexical sidecars, foreign spans, invalid owning
edges and unknown versions refuse. Reload creates a fresh owning allocation and
restores the original occurrence/namespace sidecars without evaluation. The
literal `NativeSourceSnapshot` profile and CEMDT1 still reject references.

All modules claiming one source URI must agree on the **complete** source identity
and dependency snapshot, including lexical metadata; matching IDs or hashes alone
are insufficient. Distinct module roots over agreeing complete sources are
allowed, but verification does not merge their owners. Local dependency cycles
refuse, with a deterministic local dependency order. Cross-module resolution and
cycles involving future dependency-capsule modules remain a linking gate.

Bounds remain explicit: 64 KiB capsule bytes, 32 roots, 64 source type declarations,
128 field bindings and local graph depth 32; at most 64 independently verified
artifacts and 4 MiB capsule-plus-artifact bytes. Each nested format retains its
own 64 KiB and operation limits. The outer verifier has 100,000 checks, sharing
cancellation with nested checks rather than claiming a single aggregate work
allowance or measured peak heap. Strict binary preflight precedes deserialization;
canonical encoding, versions, outer pins and original source metadata are checked.

This bounded contract supports scalar/lexical/list and inherited declarations.
Constants, other datatype kinds, authored ID tables, unresolved namespace/typed
prelude metadata, non-reference native attribute values and additional NR1
operation dependencies remain outside this format. Inline query text retained in
a source is passive; only a separately verified QD1 pin describes a supported
validator program, and even that program is not registered by verification.
Mixed receiver formats, native code closure, application/WASM admission and
worker/fallback checkpoint enrollment remain pending. Existing credential refusal,
4 MiB checkpoints, five-second deadlines and disabled automatic whole-Studio
recovery are unchanged.

## Fresh-host datatype dependency selection

The native [selection adapter](../packages/cem_ql/src/structural_receiver/dependencies.rs)
checks CEMDP1's intended dependencies using the **actual**
`FreshStructuralHost`. It borrows the existing resolver, source contexts, policies
and directed grants; it neither clones the host nor installs a persistent name
catalog. Original datatype name discovery and dependency traversal remain the
shared implementations. No target is selected directly from the capsule's assertion.

`dependency_ids`, `dependency_source`, `dependency_capture` and `roots` expose
read-only verified source inputs. First call `register_dependency_scope` for the
root and each supplied module, passing each actual runtime context/readiness and
reference policy explicitly. Registration attaches immutable captured names and
namespace metadata to the exact original arena. It creates no crossing grant.
Generic tree registration is insufficient: preparation and selection require the
exact verified capture to be attached already, so discovery cannot admit it
implicitly or change an unchanged prior context's metadata.
`prepare_dependency_lexical_scopes` then passes each saved original occurrence and
snapshot to an explicit context/policy preparation callback. Missing contexts
stay pending; a ready root context cannot stand in for an unprepared occurrence.
Preparation failures that mutate nothing preserve prior selection results.

`DatatypeDependencyImport` explicitly names the consuming registered scope, a
verified DT1 module and one of its exported roots. The exported namespace/name
come from that target's original schema/declaration fields, without consumer
aliases, renaming or a built-in fallback. Ordinary discovery resolves the original
namespace snapshots and `use` declarations. Unexported names, conflicting aliases,
non-exported roots and duplicate/unknown import requests refuse. Import requests
create no permission: an actual directed relationship grant is still required
for both literal and native crossings. A reverse grant is insufficient.

Call `select_datatype_dependencies` with the root scope, the exact module-to-scope
map, explicit imports and bounded traversal limits. Every given scope must own
the exact verified source tree and be the schema's actual registered source scope;
its context must be ready. Matching URIs, source hashes, node numbers or foreign
host indices cannot substitute another allocation. Each reached original field
must select exactly the asserted original target. Imported local inheritance is
looked up in the vendor's own environment. Native references run only in explicitly
prepared original occurrence contexts; their existing selector compilation and
evaluation use the actual host's capabilities and controls. Query/native validator
behavior targets remain terminal metadata and their implementations do not run.
Unknown datatype sources are not traversed as if they were verified declarations.

The ordinary resolver enforces singleton targets, reference cycles, request and
per-destination traversal bounds. Dependency walks share the remaining request
work across capsule roots; destination policies apply to each ordinary walk.
Name discovery is a separately bounded phase, with existing name-byte limits.
Requests refuse zero bounds, depth over 64, work over 100,000 or more than 128
import declarations. These bounds do not claim one aggregate phase/heap budget
or future cross-module code closure. Cancellation or a callback-time stop cannot
return a completed selection. Original reference target lists remain untouched.

`FreshDatatypeDependencySelection` contains inspection counts and immutable capsule
data, with a weak lease on the actual host context. `is_live`, `site_count` and
`datatype_count` check that host/context and operation; `retire` closes the result.
Changing a context, adding scopes/grants, successfully preparing occurrences,
closing/dropping the host or using a foreign host invalidates use. Retaining the
result does not retain host callbacks, and inspecting it never replays selectors.
Failed selection attempts publish nothing and leave an unchanged prior context live.

**This is not a retained permission to compile or publish.** Context liveness does
not freeze mutable native callbacks or provider data. A new selection attempt
reevaluates selectors. The primitive compiler below performs selection and
compilation in its checked invocation rather than trust a historical result.
Portable `selections_verified()` and `executable_closure_verified()` remain false.
No datatype/validator implementation is compiled or registered by this adapter,
and no receiver is installed. Wider operation/constant closure, executable
preparation/tokenizer hooks and worker/fallback
transport/checkpoint enrollment remain pending. Existing application formats,
WASM endpoints, credential refusal, disabled automatic whole-Studio recovery,
4 MiB checkpoints and five-second deadlines are unchanged.

The [native selection fixtures](../packages/cem_ql/tests/datatype_dependency_capsule/selection.rs)
cover original aliases/contexts, vendor inheritance, exact QD1/NR1 targets,
missing/reverse grants and exports, pending inputs, wrong/empty/multiple/cyclic
selections, source/host identity, budgets, cancellation, stale results and weak
callback lifetime release.

## Fresh-host primitive dependency compilation

The native [primitive compiler](../packages/cem_ql/src/structural_receiver/dependencies/compilation.rs)
adds `FreshStructuralHost::compile_datatype_dependencies` for typed scalar/lexical
and list roots with original cross-source inheritance. Supply the verified CEMDP1
capsule, actual registered root/module scopes, explicit imports, primitive metadata
and bounded traversal limits. It reuses the selection adapter's original source
capture, name discovery, readiness and directed-grant checks, then runs the shared
native datatype compiler. A historical selection inspection is not an input.

The shared native traversal checks an explicit host admission hook against each
original field and asserted singleton target before executable construction.
Selectors run again during compilation: a changed callback selecting a different,
otherwise valid datatype refuses. Actual operation/context checks guard the whole
invocation, including callback-time cancellation. Name discovery and compilation
have separately bounded phases; this is not an aggregate heap/phase budget.

`dependency_datatypes` exposes verified DT1 source metadata for explicit primitive
implementation entries. Registrations must own the exact original declaration
and scope, use scalar/lexical/list kinds and scalar/list representations, have no
validator and use `TokenizerBinding::Absent`. Foreign source copies, node/grammar/
reference implementations and executable tokenizer hooks refuse. Preparation,
conversion, equality, constant interpretation and additional facet-profile
registrations cannot be supplied through this API. Existing full/DT1 APIs retain
those capabilities.

The primitive-only `compile_datatype_dependencies` convenience path still refuses
CEMDP1 validation-rule sites with `datatype-dependency-rule-bindings-unavailable`.
Correct metadata selection does not bind a rule implementation or execution
context. Use the explicit rule-owner path below to enable checked imported rules.
Unsupported primitive metadata returns
`datatype-dependency-primitive-metadata-required`. Compilation failures expose
original diagnostics/reference issues and source-owned compiler issues; partial
executable contracts never escape in the error.

`FreshDependencyDatatypeCompilation` keeps executable descriptors private.
`validate` accepts only exported capsule root IDs and reruns ordinary typed
validation with caller limits under the actual live host operation/context. An
internal imported datatype is not a validation entry point. Closing/dropping the
host, changing its contexts/grants or retiring the compilation invalidates use;
foreign hosts refuse. The result retains source snapshots and a weak context lease,
not host callbacks. Failed compilation preserves an unchanged prior result.
Validation neither replays selectors nor caches a verdict. Lexical preparation,
sealed handoff and mixed receiver publication are not exposed by this adapter.

The [compilation fixtures](../packages/cem_ql/tests/datatype_dependency_capsule/compilation.rs)
cover imported inheritance/facets and diagnostics, root bounds, original ownership,
missing/foreign metadata, executable-hook and QD1/NR1 refusals, renewed mutable
selection, request/input budgets, cancellation, stale/foreign/retired/closed contexts
and callback release. The shared [native traversal fixture](../packages/cem_ml/tests/datatype_dependency_traversal.rs)
checks explicit admission refusal with the original field retained.

Portable selection/executable closure flags stay false. This adds no wire format,
WASM endpoint, receiver admission or application JavaScript. Credential refusal,
disabled automatic whole-Studio recovery, 4 MiB checkpoints and five-second
deadlines remain unchanged.

## Explicit imported rule owners

The native [rule-owner adapter](../packages/cem_ql/src/structural_receiver/dependencies/rules.rs)
adds `link_datatype_dependency_rule` and
`compile_datatype_dependencies_with_rules`. This uses the original CEMDP1/QD1/NR1
sources and shared compiler/validator; it does not introduce a new source format,
JSON AST handoff, WASM entry point or receiver publication.

Register all exact source scopes, occurrence contexts and directed grants first.
Install any native validator callback on the actual `FreshStructuralHost` before
creating bindings: native implementation registration advances the context epoch.
Then explicitly link each required module with its actual registered scope and a
`DatatypeDependencyRuleRegistration`. `Query` supplies the selected built-in
function registry and result adapter; `NativeValidator` supplies a result adapter
and requires the exact preinstalled NR1 implementation identity, including module
hash, export, ABI and signature. A pin alone installs no implementation. Linking
creates no crossing grant and invokes no validator.

Keep each returned `FreshDatatypeRuleBinding` in a caller-owned `Arc` and pass a
map from verified module ID to owner into
`compile_datatype_dependencies_with_rules`, alongside the existing primitive
metadata. The map must contain exactly the imported rule modules: missing,
unused, wrong-kind, wrong-scope, copied-source and foreign-host bindings refuse.
Equal source hashes and matching module IDs cannot replace an original arena,
even when both source allocations are registered on the same host. Compilation
renews ordinary source selection and crossing checks inside its invocation;
failed attempts preserve unchanged prior bindings and compilations.

The shared compiler binds each rule to the original datatype where it is
attached. An inherited base rule retains that base datatype, while a derived
local rule receives the derived datatype. Imported behavior declarations keep
their separate original owners. Signature mismatches and result diagnostics
retain original source provenance. No partial executable descriptors escape
compilation errors.

At validation time, the shared validator obtains each rule's original source
context from the actual host, using its existing closed-validator accessor. It
preserves that source's query scope, policy and diagnostics; ambient callbacks,
module resolvers and unrelated data bindings are excluded by that accessor.
The contexts are ephemeral and never stored in descriptors or binding owners.
All rules, inherited rules and scheduled list items share one enclosing query
budget. Ordinary rule/input/diagnostic bounds and operation controls still apply;
an independent validation call gets a fresh allowance and reruns the rules.
Native callbacks remain cooperative users of the supplied control/runtime.
Reentrant or concurrent validation of the same compilation refuses before
replaying a native callback or starting another allowance. The entry guard
releases on completion or failure.

Compilations retain weak rule owners. Retiring/dropping a binding, removing its
native implementation, changing the actual host context, or closing/dropping the
host disables dependent compilations. Native proxies check their weak host-owned
implementation and binding liveness before and after callbacks. Source-context
availability is also checked around each rule execution and the completed
validation; callback-time retirement or cancellation cannot publish success.
Keeping snapshots or rule bindings after dropping the host retains no native
callback or source runtime context. Inspection remains liveness, not portable
permission or a cached acceptance verdict.

The [imported-rule fixtures](../packages/cem_ql/tests/datatype_dependency_capsule/rules.rs)
cover fresh query/native outcomes, combined inheritance, original datatype and
diagnostic ownership, exact registrations and same-host copied arenas, missing
grants, source-specific policies, shared inherited/item work, rule/diagnostic
limits, dropped/retired/stale owners, removed implementations, callback-time stops,
weak callback release and reentry refusal.

Portable selection and executable-closure flags remain false. Additional
operation/constant profiles, executable preparation hooks, mixed receivers and
worker/fallback transport/recovery remain separate
checklist gates. Entered credentials still block recovery; automatic whole-Studio
recovery stays disabled, with 4 MiB checkpoints and five-second deadlines unchanged.

## Dependency-bearing datatype module graphs

The native-only [module exporter/verifier](../packages/cem_ql/src/datatype_module_capsule.rs)
introduces `CEMDM1` / `cem.native-datatype-modules/1`. This is a separate flat
source-backed format; CEMDP1 remains unchanged. Stable module IDs describe imports
inside one pinned graph, so cycle rejection needs neither recursive capsule
loading nor circular content hashes. This does not activate a module, resolve a
native selector, register an implementation or admit a receiver.

Build a map of module ID to `ModuleSource`, retaining its original
`LexicallyScopedDocument`, `SourceIdentity`, exported root IDs and complete
original dependency-field bindings. Pass a root module ID, registry hash and
explicit DT1/QD1/NR1 leaf artifacts to `export`. Module IDs start with an ASCII
letter or underscore, contain only ASCII letters/digits/underscore/hyphen/dot,
and are at most 128 bytes. They are local graph keys, not URLs or global type names.
Every source URI remains the original resolution base in its retained tree.

`ModuleTarget` distinguishes local datatypes, exported datatypes from another
module, and pinned DT1/query/native leaves. Original base/list/rule fields are
classified by the same [datatype inspector](../packages/cem_ql/src/datatype_dependency_capsule/graph.rs)
as CEMDP1, using the shared native source plan. Local names and qualified imported
literal names must agree with their asserted targets. Native references and their
complete lexical/namespace snapshots remain passive source occurrences. This is
not a name-resolution or selection verdict: the explicit fresh-host consumer
below checks actual source contexts, original aliases, explicit exports and
directed grants again.

The [shared leaf verifier](../packages/cem_ql/src/datatype_dependency_capsule/leaves.rs)
checks exact DT1/QD1/NR1 pins, registry identities and existing leaf format limits.
Missing, duplicate and unused artifacts refuse. A rule pin cannot satisfy a base
field. No nested CEMDM1 or dependency-bearing CEMDP1 blob is accepted as a leaf.
Across every module and leaf, repeated source URIs must agree on the complete
source identity and original snapshot, including regions outside the exported
roots. Identical complete sources may expose different roots under separate
module IDs; their original arenas are not merged. A shared module ID in a diamond
is restored once per verification. A later verification allocates fresh owners.

`verify` returns `VerifiedDatatypeModules` with read-only module views, original
source captures, exported roots and attributed dependency sites. `module_order`
is a deterministic dependencies-first order of module IDs. `datatype_order`
orders `(module, datatype)` identities, including local DT1 leaf dependencies
identified by their leaf hash. Unreachable modules, missing exports, unknown
fields/targets and incorrect field roles refuse. Every declared module export is
checked, including exports an eventual application may not consume.

Both module imports and datatype dependencies must be acyclic. Module cycles
return `datatype-module-cycle`, a closed `cycle_modules` path and original
attribute nodes in `related`. The [ordering implementation](../packages/cem_ql/src/datatype_module_capsule/order.rs)
uses a bounded iterative dependencies-first walk; its witness follows actual
remaining cycle edges rather than treating every blocked consumer as a cycle.
Local datatype cycles are rejected by the shared source inspector, with original
related fields. Cancelled or exhausted ordering work refuses without reporting a
spurious cycle. Depth counts across module boundaries and through DT1 leaves;
it does not restart for each source.

| Bound | Limit |
| --- | --- |
| Module records | 1–32 |
| Encoded original source record | 64 KiB per module |
| Exported roots | 1–32 per module |
| Datatype catalog | 64 per source, using the shared inspector |
| Dependency fields | 128 per module; 512 across the graph |
| Graph bytes plus supplied leaves | 4 MiB total |
| Supplied leaf artifacts | 64, with each existing verifier's own bounds |
| Shared outer source/graph work | 100,000 checks |
| MessagePack preflight | 100,000 values; depth 64 |
| Combined datatype dependency depth | 32 edges |

The most restrictive bound applies. Outer work is shared across source restoration,
classification, source agreement and graph ordering. Nested leaf verifiers retain
their existing separate bounded allowances; this is not a single aggregate heap
or decoder work reservation. Canonical MessagePack, exact versions, complete
source agreement and preflight checks reject repinned malformed, unknown,
trailing, deep and oversized records before returning verified metadata. These
bundle byte limits are independent of the Studio recovery checkpoint budget.

The [module fixtures](../packages/cem_ql/tests/datatype_module_capsule.rs) cover
chains, diamonds, local/self/cross-module cycles, original field witnesses,
source/alias preservation, exports, repeated source agreement, inert query/native
pins, malformed DT1 leaves, combined depth and cancellation. The
[wire fixtures](../packages/cem_ql/src/datatype_module_capsule/tests.rs) cover
repinned metadata/version/bounds changes and malformed binary inputs. Legacy
and module tests share the same checked rule-artifact builders.

`selections_verified()` and `executable_closure_verified()` remain false.
`VerifiedDatatypeModules` is a distinct type; existing fresh-host CEMDP1 APIs do
not accept it implicitly. The explicit actual-host module entry points below now
supply scoped selection/compilation. Additional operation/constant profiles,
mixed receivers and WASM/worker/fallback
transport remain pending. No application JavaScript or new WASM entry point is
introduced. Entered credentials still block recovery; automatic whole-Studio
recovery stays disabled, with 4 MiB checkpoints and five-second deadlines unchanged.

## Actual-host datatype module binding

The native [module entry points](../packages/cem_ql/src/structural_receiver/dependencies/modules.rs)
consume CEMDM1 through a borrowed verified-input view. This is not conversion into
a CEMDP1 capsule, a replacement host, a serialized executable or automatic module
activation. Both formats use the same actual `FreshStructuralHost`, original name
catalog, ordinary reference traversal, primitive compiler and rule-owner guards.

Use `register_datatype_module_scope` for the graph's root module, other module IDs
and every supplied leaf hash. Provide each actual context/readiness and reference
policy explicitly. Registration attaches exact original captured names and
namespaces but creates no crossing grant. Raw tree registration or a reloaded
copy with equal hashes cannot substitute for the verified capture. The
`leaf_source`, `leaf_capture` and `leaf_datatypes` metadata accessors expose original
leaf inputs for explicit preparation and primitive registrations.

Call `prepare_datatype_module_lexical_scopes` with each source's original occurrence
contexts/policy overrides. Missing contexts stay pending, even with a ready root
context. Supply directed grants at every actual declaring-module hop: an app's
grant to a grandchild does not replace its intermediate module's grant. Literal
qualified references also require `DatatypeDependencyImport` requests on the
original consuming source scope. Imports must name actual exported roots and use
their original namespace/name; they cannot repair an author's wrong alias or
manufacture a grant. A shared module in a diamond keeps one source owner and its
own scope.

`select_datatype_modules` takes the graph, the actual root declaration scope, a map
of every other source ID to scope, explicit imports and traversal limits. The root
scope must own the exact source named by `graph.pin().root`. Missing, extra,
foreign, duplicate or copied scope mappings refuse. This returns the same local
`FreshDatatypeDependencySelection` inspection API; liveness/counts are not retained
permission to compile or publish.

`compile_datatype_modules` takes those inputs plus explicit primitive metadata and
a map of reachable leaf IDs to caller-held `Arc<FreshDatatypeRuleBinding>` owners.
Create these owners with `link_datatype_module_rule`, using the same explicit
query-function selection/result adapter or exact preinstalled native implementation
as the CEMDP1 path. Register native implementations and prepare scopes/grants
before linking owners, since these actions advance the host context epoch.
The primitive-only kinds/representations and no tokenizer/validator metadata-hook
rules remain enforced. Additional converter/preparation/constant profiles are not
implicitly enabled.

A bounded [input planner](../packages/cem_ql/src/structural_receiver/dependencies/inputs.rs)
indexes intended original fields and targets and derives the root's reachable
requirements. It does not execute or authorize selectors. A transitive rule reached
through imported inheritance or list items requires its exact live owner; an unused
export's rule requires neither an owner nor a prepared rule occurrence. Source
metadata still needs explicit registered ready scopes for discovery. The binding
map must match the reachable rules exactly, so extra owners do not activate code.

Compilation independently reruns ordinary selectors under actual original contexts
and policies. Shared native admission compares every completed singleton selection
with its intended original field/target before executable construction. A mutated
callback selecting another otherwise valid source, a dynamic reference cycle or
callback-time cancellation refuses. Every reachable intended field must appear in
the completed selection. An earlier successful inspection cannot authorize a later
changed target. Unchanged private compilations survive failed attempts that change
no context.

Only the graph root's exported datatype IDs are validation entry points. Original
base/item declarations and imported rule behaviors keep their original arenas,
including cardinality and validation diagnostic provenance. The existing private
compilation wrapper reruns typed validation with actual rule-source contexts,
shared rule/input/query/diagnostic limits, weak rule ownership and reentry guards.
Validation does not replay dependency selectors or cache an acceptance verdict.
Retired/dropped rule owners, removed native implementations, changed/stale/foreign
contexts or a closed/dropped host disable use. Retaining graph inspections or
compilations retains no host selector/validator callback or source runtime context.

Requests still refuse zero work/depth, depth above 64, work above 100,000, or more
than 128 import requests. Metadata planning, name discovery and ordinary traversal
are separately bounded phases under actual operation controls. Traversal shares its
remaining work across root exports and preserves destination policies. This does
not claim a single cumulative heap/phase reservation. CEMDM1's wire/source/graph
limits remain unchanged.

The [actual-host fixtures](../packages/cem_ql/tests/datatype_module_runtime.rs)
cover chains, shared diamonds, original lists/facets, per-hop grants, source and
occurrence readiness, raw/copy/foreign/extra scope refusals, qualified imports,
request/destination limits, fresh mutable selectors, dynamic cycles, exact DT1
arenas, transitive query/native owners, unused exports, shared item budgets,
callback-time retirement/stops/reentry and weak callback release. Wire tests share
the same original source builders.

Portable selection/code-closure flags stay false. There is no new wire semantics,
WASM entry point, receiver publication or application JavaScript. Broader operation/
constant module integration, mixed receivers and runtime transport/recovery remain separate
gates. Entered credentials still block recovery; automatic whole-Studio recovery
stays disabled, with 4 MiB checkpoints and five-second deadlines unchanged.

## Versioned operation and constant dependency profiles

[`native_registrations::profiles`](../packages/cem_ql/src/native_registrations/profiles.rs)
exports and verifies `CEMOP1` / `cem.native-operation-profiles/1`. This separately
versioned native format does not extend CEMDM1, CEMDP1, CEMNR1 or component runtime
admission. A profile names one original source/scope/datatype occurrence and an
intended effective kind and representation. Stable profile IDs group exact
CEMNR1 pins for tokenizer, lexical preparation, conversion, equality, constant
interpretation, facet profile, list-item preparation and list serialization.
These are intended dependencies, not a claim that every required capability is
present or that inheritance has been resolved.

Every selected CEMNR1 record is independently verified through the existing
source/ABI/signature/result-contract verifier. The profile then checks role,
original datatype occurrence, registry, kind and representation compatibility.
Direct and source-backed conversion/equality/constant profiles retain their
distinct exact signatures, original behavior occurrences and checked result
contracts. No result adapter, callback, interpreted constant, context or runtime
policy appears on the wire. Validator requirements do not satisfy operation roles.

Each datatype occurs once and each dependency pin is used once. Missing, extra,
duplicate and mismatched artifacts refuse. Complete source identities and
canonical snapshot fingerprints must agree for each logical URI before existing
`share_source` explicitly remaps metadata occurrences into one verified original
arena. Equal URI, source hash or declaration names alone cannot authorize this
mapping. Independently verifying the same profile set allocates fresh owners.
Literal `@values` and original constant children keep their original source
occurrences; verification never tokenizes or interprets them. Native-reference
constant slots remain refused by CEMNR1's closed source profile. Supporting those
edges requires the separately versioned module integration, rather than clearing
slots or changing CEMNR1/CEMDM1 semantics.

The canonical named MessagePack manifest is limited to 64 KiB, 64 profiles,
eight operation roles per profile and 64 supplied CEMNR1 artifacts. Manifest plus
supplied artifact bytes cannot exceed 4 MiB. Each leaf keeps its existing 64 KiB,
source, decode and work limits. The outer phase uses a 100,000-visit allowance,
100,000-value / 64-depth MessagePack preflight and actual operation controls;
nested CEMNR1 verification and occurrence remapping retain their separately
bounded phases. This does not claim one cumulative heap or nested-work budget.
Unsupported magic/ABI/compiler versions, unknown fields/roles, noncanonical,
trailing, oversized or cancelled input refuses before a verified result returns.

`VerifiedOperationProfiles::executable_closure_verified()` remains false.
Callers may explicitly feed original requirements to the existing fresh-host
linkers. Actual registrations, checked source scopes, exact original arenas,
source-result adapters and live weak ownership remain required. Retiring an
operation invalidates its use; retaining a profile or compiled proxy retains no
host code. This format does not integrate operations into module compilation,
prove complete code closure, publish receivers, or add WASM/worker endpoints.

The [native fixtures](../packages/cem_ql/tests/native_registrations/dependency_profiles.rs)
reuse existing operation/source-result builders and cover all roles, constant
provenance, source/role/representation conflicts, equal declared identities with
different snapshots, fresh-owner isolation, live linking/retirement, weak callback
release, malformed repinned records, limits and cancellation. No application or
component JavaScript was added. Entered credentials still block recovery;
automatic whole-Studio recovery remains disabled, with the 4 MiB checkpoint
budget and five-second handoff deadlines unchanged.

## Dependency-aware operation sources and constant-reference slots

[`native_registrations::dependency_sources`](../packages/cem_ql/src/native_registrations/dependency_sources.rs)
exports and verifies the separate native `CEMOS1` /
`cem.native-operation-source/1` format. Each record retains one operation's exact
implementation identity, mode, original schema/type/behavior occurrences and
complete `NativeDependencySourceSnapshot`. The shared snapshot preserves
unevaluated native references, their original owning contexts, source frames and
lexical namespace metadata. It refuses resolved targets, forged contexts, partial
lexical snapshots and unknown source-node fields; it never exports live scopes
or runtime authority.

Shared CEMNR1 routines check original membership, implementation ABI/signature,
ready versus checked-replacement mode and source-backed result contracts. The
shared datatype plan provides original inherited-base, list-item, whole-list and
validation-rule fields. `constant_slots()` exposes the selected datatype's
original nontrivia child slots, including native references, in source order.
`references()` inventories every original reference in the complete source,
including other declarations. These views resolve nothing. In particular, a
retained slot is not a certified constant: only the existing shared enumeration
compiler may select targets, validate literal constant declarations and interpret
values under actual source policies, contexts, grants and runtime limits. There
is no second constant parser, selector evaluator or target reconstruction here.

Independent verifications create fresh source owners. Explicit `share_source`
requires complete source-identity, registry and dependency-snapshot fingerprint
agreement, then restores the checked operation and its original field/slot views
into the chosen verified arena. Matching URI/hash/name alone is insufficient;
metadata mapping creates no operation registration or source crossing grant.

Records are limited to 64 KiB, with 16,384-value / 64-depth MessagePack preflight,
a 100,000-visit phase allowance and actual operation controls. The selected
datatype has at most 128 dependency fields and 128 constant slots; the complete
source has at most 128 references. The shared snapshot retains its existing
source/lexical bounds. Export capture, wire verification and explicit occurrence
mapping are separately bounded phases, not one cumulative heap reservation.
Unknown magic/ABI/compiler/source-profile versions, noncanonical/trailing input,
wrong pins, validator selections, unsupported modes/signatures, oversized or
cancelled work refuse before returning a verified source record.

Both portable selection and executable-closure flags remain false. There is no
`VerifiedNativeRequirement` conversion or CEMOP1 admission path. The explicit
actual-host owner entry points below use CEMOS1 directly. CEMNR1 still refuses
reference-bearing sources, and CEMOP1 still accepts only its exact closed CEMNR1
dependencies. The separately versioned CEMOP2 pins and CEMDM2
source assembly below preserve that boundary; actual-host module/envelope
installation remains a follow-up. Unevaluated reference expressions
are not an evaluated dependency graph or proof against runtime selection cycles.
No application/component JavaScript, WASM endpoint or receiver publication was
added. Entered credentials still block recovery; automatic whole-Studio recovery
remains disabled, with the existing 4 MiB budget and five-second deadlines.

The [native fixtures](../packages/cem_ql/tests/native_registrations/dependency_sources.rs)
cover all direct/source-backed operations, tokenizers and checked replacements;
base/list/rule fields and constant-reference slots; original contexts, frames,
lexical metadata and arena isolation; explicit whole-snapshot sharing and foreign
input refusal; existing-reader refusal; repinned malformed/version/unknown-field,
resolved-target/context/source-profile corruption, bounds and cancellation.

## Explicit CEMOS1 operation owners

The native [dependency-owner entry points](../packages/cem_ql/src/structural_receiver/native/dependency_owners.rs)
on `FreshStructuralHost` consume a sealed `Arc<VerifiedOperationSource>`:

- `link_dependency_operation` binds direct preparation, conversion, equality,
  constant interpretation, facet and list-item preparation requirements.
- `link_dependency_source_operation` also requires the matching explicit
  `NativeResultAdapter` for a source-backed conversion, equality or constant
  result contract.
- `link_dependency_tokenizer` binds ready or checked-replacement tokenization.

The embedding host must first register its exact implementation identity and
actual callback or shared native capability, and register the original retained
source with a ready source context. Linking checks registry identity, original
source allocation/scope and the complete implementation identity, including ABI,
module hash, export and signature. Missing or mismatched implementations, foreign
or copied scopes, pending contexts, wrong operation categories/adapters and stopped
or closed hosts refuse. Linking executes no callback, prepares no expression
occurrence and creates no crossing grant. Dependencies and constant slots stay
unevaluated; a ready source context does not replace their own occurrence contexts.

A private borrowed requirement view shares the existing CEMNR1 entry checks,
operation registrations, tokenizer proxy and checked result adapters. It does not
construct a `VerifiedNativeRequirement`, serialize/reparse a source or substitute
a host. CEMOP2 and CEMDM2 requirements link against their already mapped original
arenas; separately verified equivalent bytes require their own actual source scope.
Existing CEMNR1 APIs and wire admission remain unchanged.

The returned `FreshNativeOperation` / `FreshNativeTokenizer` is an explicit local
owner. Its `install` method stages a selection only for the exact original datatype
in the existing `DatatypeImplementations` registry or `DatatypeImplementation`.
Wrong-source and duplicate operation selections refuse without replacing prior
entries. Ready versus checked-replacement modes are preserved. Source-backed
registrations keep the original behavior and shared typed result admission.
`is_live_on(host)` checks the actual host/context as well as implementation/owner
liveness. Keep the owner alive while using its installed registrations.

Owners and installed proxies retain immutable source metadata, weak host code and
a weak context lease. Context changes, implementation removal, host closure/drop,
operation cancellation or owner retirement/drop disable use. Shared proxies check
availability before and after callbacks, so late cancellation cannot publish a
successful result. Retaining owners or registry entries cannot keep embedding
callbacks alive after host disposal.

This links an implementation requirement; it proves neither dependency selection
nor executable/code closure. The explicit CEMDM2 compilation path below now
renews datatype/constant selection and checks operation-owner binding maps.
Versioned application-envelope/receiver publication remains a follow-up gate. An earlier inspection or successful
owner link cannot authorize those steps. No new WASM/worker endpoint or application
UI behavior is introduced. Credential privacy, disabled automatic whole-Studio
recovery, 4 MiB checkpoints and five-second handoff deadlines are unchanged.

The [owner fixtures](../packages/cem_ql/tests/native_registrations/dependency_owners.rs)
cover all direct/source-backed modes, checked replacements, exact identities,
source/scope/registry mismatches, pending contexts, duplicate/wrong-source installs,
retirement, late tokenizer cancellation and weak callback release. Additional
[CEMOP2 fixtures](../packages/cem_ql/tests/native_registrations/profile_pins.rs) and
[CEMDM2 fixtures](../packages/cem_ql/tests/datatype_assembly_runtime.rs) exercise
mapped original-source linking while portable selection/closure flags remain false.

## Versioned CEMOS1 profile pins

[`native_registrations::profiles::v2`](../packages/cem_ql/src/native_registrations/profiles/v2.rs)
exports and verifies `CEMOP2` / `cem.native-operation-profiles/2`. Its separately
typed profile and leaf pins accept exact CEMOS1 source records. CEMOS1 can describe
every supported operation on closed or dependency-bearing sources, so this format
uses it consistently; raw CEMNR1 leaves are not accepted by relabelling their pins.
CEMOP1 remains the closed CEMNR1 format, with its existing wire shape, ABI, default
public types, limits and admission checks. Changing only magic or pin types cannot
cross the version boundary.

The [shared native profile validator](../packages/cem_ql/src/native_registrations/profiles/shared.rs)
owns both versions' canonical manifest, role/shape/signature, original subject,
exact artifact-set, compatibility, budget and cancellation checks. Public profile
record/view types are shared through concrete aliases: the existing API keeps
CEMNR1 pin inference, including empty literals; v2 selects CEMOS1 types. Generic
implementation parameters add no serialized fields or runtime registration. The frozen CEMOP1 manifest-hash fixture verifies that
factoring these checks did not change the existing format's output.

Each CEMOP2 dependency carries the full CEMOS1 hash, source identity and registry
pin, rather than a name or inferred implementation. Every leaf runs its ordinary
CEMOS1 verification, including exact mode/signature, source-backed result
contracts, native reference contexts and source/slot limits. Before sharing any
occurrences, the shared original-source agreement routine independently recaptures
and fingerprints each leaf's complete dependency snapshot. One logical URI must
have the same complete declared source identity and snapshot across all profiles.
Matching URI/hash/declaration labels cannot conceal changed source or lexical
metadata, even when individual pins are internally consistent.

Only after agreement does CEMOS1's explicit metadata mapping rebind requirements
to one verified original arena per source. Field, constant-slot, reference/context
and original behavior views keep those owners; independent profile verifications
allocate fresh arenas. Caller artifact ordering changes neither manifest bytes
nor source agreement. No selector or constant interpreter is invoked. Missing,
extra, duplicate, mistyped or mismatched leaves and duplicate subjects refuse.
Role, intended kind and representation checks reuse the existing profile rules;
intended shapes do not prove effective inheritance or dependency selections.

CEMOP2 retains the 64 KiB manifest, 64 profiles, eight roles per profile, 64 leaves
and 4 MiB manifest-plus-leaf budget. The outer phase has 100,000 visits and a
100,000-value / 64-depth MessagePack preflight. Independent snapshot recapture
spends the outer allowance and polls the actual operation control; individual
CEMOS1 decoders and occurrence mappings retain their own bounded phases. This
does not claim a cumulative heap or nested-work budget. Unsupported versions,
unknown fields/roles, noncanonical/trailing/oversized data, corrupt or repinned
invalid leaves and cancellation refuse before any verified result is returned.

Both selection and executable-closure flags remain false. Returned requirements
are `VerifiedOperationSource`, with no conversion into CEMNR1 registrations or
existing fresh-host operation links. The separate CEMDM2 assembly below maps
these profiles to modules; actual-host installation and application-envelope/
receiver transport remain separate gates.
No application/component JavaScript or WASM endpoint was added. Entered
credentials still block recovery; automatic whole-Studio recovery remains
disabled, with the same 4 MiB checkpoint budget and five-second deadlines.

The [new profile fixtures](../packages/cem_ql/tests/native_registrations/profile_pins.rs)
cover all roles, source-backed and checked-replacement modes, original sources
and slots, ordering and fresh ownership, complete-source conflicts, old/new
reader refusal, exact artifact sets, bounds, cancellation and correctly repinned
malformed manifests/leaves. The [existing profile fixtures](../packages/cem_ql/tests/native_registrations/dependency_profiles.rs)
retain direct native linking/retirement coverage and the fixed v1 wire hash.

## Versioned module and profile assembly

[`datatype_module_capsule::v2`](../packages/cem_ql/src/datatype_module_capsule/v2.rs)
exports and verifies `CEMDM2` / `cem.native-datatype-modules/2`. Each flat module
retains the existing original-source/type export and dependency fields, plus
explicit module imports, constant exports/reference bindings and selected CEMOP2
profile pins/IDs for its reachable datatypes. Separate supplied profile artifacts
carry exact CEMOP2 bytes and their CEMOS1 leaves; the existing DT1/QD1/NR1 leaf
families remain available through the shared datatype dependency verifier.
CEMDM1 keeps its wire shape and constant-slot refusal. A fixed v1 manifest-hash
fixture protects its existing output, and neither reader accepts the other format.

Shared datatype inspection still checks the reviewed scalar/lexical/list subset,
original base/list/rule fields, exact leaf pins and exported datatype targets.
Only the explicit v2 entry point permits original constant slots. Module imports
must exactly match named module edges from datatype and constant-reference
dependencies; absent or unused imports refuse. Imports record intended dependency
relationships, never runtime scope-crossing grants. The root must export types;
other modules may be constant-only libraries with explicit constant exports.

Whole-source agreement spans module snapshots, legacy leaves and every supplied
CEMOS1 requirement. Each profile must name a reachable original datatype and
every supplied profile must be selected exactly once. Missing, duplicate, unused,
foreign or mismatched profiles refuse. After agreement, a private metadata mapper
recaptures and compares the complete dependency-snapshot fingerprint before
rebinding each CEMOS1 requirement into its module's original arena. Matching
logical URI/hash or coincident node IDs cannot substitute a different source.
Independent assembly verifications retain independent owners.

The [constant-reference inspector](../packages/cem_ql/src/datatype_module_capsule/v2/constants.rs)
retains original slots and explicit intended target sequences. Cross-module
targets must be exported original constant elements or reference nodes; local
targets retain their original owner. Reference chains are flattened in authored
order, preserving duplicate occurrences. Missing/unused bindings, unknown or
wrong-category targets, self/cross-module cycles and excessive depth/expansion
refuse, with original source witnesses for cycles. Exported reference nodes are
also inspected, including exports not selected by the root's vocabulary.

This is node-category and dependency metadata, not a second constant parser or
an evaluated selection. Constant field syntax, literal values, mixed enumeration
forms, effective inherited kinds/restrictions and actual selector completions
remain checks of the existing shared compiler under live source contexts and
policies. Neither reference expressions nor constant interpreters run here.
Terminal constant declarations stay unvalidated source nodes until that compiler
admits them. Both portable selection and executable-closure flags remain false.

The assembly is limited to 32 modules, 64 KiB per complete module record and
4 MiB for its bytes plus every supplied legacy/profile/source artifact. There
are at most 512 datatype/reference binding fields and 512 raw constant targets,
128 datatype bindings, 128 reference bindings and 128 constant exports per module,
128 targets per reference, 512 inventoried slots,
512 cached flattened occurrences and 512 final slot-target occurrences. Module,
datatype and reference graphs retain the shared 32-edge depth bound. Profile
inputs are limited to 32 bundles, 64 source leaves and 64 selected profiles in
total; each nested decoder keeps its existing limits. The outer phase uses
100,000 visits and 100,000-value / 64-depth MessagePack preflight. Source/profile
verification and explicit occurrence mapping retain separately bounded phases,
not one cumulative heap reservation.

The [assembly fixtures](../packages/cem_ql/tests/datatype_module_assembly.rs)
cover original contexts/owners, profile mapping, imports/exports, order/duplicates,
reference chains and cycle witnesses, legacy graphs/leaves, whole-source mismatch,
depth/expansion, malformed repinned input, aggregate/nested limits and cancellation.
Actual-host selection is the explicit inspection gate below. Compilation,
constant interpretation, executable ownership, application-envelope installation
and receiver/worker transport remain separate gates. No application/component
JavaScript or WASM endpoint was added. Entered credentials still block recovery; automatic whole-Studio recovery stays disabled,
with the existing 4 MiB checkpoint budget and five-second deadlines unchanged.

## Actual-host assembly selection

The [CEMDM2 adapter](../packages/cem_ql/src/structural_receiver/dependencies/assembly.rs)
adds explicit `register_datatype_assembly_scope`,
`prepare_datatype_assembly_lexical_scopes` and `select_datatype_assembly` entry
points on `FreshStructuralHost`. It borrows the verified assembly directly,
sharing the existing datatype name catalog, original source capture and actual
resolver used for CEMDP1/CEMDM1. It neither converts the assembly into an older
wire format nor constructs a substitute host. Constant-only modules retain their
original schema scope with an empty datatype vocabulary.

The caller supplies exact source scopes, ready source/expression contexts,
explicit datatype name imports and directed crossing grants. Preparing original
lexical occurrences preserves their captured namespace/name metadata and policy
overrides. A raw scope without that capture, copied/foreign source, extra scope,
missing context or unprepared selected occurrence refuses. A declared CEMDM2
module import records intent; it cannot create a runtime name import or crossing
permission. Destination policy limits remain effective at every hop.

Every attempt renews ordinary datatype selection. For reachable datatypes with
constant slots, the [constant adapter](../packages/cem_ql/src/structural_receiver/dependencies/constants.rs)
uses the shared owned-reference traversal over the original datatype container.
It delegates expression evaluation and readiness to the actual resolver, checking
the raw intended original target sequence at every reference occurrence and the
flattened terminal sequence at each owning datatype. Order, duplicates and source
arena identity must all agree. Returning the same terminal nodes while skipping
an intended intermediate reference refuses. Each intermediate/destination needs
its own ready context and applicable directed grant; a root-to-terminal grant
cannot replace an intermediate crossing. Unselected constant exports do not
require prepared expression occurrences.

Datatype and constant traversal debit one remaining traversal work allowance.
Metadata planning remains separately bounded by the caller's work limit; shared
depth, destination-policy bounds and operation cancellation also apply. Provider
callbacks run again on each selection; context liveness does not freeze mutable
callback state. Stops or context changes during evaluation prevent late success.

The returned `FreshDatatypeDependencySelection` is a process-local inspection,
with guarded datatype/constant slot and target counts. It retains immutable inputs
and a weak actual-context lease, without retaining selector callbacks. Context
changes, closure, host shutdown or explicit result retirement invalidate its
accessors. An unchanged context can keep an earlier inspection live while a
changed mutable provider makes the next attempt fail: an inspection is never
permission for compilation or publication. CEMDM2's portable selection and
executable-closure flags remain false.

The [actual-host fixtures](../packages/cem_ql/tests/datatype_assembly_runtime.rs)
cover exact original targets, ordered duplicates, transitive datatype and constant
chains, qualified-name imports, per-hop grants, raw/foreign/copied scopes, pending
source/occurrence contexts, intermediate shortcuts, mutable selectors, late stops,
work/depth/destination limits, unused exports and weak ownership/retirement.
The shared CEMDM2 compilation path below now performs literal admission and
effective datatype checks with explicitly owned operations. No new WASM endpoint,
application UI JavaScript or credential checkpoint path is introduced.

## Shared CEMDM2 compilation with renewed selections

`FreshStructuralHost::compile_datatype_assembly` consumes the verified graph,
actual root/source scopes, explicit datatype imports, primitive metadata, reachable
rule owners and an exact operation-owner map. It returns the existing private
`FreshDependencyDatatypeCompilation`, using the same actual resolver and shared
[datatype compiler](../packages/cem_ql/src/datatype_compilation.rs) as CEMDP1/CEMDM1.
No prior selection inspection is accepted as permission, no substitute host is
constructed and no CEMDM2 input is converted into an older wire format.

The [operation map](../packages/cem_ql/src/structural_receiver/dependencies/operations.rs)
uses `DatatypeAssemblyOperationKey { module, datatype, role }` and
`DatatypeAssemblyOperationBinding::Operation(Arc<FreshNativeOperation>)` or
`Tokenizer(Arc<FreshNativeTokenizer>)`. It must exactly cover the profiles on the
root's reachable original datatype graph. Missing, extra, wrong-role, foreign-host,
copied-source, stale or retired bindings refuse. An owner must match the original
CEMOS1 pin, source allocation, declaration and actual source scope. Requirements
on unused exports activate no code and require no owner.

Only primitive kind/representation/bounds metadata may enter directly; caller
validator/tokenizer hooks remain refused. Operations install in the existing
`DatatypeImplementations` registry, with tokenizers applied only to explicit
matching list metadata. Ready versus checked-replacement modes and source-backed
result adapters retain the shared compiler's admission rules. The completed
effective kind and representation must also match each reachable CEMOP2 profile.
`link_datatype_assembly_rule` binds original query/native leaf rules through the
existing explicit rule-owner path. Original leaf source/capture/datatype accessors
allow preparation without reconstructing sources.

Datatype dependencies are renewed during the compiler's ordinary walk. Constant
references use that same actual host, captured occurrences, original contexts,
destination policies, directed grants and cancellation checks. Every raw reference
sequence must match its intended original targets, preserving order and duplicate
occurrences. A shared native admission hook checks the completed terminal sequence
before literal parsing or interpreter callbacks. Skipping an intermediate reference,
selecting another source allocation or returning a wrong ordered sequence refuses.
The existing enumeration compiler alone parses constant fields, interprets values,
checks inherited restrictions and retains original token/candidate provenance.

Datatype dependency traversal and constant traversal share the compiler's remaining work
allowance. Metadata/name/owner planning remains separately bounded by the request;
this does not claim one aggregate heap reservation. Existing work/depth/destination,
constant/value/diagnostic and query-budget limits apply. All scoped callbacks share
one invocation's query budget. Retained-vocabulary comparison limits report a stop
with the original declaration when there is no legacy `@values` attribute.

Constant interpreters and inherited equality callbacks receive their original
operation declaration's restricted actual source context. Imported rules receive
their original behavior context, including while compiled constants are validated.
Compilation borrows actual registered contexts for one guarded invocation; it
stores no runtime context in the resulting descriptors. Closed native context
filtering remains intact: ambient input bindings, callbacks and module resolvers
are not inherited. Validation borrows current original source contexts afresh,
without replaying dependency selectors.

Compiled snapshots retain only weak operation/tokenizer/rule owners and immutable
source/result metadata. Keep all required owners alive. Owner retirement/drop,
implementation removal, changed contexts, host closure/drop or cancellation disable
use; callback availability and actual context/control are checked before and after
work. Failed attempts that change no context preserve earlier snapshots. Mutable
providers are evaluated on each new compilation; an earlier completed snapshot
remains a snapshot and grants no permission for a changed selection.

All execution entry points remain the graph root's exported datatype IDs. The
explicit root operation APIs below now share this guard. Versioned envelope and
mixed-receiver publication, plus worker/fallback transport, remain separate gates. Portable selection/code-closure flags remain false; this is a native local
compiler installation, not a code-closure or deployment attestation. No WASM endpoint
or application/component JavaScript is added. Credential privacy, disabled automatic
whole-Studio recovery, 4 MiB checkpoints and five-second deadlines are unchanged.

The [assembly compiler fixtures](../packages/cem_ql/tests/support/datatype_assembly_compilation.rs)
cover ordered retained constants, inherited source contexts, exact reachable owner
maps, copied-source and weak-owner refusal, mutable selections, pending contexts,
grants, literal diagnostics, fixed limits, imported rules, unused exports and late
cancellation. [Profile fixtures](../packages/cem_ql/tests/native_registrations/profile_pins.rs)
exercise all operation roles/tokenizers and every source-backed result profile.

## Guarded assembly root operations

[`FreshDependencyDatatypeCompilation`](../packages/cem_ql/src/structural_receiver/dependencies/compilation/root_invocations.rs)
now offers synchronous native `prepare_lexical`, `convert` and `serialize_list`
methods for CEMDM2 assembly roots. Each takes the actual `FreshStructuralHost`, an
exported root datatype ID, the existing shared input type and its operation limits.
It returns the existing `DatatypePreparation`, `DatatypeConversion` or
`DatatypeSerialization` report behind a host/lifetime `Result` guard.

One common execution guard now covers validation and all three operations. It
checks the actual host/context, graph root membership, compilation liveness and
every required weak operation/tokenizer/rule owner before and after execution.
A single reentry lock covers cross-operation calls as well as validation, and is
released after success, refusal or an interrupted call. Invalid/hidden roots,
foreign or closed hosts, changed epochs and dropped/retired owners refuse before
execution. A late cancellation or owner retirement withholds the complete report,
so a successful callback cannot expose a stale result. CEMDP1/CEMDM1 keep their
existing typed validation boundary; these new operation methods require CEMDM2.

The wrapper delegates to the existing shared preparation, conversion and
serialization engines. It adds no parser, converter, validation loop or list
formatter. Every inherited/checked preparation step receives the restricted actual
context of its original declaration. List-item preparation uses each item's
original preparer context. Conversion and serialization use their selected
implementation's original context. Subsequent validation uses each original rule
and equality context, sharing one query budget across all phases and items.
Closed native context filtering still excludes ambient inputs, callbacks and
module resolvers; no live context is stored in a descriptor or report.

Preparation preserves decoded UTF-8 token spans, scalar/list shape checks,
checked-preparer/tokenizer compatibility, cumulative call/input/diagnostic limits
and the meaningful distinction between empty lists and missing preparation.
Conversion validates its result without invoking preparation. List serialization
validates effective list/item constraints before calling the selected serializer,
then checks output byte limits. It does not serialize native ASTs. Each API runs
only its own explicitly selected capability; it never falls back to a different
operation. Missing capabilities retain the shared `NoPreparation`, `NoConverter`
or `NoSerializer` stop rather than activating a default.

Shared reports preserve accepted, rejected, pending, unavailable, failed, invalid
shape and budget/control stop semantics. Prepared/converted values can remain
inspectable after ordinary validation rejection; successful publication still
requires `accepted == Some(true)`. Diagnostic attribution and original candidate
handles survive the shared adapters. Failed typed validation produces no list
export text. Empty validated lists can explicitly serialize to empty text.

One call is one bounded invocation. Nested callbacks, inherited steps, item work,
validation and query work share that call's existing allowances; independent calls
start fresh allowances. These synchronous reports are not sealed preparation
receipts, a portable execution grant or receiver publication. The existing sealed
CEMDT1 handoff below is unchanged. No WASM endpoint, runtime transport or app/UI
JavaScript is added. Entered credentials still block recovery until explicitly
cleared; automatic whole-Studio recovery stays disabled, with 4 MiB checkpoints
and five-second deadlines unchanged.

The [root-operation fixtures](../packages/cem_ql/tests/support/datatype_assembly_root_ops.rs)
cover inherited/checked source contexts, decoded spans, empty lists, rule-source
validation, pre-export rejection, completion states, incompatible replacements,
candidate/diagnostic provenance, cumulative/query limits, weak owners, stale/foreign
hosts, cross-operation reentry, late cancellation and retirement. The
[module fixtures](../packages/cem_ql/tests/datatype_module_runtime.rs) preserve
legacy-format refusal; [profile fixtures](../packages/cem_ql/tests/native_registrations/profile_pins.rs)
exercise checked source-backed conversion through the same root entry point.

## Guarded datatype preparation handoff

`FreshDatatypeCompilation::preparation_invocation` creates a frozen lexical
invocation for an explicitly exported root on the actual live host. It returns
[`FreshDatatypePreparationInvocation`](../packages/cem_ql/src/structural_receiver/datatypes/preparation.rs),
which offers `prepare`, `validate_prepared`, `close` and `remaining_limits`.
The shared [datatype evidence implementation](../packages/cem_ql/src/preparation_evidence/datatype.rs)
uses the existing attribute evidence's immutable-value checks, metadata accounting,
work reservation and preparation refund rules. It does not add a parallel
parser, datatype validator or conversion path.

Preparation returns inspectable diagnostics and a separate optional sealed
`SealedDatatypePreparationEvidence`. The seal has no public constructor, codec,
mutable access or acceptance verdict. Raw inspection reports and caller-authored
value/text pairs cannot manufacture it. Primitive values and the engine's exact
immutable scalar/token adapters are eligible; an unrelated view imitating their
representation string refuses. Original lexical text, decoded ordered token
spans, diagnostics and the frozen candidate remain distinct. A present empty
list can carry evidence; missing evidence stays incomplete. Rejection, pending,
unavailable, failed or untracked preparation cannot issue a seal, and a
typed-only datatype never gains implicit lexical preparation.

The issuer is bound to the exact compilation, root and invocation. Identical
names, pins, values or input text do not authorize another invocation, root or
host to consume a receipt. The compilation is held weakly: dropping or retiring
it invalidates retained invocations/evidence. Closing the invocation or changing,
closing or dropping its actual host/context invalidates use too. Source-less
input is explicit; an authored candidate must be an original literal attribute
with matching text and source stack. Root preparation makes no attribute-default,
local facet, crossing-grant or receiver-publication claim.

Every `validate_prepared` call reruns the ordinary datatype validator and current
rules against the sealed immutable values. Native callback state may now reject
or remain pending where an earlier consumption accepted. Preparation is not
replayed and no verdict is cached in the receipt. Host/compilation/context and
control are checked around calls; callback-time retirement or cancellation cannot
publish a completed handoff. Executable snapshots remain private, and no raw
preparation capability is exported.

Preparation and consumption share one finite remaining allowance for preparer
calls, output values, inspected/scheduled input visits, rules, comparisons and
diagnostics. Even empty receipts spend inspection work. Cloned evidence or
retaining an invocation does not reset its budget. The shared reservation refuses
reentrant callbacks; an unwind conservatively spends the reserved allowance.
Availability hooks have a separate reentrancy guard so they cannot prepare
recursively before reservation. Existing lexical, value and 1 MiB evidence
metadata limits remain unchanged; these checks are not a measured peak heap.

Selected and inherited tokenizer owners now receive read-only, control-checked
availability inspection before retained root **and attribute** evidence is used.
Retiring a tokenizer invalidates its lexical proof without repeating tokenization.
The existing checked preparation/facet guards remain in force. The
[root fixtures](../packages/cem_ql/tests/datatype_source_capsule/preparation.rs)
and [attribute regression](../packages/cem_ql/tests/datatype_validation/preparation_evidence.rs)
cover original spans, empty/missing/stopped input, immutable-view refusal, source
metadata, foreign/stale issuers, lifetime, fresh rules, cumulative limits,
callback reentrancy/unwind and tokenizer retirement.

This is a native, process-local root handoff. CEMDT1 still rejects executable query
bodies, imported/reference dependencies and constants requiring their own
adapters. Complete behavior/query/imported closure, mixed receiver wire contracts,
local receiver installation and worker/fallback generation/checkpoint enrollment
remain pending. No component/application JavaScript or WASM endpoint was added;
existing wire formats, credential refusal, disabled automatic whole-Studio
recovery, 4 MiB checkpoints and five-second deadlines are unchanged.

## Native fresh-host linking and node receivers

[`cem_ql::structural_receiver::FreshStructuralHost`](../packages/cem_ql/src/structural_receiver.rs)
owns a concrete query host, operation control and a private live context. It is
neither cloneable nor serializable. Its configured trusted registry hash must
match the verified capsule's retained pin; matching names, source URIs or hashes
alone do not register an implementation or grant access to another scope.

Register the receiver declaration source and data scopes, set contexts and grant
directed crossings first. Then call `link_receiver` with the verified capsule,
receiving schema URI, exact registered declaration tree and original attribute
occurrence. Linking checks the receiving schema/collection, literal `node` or
`schema:node` type, original owner, unique declaration and supported local
cardinality fields. The receiving source uses the bounded literal schema
inspector, with node count fields enabled. Unknown policies, duplicate namespace headers, dynamic preludes
and unsupported collections refuse instead of being ignored by compilation.
It constructs the shared native
[`StructuralNodeReceiver`](../packages/cem_ml/src/schema/structural_receiver.rs)
and returns a new receiving model only after live-context checks. A failed link
does not replace a previously returned model or mutate grants. The expected
registry/build hash is supplied by the trusted embedding; physical worker/module
registry attestation remains a later deployment gate.

This first adapter uses ordinary schema attribute declarations with explicit host
selection of the structural schema. It does not resolve a named type alias from
`TemplateData.value_types`, add a template renderer API, or admit `CEMAPPD2`.
Defaults, scalar facets on a node receiver, unsupported declarations and
unavailable executable/tokenizer implementations refuse. Local `itemCount`,
`minItems` and `maxItems` must be consistent literal bounds from 0 through 64;
the shared native default is exactly one node when no count facet is authored.
Explicit minimum zero admits a completed empty selection, not a pending one.

`validate_input` uses the ordinary native reference walker and its directed
grants to capture targets. Only this host supplies the native receiver context.
The captured access privately records the live epoch, exact originating data
attribute/tree, selected original receiver declaration, schema identity and local
receiver metadata. None of those live receipts is serialized. A bare node
capture cannot stand in for a linked receiver invocation, and an otherwise valid
capture cannot be relabelled to another attribute or host. The shared dispatch
checks the receiving snapshot before calling the contract; changed metadata
requires relinking instead of silently ignoring new restrictions.

The linked adapter validates the already-authorized literal forest through the
same shared structural validator and scalar/count rules. It preserves order,
repeated occurrences, original owners and name snapshots. A closed access host
exposes only the captured owning closure, polls operation control and has no
reference evaluator or resource loader. Nested references/native expression
slots, unsupported root kinds and unavailable names remain incomplete; strings
are never parsed into nodes. Original target diagnostics match independent
full-native validation, while local receiver-count failures belong to the
original caller attribute. Compiling the receiving model uses retained native
declarations; neither target data nor source text is reparsed during validation.

The value gate permits at most 64 element roots, 16,384 captured placements, 64
levels and 1 MiB of accounted literal/name/source-map content. One 100,000-visit
budget is shared by literal-access preflight and subsequent closed-host name
reads across the complete root sequence. Node roots must carry complete original
source/name snapshots. Each root's shared literal validation is bounded by that
aggregate preflight and the existing traversal limits. This is a bounded native
adapter, not a claim to cover dynamic schema evaluation or measure peak heap.

Registering a scope, replacing a context or adding a grant advances the host
epoch and expires old links/captures. Repeating an existing grant is idempotent.
Relink and recapture under the current host before further validation. Explicit
close and drop invalidate the context even if another caller retained a context
clone; a closed host cannot reopen by registering a scope. Model retirement
deactivates its exact adapter. Control, epoch and retirement checks run again
before returning a verdict; acceptance is not cached or issued as a portable
preparation capability.

The [native receiver acceptance tests](../packages/cem_ql/tests/structural_receiver.rs)
cover structural/scalar/cardinality parity, mixed source owners, repeated/empty
targets, provenance, registry/source mismatch, stale and relabelled captures,
foreign/closed hosts, model changes, retirement, bounds and cancellation. This
delivery adds no application/component JavaScript or worker/checkpoint entry
point. Existing WASM generations continue refusing native schema capsules;
compiled-component/worker integration must establish fresh ownership and demand
contracts separately. Entered credentials still block recovery, with the same
4 MiB checkpoint budget and five-second handoff deadlines.

Current regression fixtures in
[`named_receiver_types.rs`](../packages/cem_ql/fixtures/component-application/named_receiver_types.rs)
exercise correctly repinned node/list/content-type, unsupported inherited
restriction/model-policy, executable-extension and future-version refusal
against v1 native admission and independent WASM/browser heaps. They preserve a
live scalar owner and a subsequent valid render. These tests protect today's
boundary; they do not implement the proposed executable linker or component admission.
