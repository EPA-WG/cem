# Reference consumer verification

This matrix records the implemented reference boundaries as of 2026-10-06.
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

## Remaining integration gaps

The native syntax, query access, bounded resolution and supported source/value
transport boundaries are implemented. The following adapter work remains
actionable in [todo.md](todo.md#next-three-reload-consumer-items):

1. Attach a later verified lexical sidecar or source bytes to an already decoded
   owner, refreshing only its inspection view. Today missing metadata is exposed
   correctly, and context/grant retries retain the owner; client reload otherwise
   constructs a fresh owner when metadata arrives in another bundle.
2. Connect explicit reload admission to high-level engine and CLI `validate` /
   `check` requests, including their resumable schema/namespace lifecycle. The
   current native `ReloadIngress::validation_request` supports embedding stages;
   the CLI bundle flag is currently on `query`.
3. Expose retained source handles to an explicit WASM lifecycle consumer with
   fresh runtime inputs and grants. The current WASM query API performs inert
   native queries; it does not supply a schema/namespace validation session.

These gaps do not authorize source reparse, destination-default substitution,
serialized live contexts or automatic URL/ID lookup.

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
