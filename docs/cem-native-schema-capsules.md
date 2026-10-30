# Portable native schema contracts for component applications

Status: design contract for the separate component-application WASM target.
Executable and node-schema capsule export, reload and receiver admission are
**not implemented**. The deployed `CEMAPPD1` format remains scalar-only. Its
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

Reserve a separate native schema capsule, tentatively `CEMSC1`, embedded by a
future application declaration version, tentatively `CEMAPPD2`. These identifiers
are design names, not available loaders or package exports. Older runtimes must
refuse them. Do not reinterpret a v1 scalar model, attach ignored executable
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

Current regression fixtures in
[`named_receiver_types.rs`](../packages/cem_ql/fixtures/component-application/named_receiver_types.rs)
exercise correctly repinned node/list/content-type, unsupported inherited
restriction/model-policy, executable-extension and future-version refusal
against v1 native admission and independent WASM/browser heaps. They preserve a
live scalar owner and a subsequent valid render. These tests protect today's
boundary; they do not implement any stage of the proposed executable loader.
