# Portable native schema contracts for component applications

Status: the native-only structural `CEMSC1` exporter/verifier and explicit
fresh-host node receiver linker are implemented. They restore a bounded shared
model and bind its use to a concrete native host and invocation. A native-only
`CEMQV1` capsule also verifies a closed subset of compiled validator queries and
registers them against separately supplied checked original declarations.
Portable query declaration-source transport, exact native implementation linking,
WASM transport and compiled-component receiver admission remain **unimplemented**.
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

The native structural capsule uses `CEMSC1`; embedding requires a future
application declaration version, tentatively `CEMAPPD2`. The latter remains a
design name, not an available component loader or package export. Existing
component runtimes refuse both. Do not reinterpret a v1 scalar model, attach ignored executable
fields to it, or silently route it to a general schema parser.

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
inherited/imported schema composition, complete query declaration-source transport
and exact native implementation registration remain gates before deployment admission.

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
registry; duplicate/failed registration preserves existing entries. Original
declaration transport and host/generation enrollment are still required before
this can be used as a self-contained application or recovery payload.

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
