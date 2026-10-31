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
