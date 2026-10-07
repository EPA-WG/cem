# Reference consumer verification

This matrix records the implemented reference boundaries as of 2026-10-07.
Loading retains authored references; a consumer supplies the evaluation stage,
runtime inputs, effective policies and crossing grants. Query field access and
transport never claim that an authored reference has been evaluated.

| Consumer / boundary | Verified behavior | Native evidence |
| --- | --- | --- |
| CEM source loading | Standalone and attribute slots retain expression, occurrence identity, source maps and captured namespace/schema bindings; targets remain absent. | `cem_ml/tests/node_references.rs`, `cem_ql/tests/lexical_scope_handoff.rs` |
| XML source loading / reload | Explicit CEM expression elements retain native references. Ordinary XML attribute strings remain literal, including `{#input}`. | `cem_ql/tests/reference_transport_api.rs`, `cem_ml/tests/pending_namespace_names.rs` |
| Native QL / shared query ingress | `#` accepts nodes, ordered repetitions, empty sequences and other references. Repeated executions share the decoded arena; query request bytes are not reparsed. Original owners and descendant references remain intact. | `cem_ql/tests/constructor_references.rs`, `cem_ml_transform_cem_ql/tests/reload_ingress.rs` |
| Public target access | `reference` is a node refinement. `.targets_available` distinguishes absent metadata from available empty targets; `.targets` accesses one edge list without evaluation. `same_node` compares occurrences. | `cem_ql/tests/public_reference_contract.rs`, `cem_ql/tests/constructor_references.rs` |
| Schema declaration reuse / input validation | Consumers resolve original declarations or input placements. Reload ingress supplies the same decoded owner and capture. Missing capture is a typed dependency; missing runtime context and absent crossing grants prevent completion. A later explicit context/grant retry leaves source targets unchanged. | `cem_ml_transform_cem_ql/tests/reload_ingress.rs`, `cem_ql/tests/lexical_scope_handoff.rs`, `cem_ml_transform_cem_ql/tests/retained_behaviors.rs` |
| Namespace / schema lifecycle | Saved declarations and pending QName identities remain passive. Runtime completion and selected schemas are execution views, admitted through the existing explicit lifecycle APIs. Query ingress does not manufacture those completions. | `cem_ml_transform_cem_ql/tests/namespace_query_ingress.rs`, `cem_ml/tests/reference_reload_bundle.rs` |
| Executable reload bundle | Versioned debug CEMB plus verified source manifest and passive lexical capture. Single-arena handles are rebound to the decoded allocation once; subsequent queries/validation retain that owner. Missing source bytes do not borrow primary-source coordinates. CEMV is rejected as a source bundle. | `cem_ml/tests/reference_reload_bundle.rs`, `cem_ql/tests/reference_transport_api.rs` |
| Materialized CEMV export | Empty, nested and repeated constructed references survive DAG transport. Executable source references reject in absent/empty/nonempty target states. Cycles, failed streams, limits and cancellation have distinct attributed outcomes; parent backlinks are excluded from cycle checks. | `cem_ql/tests/reference_export_contract.rs`, `cem_ql/tests/native_value_control.rs` |
| CLI query / transport | Explicit bundle admission and primary source ID; original source/capture export; binary CEMV output without a text newline. Export failures retain kind and source map in the report and emit no partial successful bytes. Existing JSON descriptors and textual projections remain explicit compatibility exports. | `cem_ml_cli/tests/reference_transport_cli.rs`, `cem_ml_cli/tests/query_cli.rs` |
| Low-level WASM / worker transport | Source and result handles stay local. Result handles retain the source capture after the source handle is disposed. Workers transfer bundle or CEMV bytes explicitly. Guards report code, kind, original source URI and source map; disposal rejects stale handles. | `cem_ql/tests/reference-transport-wasm.mjs` |

## Reload lifecycle adoption (2026-10-07)

Late `RetainedReferenceSource::attach_bundle` verifies the payload and complete
source identities before atomically filling missing lexical capture/source bytes.
It retains the arena and existing capture identities; older query/session clones
retain their previous view. A retry explicitly prepares a new view/session.
`reference_reload_attachment.rs` and `reference_lifecycle_api.rs` cover attachment,
failed replacement and retained execution independence.

Engine `EngineContext.reload_validation_sources` admits prepared owners through
host code. CLI `validate`/`check --reload-bundle` provides explicit admission and
optional `--reload-source-id`; configs and request JSON cannot construct native
owners or grants. The existing native resumable stage receives the exact owner
and capture, coordinating namespace completion and schema URI reads with CPU
release during I/O. Without the needed capture/consumer, governed validation
stays incomplete and avoids inherited-schema fallback. Evidence:
`reload_engine_validation.rs`, `reload_validation_cli.rs`,
`resumable_schema_validation.rs` and `resumable_input_validation.rs`.

`api::reference_lifecycle::ReferenceValidationSession` connects retained input and
schema sources to native schema compilation, namespace completion and structural
validation. Low-level WASM exposes local sessions, fresh context bindings from
native result handles, policy bounds and directed host grants. Each run uses fresh
scopes; its control report contains completion, failure, typed dependencies and
attributed diagnostics. Neither bindings nor grants are serialized into bundles.
`reference_lifecycle_api.rs` and `reference-transport-wasm.mjs` verify pending/empty,
foreign target denial/grant, bounded nested references and independent disposal.

## Host-driven resource and snapshot adapters (2026-10-07)

The native resumable coordinator now lives in
`cem_ql::schema_references::validation_session`; the engine bridge re-exports its
existing API. Engine I/O scheduling and CPU release remain unchanged. The local
`ReferenceResourceExecution` freezes requesting inputs, yields original attributed
URI requests, stages each correlated completion through the same importer once,
and resumes only after the full batch settles. URL resolution/public-part
interpretation remains in the ML loader, separately from CEM-QL reference evaluation. The completing host receives a
retained source over that exact imported owner. Loaded contexts and directed
crossings require separate host calls; neither bytes nor bindings establish them.
Unknown/repeated completion, context or authority replacement, cancellation and
bounds prevent activation. Resource counts and byte charges persist across rounds;
the selected consumers still apply request/destination traversal bounds.

`ReferenceValidationSession::query_snapshot` prepares namespace names independently
of schema validation readiness. `ReferenceQuerySnapshot` queries the same native
`NamespaceQueryTree` over selected ready roots. Pending regions remain excluded,
including an entirely empty ready forest. The immutable view retains the original
arena, authored descendants and its completed names. Later context replacement,
source/session disposal or disposal of another snapshot cannot change its results.
Result nodes keep their completed view alive after snapshot disposal.

Evidence: `reference_resource_session.rs` checks native staging, multiple-resource
correlation, separate contexts/grants, failed transport, cancellation, stale parent
inputs and bounded work. Its declaration reuse case binds nodes queried from the
returned loaded source, proving that execution uses the same imported owner.
`reference_query_snapshot.rs` checks pending/empty and selected forests, independent
completed namespaces, authored reference children and retained result lifetime.
`reference-transport-wasm.mjs` runs resource and snapshot examples in two heaps.

Run `yarn nx run cem_ql:test:reference-consumers`. This maintained target builds
WASM, runs the listed native source/query/lifecycle/transport suites in ML, QL,
the engine bridge and CLI, then runs both reference-transport and native-value
worker fixtures. CI invokes this target explicitly. The final local run passes
284 native checks in 34 suites, including eight new adapter cases, plus both
worker fixtures. Ordinary XML attributes remain
literal; the XML expression-element/CDATA path uses the native reference consumer.

The convenience adapter currently supplies one context per registered owner;
individual occurrence overrides and snapshots of the resource execution itself
are follow-ups, rather than prerequisites for source/query reference semantics.
It has no URL public-part export callback: those requests fail with an attributed
`cem.schema.public_part_unavailable` outcome. The native loader's explicit public
exports hook remains available to embedding hosts; no implicit ID scan is added.
These adapter extensions are actionable with adjacent scenarios in
[todo.md](todo.md#next-three-reference-host-adapter-follow-ups).

## Separately deferred work

The exact enclosed child scope override syntax remains in
[roadmap.md](../roadmap.md). XML attribute expression recognition requires its
own authoring/admission decision; ordinary attributes remain literal. General
datatype compilation, executable native attribute `@type`, conversion and
equality remain the separate datatype workstream. cem-element reference-to-ID
projection and interaction conveniences remain deferred until the ML reference
contract is complete; their actionable items and scenarios remain in
[todo.md](todo.md#deferred-cem-element-reference-consumption).

The debug CEMB codec is not a stable production binary format. External AST-owner
declaration graphs are not admitted by this single-arena reload bundle. Neither
limitation is hidden by flattening records or inventing context IDs.
