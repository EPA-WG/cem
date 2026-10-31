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
`CEMAPPD2` dependency envelope now packages and inertly verifies compiled
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
Native `CEMOP2` now groups exact CEMOS1 pins through the shared profile validator,
with independent whole-source agreement before occurrence mapping. These are
still inert source requirements, not installed module/receiver capabilities.
Native `CEMDM2` now assembles original datatype modules, CEMOP2 profiles and
intended constant-reference targets, with explicit module imports and bounded
graph checks. It remains separate from CEMDM1 and actual-host installation.
WASM transport/generation enrollment and compiled-component receiver admission remain
**unimplemented**.
The deployed `CEMAPPD1` format remains scalar-only. Its
existing wire version must not acquire executable semantics implicitly.

This extends the [component profile](cem-components-wasm-profile.md) and
[application declaration contract](cem-components-shopping-cart-profile.md).
Implementation and fixture gates are in
[the profile checklist](todo.md#component-application-wasm-profile-separate-target).

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
provides native-only `export` and `verify` entry points for `CEMQV1` programs.
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

## Exact native validator and tokenizer requirements

[`cem_ql::native_registrations`](../packages/cem_ql/src/native_registrations.rs)
exports and verifies native-only `CEMNR1` requirements. Each record carries an
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
`VerifiedNativeRequirement` conversion, native-linker or CEMOP1 admission path.
CEMNR1 still refuses reference-bearing sources, and CEMOP1 still accepts only its
exact closed CEMNR1 dependencies. The separately versioned CEMOP2 pins and CEMDM2
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
Actual-host compilation, constant selection/interpretation, executable ownership,
application-envelope installation and receiver/worker transport remain separate
gates. No application/component JavaScript or WASM endpoint was added. Entered
credentials still block recovery; automatic whole-Studio recovery stays disabled,
with the existing 4 MiB checkpoint budget and five-second deadlines unchanged.

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
