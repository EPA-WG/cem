# cem-ml

`cem-ml` is the Rust library that owns CEM's schema-defined parsing, validation,
conversion, query, transformation, reporting, scheduling, source-map, and
operation-control semantics. Its version is the authority shared by the native
CLI and synchronized npm/WASM deployment packages.

## Public boundary

The crate exposes reusable library APIs and a WebAssembly-compatible `cdylib`.
It owns content-type and schema identities, typed lifecycle artifacts, command
services, transformation graphs, and portable diagnostics. It does not own CLI
argument policy, browser UI, component behavior, or package-specific deployment
wrappers.

The low-level npm/WASM deployment is published separately as
[`@epa-wg/cem-ml`](../cem-ml-npm/README.md). The native command host is the
[`cem-ml-cli`](../cem_ml_cli/README.md) crate, while CEM-QL evaluation is owned by
[`cem-ql`](../cem_ql/README.md).

`CemtTreeArtifact::to_public_json` produces an explicit public/debug sidecar.
It exports formatted node sequences in one pass over their gaps and retained
nodes, borrowing the native tree and preserving overlay order and provenance.
Native pipeline stages retain the artifact; this export is not an AST handoff.
Indexed access to the borrowed evaluator view remains available.

Portable Inspect requests carry an optional `format` (`cem` or `json`). Missing
fields in older requests retain the default CEM-ML presentation. Shared command
lowering preserves an explicitly requested JSON inspection through preparation
and worker execution, with JSON output identity and no terminal color encoding.
This is a named structural export for callers; retained native trees remain the
runtime handoff. CEM-ML presentation uses native collision-free attribute literal fences for
embedded template strings; explicit JSON remains a separate named export.

JSON/YAML/JSON Schema Parse and Inspect use `import::retain_lifecycle` rather
than reinterpreting their source bytes as CEM. Their AST/events output identity
is the presentation vocabulary; root metadata carries the selected source
schema/MIME, original URI and ranges. Events describe the common imported tree.
The original native owner retains lexical details; the imported projection is
not a lossless language-specific AST. Literal query-looking strings stay inert.
Malformed sources, unsupported YAML features/dialects and import limits produce
no structural artifact; diagnostic inspection remains available. Existing CLI
failure levels are unchanged, and strict source examples select `validate`.

## Native document I/O

### Optional immutable schema assets

`schema::asset_pack` provides a native-generated catalog and a per-consumer
`SchemaAssetPack` for explicitly admitted immutable source bodies. Authenticate
the exact index digest independently, then acquire/admit only required
package/version/path entries. Defaults bound the index to 4 MiB, 1,024 sources,
each source to the document import limit and resident source bytes to 32 MiB;
these are source-store limits, separate from retained arenas and checkpoint
budgets. Resolver policy/cancellation and digest, length, MIME and UTF-8 checks
apply before admission. Failed reads never consult embedded sources.

`load_package` revalidates the native manifest/schema identity against the
bootstrap. Source import preserves its original catalog URI and creates fresh
native owners; CEM/XML sources retain lexical capture, while data-format sources
use the shared data importer. Catalog metadata alone does not activate a schema.
`SchemaPackage::from_sources` accepts owned manifest/schema text and verifies the
selected bootstrap metadata through native imports. Package consumers use shared
source-text inputs; `BuiltinSchemaPackage` remains a compatibility name.
Admitted text uses `Arc<str>` storage, while the complete embedded default
adapter borrows its existing static bytes without a second catalog copy.
`SchemaAssetPack::owned_package` supplies the same consumer object after pinned
native identity validation, and that object's text survives releasing the store.
Authoritative packages extract conversion metadata from their own manifest,
without validating against a similarly named embedded template.

Use `SchemaAssetPack::conversion_reader()` for opt-in packed transformations.
It checks the original admission on every formatter/colorizer read and cannot
select an embedded artifact when a source is missing. The per-pipeline parsed
module cache holds at most 256 entries, keyed by the full artifact descriptor,
canonical source URI, source content type and SHA-256 of the actual bytes.
Admission runs before every lookup, including cache hits; cached options never
satisfy a refused read. Identical admitted sources can share parsed options.
`ConversionPackageArtifactReader<'a>` now names the object-safe
`ConversionPackageArtifactSourceReader` contract. Closure values and the
environment's lifetime shape remain supported; call erased readers with
`reader.read(artifact)`. Wrap an already erased `&dyn Fn` in a forwarding closure
when supplying it to the environment.

`schema::asset_pack_activation::activate_schema_asset_packages` publishes exact
package versions into an ordinary native `EngineContext`. Supply an immutable
`Arc<SchemaAssetPack>` and a bounded selection such as
`&[("cem-dom-projection", "1.0.0")]`. Admit manifest/schema bodies first; declared
converter and formatter identities must exist in the pinned index, while their
bodies can be acquired before the operation that needs them. Dependencies must
already be registered or belong to the selected set. This API checks the native
manifest/schema identity and uses the existing model compiler, readiness rules
and exact `SchemaPackageReplacementGrant` contract. A built-in package cannot be
replaced merely by supplying a pack or manifest.

Publication of the selected set is atomic. Refusal preserves the previous
registries, source binding and prepared contracts. Old preparations retire only
after the complete set succeeds. Context clones share the immutable source
allocation; releasing the last clone releases that ownership. Republishing a
context cannot mix index pins or discard a previously owned admitted body.
The ordinary mutable-manifest loader also cannot change a pinned package, even
with an owner replacement grant; use a fresh consumer context to change its pin.
`SchemaAssetActivation` exposes its index digest, retained source count and bytes
for embedding ownership accounting; these values confer no retirement grant.

Ordinary converter/template imports and output pipelines consume this binding.
Selected package reads preserve canonical source URIs and reject missing bodies,
MIME/identity mismatches and escapes to filesystem/embedded fallback. Unselected
packages retain their legacy behavior. The reader contract's
`allow_embedded_fallback_for(artifact)` permits this mixed context; its default
preserves existing readers' `allow_embedded_fallback()` policy.

The native tests activate the actual DOM projection and CSV packages and exercise
ordinary conversion, validation, formatter/colorizer admission and owned
schema diagnostic overrides. Incomplete or unsupported models remain refused;
no diagnostics are discarded to make activation succeed.

CSV's `csv-parse-report-fact` constraints compile in a registered native parser
phase. The compiler checks all eight actual decoder/parser facts, their targets,
unique bindings and declared diagnostics. Parser/encoding failures must stay
hard violations; advisory diagnostics can be upgraded, and diagnostic codes,
contract names and policies remain schema-owned. Missing, unknown, duplicated,
redirected or unsafe bindings cannot publish an executable contract.

`compile_schema_document_model_from_retained` pairs the compiled parser contract
with the original retained schema tree. The linked CEM-QL compiler forwards that
same owner through its registered source scope. A foreign or unbound owner, or a
custom compiler omitting the required native contract, cannot publish a CSV
model. Plain `compile_schema_document_model` remains useful for inspection and
legacy catalog extraction, with its parser contract unbound and not ready for
publication. Low-level catalog constructors keep their existing behavior;
ordinary activated consumers use the checked contract.

The lifecycle import boundary dispatches original source bytes to the compiled
native parser. CSV facts, typed ASTs, source positions and authored diagnostic
bindings come from that invocation, without rereading embedded schema bodies.
Native parser readiness is separate from structural CEM-AST readiness:
`is_ready_for_structural_validation` and structural scope/region checks refuse a
parser-only model. A CEM projection cannot stand in for validated CSV syntax.
A refused replacement also preserves the actual default parser incumbent.

The optional WASM profile also exposes a persistent embedding-owned source
session (`schemaAssetOwnerOpenV1`, `schemaAssetOwnerAdmitV1`,
`schemaAssetOwnerActivateV1`, `schemaAssetOwnerDisposeV1`). One session owns the runtime's published package
context; ordinary command and resolver contexts clone it, and CSV resource
imports use its checked parser at the native import boundary. Opening requires
explicit trusted command-host configuration; package requests cannot supply
replacement grants. Failed activation preserves the incumbent, and repeated
successful selections are idempotent. The `schema-package-owners` inventory root
blocks retirement until exact-owner cleanup; asynchronous commands independently
retain their native activity. Closed admission permits cleanup only.

Owned immutable package admission checks declaration metadata and compiles its
native schema contract separately from deployment-example execution and eager
formatter compilation. Full package `validate`/`check` keeps its existing resource
checks. Runtime artifact readers remain authoritative, including strict refusal
of missing selected bodies. Adding admitted bodies republishes a fresh immutable
source binding; unchanged stores reuse their existing publication identity.

The shared optional loader exposes this through existing declarative HTTP
resources and reimports/recompiles active packages on a distinct target heap.
This API does not enable automatic recovery, publish a generation, serialize
native owners, or change credential/checkpoint admission. Opt-in Studio
deployment and coordinated generation publication remain separate integration
work; default complete release artifacts remain intact.

The `schema-asset-packs` feature adds stateless WASM probes, leaving the default
ABI/catalog intact. The native producer and isolated offline/WASM verification
commands are documented in the
[deployment package](https://github.com/EPA-WG/cem/blob/develop/packages/cem-ml-npm/README.md#optional-native-schema-asset-packs-prototype).

### Shared imports

`import::import_data_bytes` resolves external content types into retained CEM
AST documents. XML, XHTML, SVG, MathML, HTML, JSON, YAML, CSV and CSS semantics
belong to this boundary; query/render consumers receive native nodes. XHTML,
SVG and all three MathML media profiles retain their original lifecycle wrappers
and source ranges while reusing XML semantic import. Their family contracts
apply before query or template execution, including SVG execution/resource
policy and MathML root/namespace/profile checks. Advisory MathML annotation
references remain lexical and are not dereferenced. Explicit family media types
are selected before the generic `+xml` fallback; unrelated `+xml` types retain
generic XML identity. These contracts preserve the existing family validation
limits and do not provide complete vocabulary validation or HTML sanitization.

The explicit `value::json::write_json(&CemValueGraph, &CemValueArtifactLimits)`
export writes one generic-data document/value or JSON-compatible native scalar.
It follows reference targets, preserves property order, duplicate keys and exact
number lexical values, and escapes string content. It rejects unsupported shapes
or scalar types without publishing partial output. Export never mutates nodes.
Graph values, graph/output bytes, depth and bounded reference expansion obey the
supplied limits. It produces compact JSON; retaining original formatting is an
import-owner concern, not a promise of explicit export.

## Lexical scope capture

`schema::machine::CemSchemaMachine::track_lexical_scope` wraps a normalized
event stream for `CemAstBuilder`. Its observer sees the effective state before
each incoming event and receives `None` once after final EOF validation.
`lexical_snapshot()` returns owned namespace and schema metadata that callers
can retain after frames close; `diagnostics()` exposes validation findings.
The wrapper forwards the original events without reparsing or keeping history.

`build_with_lexical_scopes()` builds one fragment and returns a
`LexicallyScopedDocument` sharing the original AST allocation. Its
`snapshot(owner, node)` lookup checks owner identity and retains snapshots for
surviving standalone/native attribute reference and general expression nodes.
Associations use builder node identities, without matching source offsets.
Schema-machine diagnostics remain available separately from builder diagnostics.
`parser::tree::RetainedCemTree::from_shared` can project that same allocation for
query consumers, applying the same structural checks as the owning constructor.

Capture does not evaluate references or schema selectors. Completion does not
establish schema readiness. Preparing runtime contexts, effective policies and
crossing grants remains consumer lifecycle work. The QL host's explicit
`attach_captured_lexical_scopes` callback accepts this metadata at that stage;
runtime preparation remains explicit.

`import::import_xml_ast_with_lexical_scopes(document, schema)` captures standalone
XML reference occurrences during the existing import pass. The returned
`ScopedXmlCemImport` contains the scoped original AST, semantic metadata and event
correspondence; pass its owner and semantics to `RetainedCemTree::from_shared`.
The XML parser's expanded namespace names select CEM schema behavior, including
authored aliases and local namespace declarations. Capture preserves entity/CDATA
payload provenance and final schema diagnostics. XML attributes remain literals.
The same explicit QL lifecycle handoff consumes these associations;
runtime preparation remains explicit.

`LexicallyScopedDocument::namespace_binding(owner, node)` exposes completed
bindings from original CEM namespace directives and CEM/XML namespace attributes.
Lookup checks original allocation identity, preserving earlier URI values across
rebinding and child restoration. `namespace_references::admit_namespace_scope_target`
accepts those declarations without interpreting schema/data nodes as namespace
sources. Empty default URIs are ready resets. Given root bindings do not create
source declarations. Admission leaves evaluation, singleton checks, crossing grants
and activation to explicit consumers tracked in `docs/todo.md`.

Native namespace attributes retain pending declaration IDs rather than inheriting
an earlier URI. Owner-checked `pending_namespace_declaration`,
`pending_namespace_name` and `pending_namespace_bindings` accessors expose original
declarations, dependent QNames and expression-prefix associations. Existing default
aliases retain their earlier pending declaration. `NamespaceNameCompletion` accepts
explicit admitted results and requires the selected source forest's dependencies
to be complete. It retains scalar name metadata and original owners, preserving
fixed names and authored reference descendants. CEM-QL's `NamespaceQueryTree`
exposes an independent native name view with explicit authored `source` access;
The explicit CEM-QL declaration host provides bounded namespace selection through
`attach_captured_namespaces` and `prepare_namespace_scope`. Pending property-value
consumption is available through `prepare_namespace_property`; the explicit
`activate_namespace_properties` consumer connects ready reports to selected-name
completion and original occurrence lexical handoff. Its returned completion can
enter shared native query ingress without reevaluation or source mutation.
`with_completed_namespace_names` supplies selected completion to schema admission
and control discovery for one explicit invocation, without replacing original
captured names. Shared structural validation uses the host's `input_expanded_name`
hook for owning and selected attribute permissions/typing; pending names defer
presence/field checks, retaining source values, diagnostics and traversal bounds.
Pending schema QNames retain their original body/following form;
completed namespace identity still determines core kind. Retained placements and
native attribute targets now snapshot `InputNodeView` names and optional original
source provenance. Pending names keep dependent behavior incomplete; authored
subtrees/references stay intact. Completed-declaration publication and automatic
opt-in lifecycle coordination remain tracked tasks.
A ready native query/admission view does not establish structural validation
readiness.

`namespace_references::decode_native_namespace_property` accepts the original
captured native namespace attribute and returns its destination prefix and owning
reference/general-expression slot. Original allocation identity and occurrence
capture are required; literals, default aliases, ordinary attributes and XML
expression-looking namespace literals are not reinterpreted. Decoding performs
no evaluation. The QL consumer supplies contexts, readiness and crossing authority
and consumes this exact slot through the shared bounded graph resolver.

`NamespaceNameCompletion::binding_namespace_uri` accepts an original declaration
handle and returns its completed URI or pending dependency. It follows captured
default aliases, preserves empty literal resets and rejects foreign owners and
ordinary nodes. An inherited declaration can be outside the selected query forest;
reading its metadata does not expose it through axes or create an expression context.

`NamespaceNameCompletion::lexical_snapshot` accepts only selected original
expression occurrences. It requires every captured pending prefix to complete,
including unused prefixes. `NamespaceLexicalSnapshot::original()` preserves the
saved scalar namespace/schema metadata; `namespace_uri(prefix)` reads this
execution's completed URI or the original completed binding. Empty URIs remain
ready resets. `completed_bindings()` retains original typed declaration handles,
including pending default aliases; no namespace record IDs are manufactured.
The explicit QL completion handoff uses these snapshots before constructing
runtime contexts, leaving the source owner and original snapshots unchanged.

Shared query preparation accepts `query::QuerySourceOwner::NamespaceCompleted`
with the original retained tree and an explicit consumer's name completion.
`query::run_query_with_source_owner(request, owner)` uses the shared query stages
without loading or reparsing input bytes; the request supplies input identity,
scope, query and limits. CEM-QL admission checks matching AST allocation identity
and projects only selected roots. Original source inspection remains available;
capture, completion and source references are not evaluated or rewritten during
ingress. Existing `run_query` retains its ordinary loading path, and namespace
property activation remains an explicit consumer task.

Binary AST reload currently preserves source expressions, source-map frames and
supported single-arena reference graphs through the debug CEMB codec. Captured
lexical metadata is a separate, allocation-bound input. The
[adopted reload contract](../../docs/cem-ql-cem-ml-node-references-design.md#binary-reload-and-lexical-handoff-adopted-2026-10-06)
is implemented by `ast::reload::ReferenceReloadBundle`: a versioned MessagePack
source manifest/passive lexical sidecar tied to the exact payload through SHA-256,
with capture reconstructed over the decoded owner. Byte and binary collection
counts are bounded; every referenced source ID requires a manifest entry. AST-only reload supports inspection; it cannot claim
equivalent capture-dependent evaluation without a verified lexical handoff.
`require_lexical` returns a typed missing-metadata dependency;
`ReloadSource::supply_bytes` verifies a later explicit source-byte handoff.
`ast::reload::ReloadIngress` supplies `query_source_owner` for inert inspection
and `validation_request` for embedding validation stages. The latter returns
`MissingLexicalMetadata` before capture-dependent validation admission, and
retains the same decoded arena for context/grant retries. `decode_with_document`
returns the verified decoded owner with the envelope, avoiding a second decode.
Primary source manifest ID is explicit; each node retains its mapped URI and
unknown coordinates when its source bytes are absent.
Runtime contexts, package/crossing grants and pending sessions are supplied afresh.
Passive namespace and inline-schema lookup maps serialize in key order, while
runtime hash lookup and authored declaration order remain unchanged. Newly
exported bundles therefore retain identical bytes across decode/re-export and
independent captures of the same source with equivalent passive metadata.
Historical unordered wire maps remain
readable; re-exports use canonical ordering without a protocol-version change.
This normalization does not change the AST payload, source bytes/fingerprint,
lexical ownership or admission of runtime contexts and crossing grants.
CEMV's materialized value transport has a separate acyclic graph contract and
does not preserve executable source references; see the design's
[transport matrix](../../docs/cem-ql-cem-ml-node-references-design.md#graph-export-and-native-transport-adopted-2026-10-06).

`EngineContext.schema_package_sources` captures these bindings alongside each
valid package source tree. Its `get_lexical_scopes(uri)` accessor and the installed
compiler's `SchemaPackageCompilationRequest.lexical_scopes` share the same original
AST owner. Unchanged reads reuse both; changed sources replace both together.
Descriptor extraction borrows that tree, while compiler callbacks supply current
contexts, policies and grants through the explicit QL handoff. Machine diagnostics
remain inspectable and do not replace the existing package publication gates.
The ordinary CEM pipeline now shares its schema-machine stream with AST
construction. `PipelineRun.document` is an `Arc<CemDocument>` and
`PipelineRun.lexical_scopes` shares that owner, including root namespace and
module-map bindings. Diagnostics and root version pins are finalized before the
owner is shared. `InputValidationRequest.lexical_scopes` exposes the same capture
to an explicitly installed validation stage, which can prepare occurrence contexts
through `attach_captured_lexical_scopes`. Parsing does not evaluate references.
Existing callbacks remain usable without adopting capture. With an explicitly
installed stage and a ready consuming model, ordinary XML validation uses the
specialized importer and supplies captured bindings too. It reuses an already
parsed native XML document or parses custom-schema XML once, retains that native
owner on the query tree, and preserves imported source semantics. XML attributes
remain literal. Source-only XML and other specialized XML-family validators keep
their dedicated paths. JSON/YAML/CSV validate/check inputs with a ready model
reuse their existing lifecycle AST through the same native import as query
ingress. Their native source owners remain attached to the retained tree, and
`lexical_scopes` is `None` because their data payloads remain literal. Missing
or incomplete consuming models do not invoke the stage; an inspected incomplete
model or a failed native import cannot report completed runtime validation.
Parse/load does not invoke the stage. Other specialized validators keep their
existing paths.

## Verification

Use Nx as the workspace task authority:

```bash
yarn nx run cem_ml:lint
yarn nx run cem_ml:test
yarn nx run cem_ml:build:wasm
```

The acceptance criteria are documented in
[`docs/cem-ml-ac.md`](../../docs/cem-ml-ac.md), and synchronized distribution
ownership is defined in
[`docs/cem-ml-deployment-contract.md`](../../docs/cem-ml-deployment-contract.md).

## Named source provenance

`SourceMapStack::with_source_uri` appends a `SourceBoundary { uri }` frame to a
nonempty, unnamed stack, preserving the existing source ID and original UTF-8
range. An existing boundary cannot be rebound. Native owners retain their
explicit source URI even when their XDM document URI is absent. Numeric IDs stay
local and must not be interpreted as resource addresses.

The binary source-map codec encodes this transform kind as tag 15 with a nonempty
URI payload; older tags retain their encoding. New readers accept existing
streams. Older readers that do not support this kind reject such new metadata.
The explicit source-map JSON protocol exposes the same frame. Runtime ASTs remain
retained native trees; these public source maps are not an AST transport.

## Native dependency identities

`dependency_identity::DependencySnapshot::identity()` derives a versioned,
BLAKE3-domain-separated identity for template compilation, derived stylesheets or
render projections. This is the identity/invalidation foundation; it does not
lookup/store artifacts or enable reuse. The producer supplies an already
validated, complete transitive closure with original source URIs, content
identity/type/schema, resolved parent/specifier edges, compiler/IR versions,
effective options and explicit policy/resolution/scope versions. Dependency graph
edges are canonicalized; semantically ordered inputs must be represented in
options as well. Unknown completeness is not `Ready`.

Versions are either native content fingerprints, explicit provider-owned
`{ owner, generation, revision }` stamps, or `Opaque`. All supplied dependencies
must be versioned. Render projections additionally require separate data,
attributes, payload, slices, resources, form, validation, accepted events,
explicit refresh and evaluation-provider revisions. Providers include clock,
random and plugin/native-function state. Unknown freshness remains opaque;
unchanged ordinary inputs alone cannot authorize skipping evaluation. A provider
must establish a valid revision/freshness contract before using a revision stamp.
Canonical maps and validated fingerprints avoid hashing source/data payloads on
each attempted reuse; compute identities at canonical mutation/import boundaries.

Admission fails closed for pending, failed or released closures, opaque versions,
missing render axes/contexts, malformed hashes, absent/conflicting graph identities
and excess metadata. Limits are 1,024 source identities, 4,096 import edges, 128
option entries and 1 MiB encoded identity metadata (including JSON escaping).
The preflight string budget bounds serialization allocations. Sources, trees,
DOM, callbacks, handles and history are never retained by an identity or error.

`DependencyAdmission` is a separate nonportable owner/runtime-generation/revision
guard for publication. An equal portable dependency key does not revive a native
handle after runtime replacement or authorize an old asynchronous result. Resource
authorization/freshness and event/command lifecycles remain mandatory even with an
equal key. Cache leases, eviction, browser transport and DOM/style reuse are
subsequent work; no cache hit path is enabled by these APIs.

## Bounded native artifact storage and leases

The existing `content_cache::ContentCache` now bounds its serialized native
artifacts by entry count and charged bytes. Defaults are 64 owners and 32 MiB;
`with_limits(ContentCacheLimits)` supplies positive explicit limits. `insert`
returns `Result` and verifies format, body hash, dev/prod sidecar compatibility
and the sidecar's hash before admission. Existing callers must handle rejection.
Metadata encoding is independently capped at 1 MiB with a bounded writer.
Payload vectors and deserialized metadata are compacted, so small values do not
keep former oversized source-buffer capacity alive.

Charges include exact compact payload length, encoded metadata/sidecars, duplicate
lookup/recency key metadata and entry/key structural overhead. This is the store's
logical ownership budget, not a process RSS or compilation-scratch limit. Processing
admission still owns in-flight compilation, decoded trees, host allocations and
allocator overhead. Unrelated native handle registries are not charged by this store.

Successful `get` and `lease` requests promote LRU order. Admission plans all victims
before removing any, evicts only unleased owners and rejects oversize or pinned
pressure with `cem.cc.capacity`. Rejected admission leaves entries and recency
intact; diagnostics change only bounded counters. Identical records deduplicate
without adding charge. A different record/policy at a leased key is rejected with
`cem.cc.cache_pinned` until all its leases release. Byte/count limits apply to all
retained owners, including old generations still held by leases.

`ContentCacheLease` shares one immutable owner and its optional sidecar. Cloning
adds a pin, not another payload charge; the last release allows normal eviction.
Every `artifact(current_policy)` / `sidecar(current_policy)` access checks live
runtime generation and current policy. Sidecars are owned by their artifact,
not an unbounded global table, and disappear when that owner is retired/released.
The borrowed compatibility APIs cannot survive a mutable store operation; use a
lease when ownership crosses that boundary. Lease values are immutable borrowed
bytes; explicitly copied/reloaded values belong to the consumer's separate budget.

`reset_runtime` advances the generation and clears resident lookup/recency state.
Old leases immediately reject reads with `cem.cc.runtime_replaced`, but their
bytes remain charged until release, including release from another native thread.
Dropping the cache also invalidates outstanding leases. Async producers capture the opaque
`runtime()` owner ticket **before** work and use `insert_for_runtime` and
`lease_for_runtime`; late old-generation requests are rejected before admission
or lookup, even if identical bytes are resident again. Tickets bind a weak native
owner identity and generation; another cache at the same counter value cannot
accept them, and tickets keep no store/payload owner alive. Numeric generations
in stats are diagnostics, not admission credentials. Call reset when replacing
the runtime; it is not inferred from portable content hashes. Publication still
requires current source/resource freshness and the owner's revision admission.

`stats()` exposes resident/retained owner counts, bytes, lease count, generation
and saturating hit/miss/eviction/rejection/invalidation counters. Concurrent lease
release can make separate counter samples nontransactional; there is no payload or
history log. Real dev/prod compiled-template reload fixtures verify exact rendering,
diagnostics and native source/output frames through this store. These native
storage APIs do not yet enable dependency-to-artifact lookup in CLI/WASM/browser
compilation consumers or bypass evaluation/resource/event lifecycles. That wiring
must use the shared dependency identities and fresh context before enabling hits.

## WASM memory admission and headroom

Shared `runtime_memory` owns per-runtime limits and RAII workspace reservations.
The WASM allocator wrapper measures **requested live allocation bytes** and their
peak, including realloc/free; it never returns null to enforce policy. Each WASM
instance independently observes its linear-memory capacity. These figures overlap
and are not summed: admission conservatively uses the larger observation plus
outstanding workspace reservations. Allocator overhead/fragmentation, JS heaps,
DOM, host buffers and other WASM instances are not totalled into this report.
Native hosts may use the same policy with their own trusted allocator samples.

Defaults are a 768 MiB soft admission threshold, 1 GiB hard budget, 64 MiB of
withheld recovery headroom and at most 64 concurrent admitted operations. Trusted
embedders can call `configureRuntimeMemoryBudget(JSON.stringify({
softLimitBytes, hardLimitBytes, recoveryHeadroomBytes, maxConcurrentOperations
}))`; the complete configuration is validated and cannot change while work holds
reservations. `runtimeMemoryStatus()` returns small counters/configuration only.
No authored document, query or browser byte-count claim can set these counters.
The same exports are linked into CEM-QL and the command-service WASM runtime.

New guarded work reserves eight times its actual input byte length plus 64 KiB
of workspace before parsing. This is a conservative admission estimate, not a
proof of its eventual allocation requirement. Reservations are bounded, released
on return/error/drop and held across asynchronous command execution. Soft
pressure stops new work; already admitted work can finish while below the hard
work ceiling (`hardLimitBytes - recoveryHeadroomBytes`). Operation-control safe
points observe actual runtime pressure and deliver the existing root memory
failure/unwinding contract at that ceiling. Earlier cancellation/failure wins.
No automatic command/action retry or runtime replacement is performed.

Compile/render, retained CSS/document/native input and command-service entrypoints
perform native admission. Disposal/cancellation and status/configuration remain callable under pressure;
taking an already-produced value artifact is also ungated. Export routines still
obey their existing operation-control checks. Bounded committed-checkpoint
transport has exclusive recovery admission after ordinary work drains. Host-to-WASM
ABI input copies happen before Rust entrypoint admission; initial instantiation,
allocations between safe points and APIs without an admission wrapper can still
exceed a forecast. This is proactive cooperative control, not an allocator hard
cap, process-RSS bound, or a guarantee that fatal WASM OOM can be caught. Headroom
is withheld from ordinary admission; host-owned checkpoint transport is available,
with coordinated runtime replacement next. Freeing data reduces live bytes, but does not
shrink WASM capacity or automatically clear capacity pressure.

Native policy/allocation tests: `cargo test -p cem-ml --test runtime_memory` and
`cargo test -p cem-ml --lib operation_control::`. After the CEM-QL WASM build, run
`node packages/cem_ql/tests/runtime-memory-wasm.mjs` for actual allocator/capacity,
pressure refusal, cleanup and subsequent admission checks.

`committed_checkpoint::seal` / `sealCommittedCheckpoint` validate bounded portable
transport against an independent trusted publication receipt. Shared
`cem-elements` host storage keeps its bytes outside WASM. Island execution still
requires lifecycle admission; no automatic restart or action replay is provided.
See [portable checkpoints](../../docs/committed-checkpoints.md). Run
`cargo test -p cem-ml --test committed_checkpoint` and, after building WASM,
`node packages/cem_ql/tests/committed-checkpoint-wasm.mjs`.

## Versioned deployment-map source validation and AST JSON exports

Explicit module-map v1/v2/v3 identities use shared native deployment-map decoding
and semantic validation during Validate/Check and source tooling. Selected schema
and embedded version must agree; MIME-only module maps select v1. Generic JSON
remains inert. Exact bare identities, allowed typed assets, extension agreement,
identity collisions and v3 declared module edges are checked without resource
reads. Pair matching, destination containment and asset reads remain planning
operations. Root-scope alias maps and ordinary browser import maps are separate.
Diagnostics use original native JSON key/value ranges and source maps.

The lifecycle success fixtures load each version's canonical schema-package
`examples/source.module-map.json` through the JSON syntax adapter, preserving
original bytes and document identity. Native refusal fixtures cover all selected
versus embedded version mismatches, missing/unsupported declarations, v1/v2
string versus v3 typed import shapes, version-specific resources and declared
edges. They assert actual diagnostics, field paths and original source ranges;
semantic refusal retains the syntax AST for inspection. A MIME-only map selects
v1, while generic JSON stays inert even when its `$schema` names an invalid v3
map. These tests change no production admission or execution policy. Run
`cargo test -p cem-ml --lib module_map` and the same cohort with
`--no-default-features`.

`projection::ast_json` and explicit AST conversion export the registered semantic
document envelope, sharing native node projection with DOM JSON. Namespaces,
text `data`, attribute `valueNodes`, native references, original ranges and maps
survive; non-owning reference targets are never traversed. CEMT presentation
subjects and binary/runtime ownership retain their own contracts. Structural
source validation does not decode an executable owner or grant source navigation.

## Owned source-operation provenance

Native CEM-ML Parse/Inspect presentations retain typed writer spans with an
original-source witness through common command publication. A witness binds the
source URI, original bytes and final writer bytes; it is not serialized into the
result or used as an AST transport. Only named native boundaries with valid
original UTF-8 and output byte ranges become navigation references. Anonymous
IDs, formatter-module origins, foreign buffers and JSON source fields cannot
supply endpoints. Explicit JSON inspection and binary exports remain separate.

The common result exposes bounded `sourceMaps` references owned by the published
artifact, with an optional representative `outputRange`. Repeated native source
ranges are coalesced; counts disclose truncation, and each published output keeps
its primary reference within the negotiated cap. These ranges do not assert
linear character equality through formatting or escaping. Native CEM, XML, HTML, JSON, YAML, CSV, CSS, XPath and Markdown formatter
views bind their original source URI before package formatting. CEM-to-HTML/XML
conversion carries typed final writer bytes and spans through direct pipelines,
registered converters and native fallback. JSON/YAML conversion and CSV-to-XML
retain their original input identity. Markdown tokens with native source maps
also retain ranges when they lack an explicit output span. Generated layout
stays unmapped; an existing foreign/module boundary is never rebound to the
input. Terminal coloring and HTML escaping use actual final output byte ranges.
SCSS expansion keeps its dedicated module-origin frames: only a native frame
whose module URI matches the admitted input can supply that input's coordinates.
Foreign module offsets are never relabeled as root-input ranges.
Public/debug JSON fields never supply the writer witness.

Studio projects these references through its existing declarative results and
source-selection controls. Saved edits invalidate captured revisions; rerun
before following old links. Failed or stale publication exports no artifact
ownership. Entered Login passwords and one-time codes still block recovery until
cleared; automatic recovery stays disabled and the existing 4 MiB and node/time
limits are unchanged.

## Passive projection-source imports

The shared native import boundary accepts explicit DOM, AST and event projection
JSON/binary identities. It retains original bytes and native JSON owners, maps
source operations to actual artifact bytes and never adopts reported source maps,
schema IDs or reference context/target handles. DOM/AST become common semantic
CEM trees; events become ordered literal `event` records in the events namespace,
including passive expression/prelude fields. This does not replay event streams
or restore execution, resources, credentials or recovery authority.

Admission bounds are 16 MiB, 65,536 materialized nodes/JSON values and 64 levels;
string imports retain their smaller existing limits. Binary v1/v2 require complete
record/UTF-8/tag/count/range/ID and tree validation, exact consumption and matching
family/schema/MIME. Sealed JSON envelopes require original payload, per-chunk and
whole BLAKE3 hashes, contiguous record boundaries and matching ownership links.
Metadata-only envelopes, malformed graphs, semantic error nodes and trailing
bytes are refused. Bare binary provides structural integrity, not a checksum.
JSON is UTF-8. Refusals retain the actual input URI and never fall back to CEM
text tokenization. Reported offsets, including chunk source-map deltas, remain
data. Conversion supports passive CEM/XML/HTML and explicit DOM/AST JSON/binary
exports, without a serialized AST handoff or executable normalized-event replay.

The ordinary `native-projections` Studio project exercises JSON inputs through
existing declarative editor, command/result and source-selection controls.
Binary Studio files stay read-only; explicit saved configurations use the owned native command admission described below. Entered Login passwords/one-time codes continue to block
recovery until cleared; automatic recovery and all existing budgets are unchanged.

### Owned binary Studio commands

An explicit saved configuration can use a locally owned binary source after
native invocation admission validates its original bytes, declared/effective
identity and supported input codec. Informational `binaryInputs` facts bind the
input ID, URI, size and SHA-256; they contain no runtime handles and grant no
execution authority. Common operation preparation independently rechecks the
snapshot digest, scope, identity and full payload before execution. Unsupported
compiled identities, malformed bytes and text relabeling are refused.

A binary file selection has no implicit command or text editor. A selected
configuration exposes Run, Reload, exact downloads and existing results through
production declarative CEM controls. Source edits, ad-hoc Parse/Inspect, command
replacement and text-coordinate navigation are unavailable in that view; edit
the command resource through the project tree. External changes require reload,
and changes during execution mark results stale. Source byte counts refer to
actual bytes, never a decoded placeholder. All existing source/resource-set and
recovery budgets, credential refusals and timeouts remain unchanged.
