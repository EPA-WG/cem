# Reference consumer verification

This matrix records the implemented reference boundaries as of 2026-10-07.
Loading retains authored references; a consumer supplies the evaluation stage,
runtime inputs, effective policies and crossing grants. Query field access and
transport never claim that an authored reference has been evaluated.

| Consumer / boundary | Verified behavior | Native evidence |
| --- | --- | --- |
| CEM source loading | Standalone and attribute slots retain expression, occurrence identity, source maps and captured namespace/schema bindings; targets remain absent. | `cem_ml/tests/node_references.rs`, `cem_ql/tests/lexical_scope_handoff.rs` |
| XML source loading / reload | Explicit CEM expression elements and core `expression-attributes` slots retain native references/general expressions with original entity-aware frames. Unlisted XML strings remain literal, including `{#input}`. | `cem_ql/tests/reference_transport_api.rs`, `cem_ml/tests/reference_syntax_adoption.rs`, `cem_ql/tests/reference_host_adapters.rs` |
| Native QL / shared query ingress | `#` accepts nodes, ordered repetitions, empty sequences and other references. Repeated executions share the decoded arena; query request bytes are not reparsed. Original owners and descendant references remain intact. | `cem_ql/tests/constructor_references.rs`, `cem_ml_transform_cem_ql/tests/reload_ingress.rs` |
| Public target access | `reference` is a node refinement. `.targets_available` distinguishes absent metadata from available empty targets; `.targets` accesses one edge list without evaluation. `same_node` compares occurrences. | `cem_ql/tests/public_reference_contract.rs`, `cem_ql/tests/constructor_references.rs` |
| Schema declaration reuse / input validation | Consumers resolve original declarations or input placements. Reload ingress supplies the same decoded owner and capture. Missing capture is a typed dependency; missing runtime context and absent crossing grants prevent completion. A later explicit context/grant retry leaves source targets unchanged. | `cem_ml_transform_cem_ql/tests/reload_ingress.rs`, `cem_ql/tests/lexical_scope_handoff.rs`, `cem_ml_transform_cem_ql/tests/retained_behaviors.rs` |
| Namespace / schema lifecycle | Saved declarations and pending QName identities remain passive. Runtime completion and selected schemas are execution views, admitted through the existing explicit lifecycle APIs. Query ingress does not manufacture those completions. | `cem_ml_transform_cem_ql/tests/namespace_query_ingress.rs`, `cem_ml/tests/reference_reload_bundle.rs` |
| Executable reload bundle | Versioned debug CEMB plus verified source manifest and passive lexical capture. Single-arena handles are rebound to the decoded allocation once; subsequent queries/validation retain that owner. Missing source bytes do not borrow primary-source coordinates. CEMV is rejected as a source bundle. | `cem_ml/tests/reference_reload_bundle.rs`, `cem_ql/tests/reference_transport_api.rs` |
| Materialized CEMV export | Empty, nested and repeated constructed references survive DAG transport. Executable source references reject in absent/empty/nonempty target states. Cycles, failed streams, limits and cancellation have distinct attributed outcomes; parent backlinks are excluded from cycle checks. | `cem_ql/tests/reference_export_contract.rs`, `cem_ql/tests/native_value_control.rs` |
| CLI query / transport | Explicit bundle admission and primary source ID; original source/capture export; binary CEMV output without a text newline. Export failures retain kind and source map in the report and emit no partial successful bytes. Existing JSON descriptors and textual projections remain explicit compatibility exports. | `cem_ml_cli/tests/reference_transport_cli.rs`, `cem_ml_cli/tests/query_cli.rs` |
| Low-level WASM / worker transport | Source and result handles stay local. Result handles retain the source capture after the source handle is disposed. Workers transfer bundle or CEMV bytes explicitly. Guards report code, kind, original source URI and source map; disposal rejects stale handles. | `cem_ql/tests/reference-transport-wasm.mjs` |
| cem-element DOM relationships | Native attribute targets map to unique produced placements, preserving explicit IDs or generating instance/placement IDs. All slots share bounded traversal; authored source chains require explicit original contexts/grants. Explicit invocation source captures/contexts/grants work in worker/fallback; typed action, popup and submenu endpoints coexist with local names. SSR/Edge/hydration preserves IDs and rejects incomplete publication. | `cem_ql/tests/reference_runtime_resolution/{element_ids,element_lifecycle}.rs`, `cem_ql/tests/native-values-wasm.mjs`, `cem-elements/src/lib/element-reference-{ids,lifecycle}.stories.ts`, `cem-elements/src/lib/element-reference-ids.edge-ssr.{spec,stories}.ts` |
| Granted producer placements | Native snapshots require source and host grants, unique producer placement and bounded work. Private transactions admit prepared producers, then publish the entire group with producer-owned ID reservations. Browser revocation/disconnect clears affected routes; SSR hints grant no authority and hydration/reconnect requires fresh admission. | `cem_ql/tests/reference_runtime_resolution/element_placements.rs`, `cem_ql/tests/native-values-wasm.mjs`, `cem-elements/src/lib/element-placement-authority.spec.ts`, `cem-elements/src/lib/element-placement-coordinator.stories.ts`, `cem-elements/src/lib/element-placement-ssr.edge-ssr.spec.ts`, `cem-elements/src/lib/element-placement-hydration.edge-ssr.stories.ts` |

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
worker fixtures. CI invokes this target explicitly. The adapter checkpoint passed
284 native checks in 34 suites, including eight new adapter cases, plus both
worker fixtures. Explicitly marked XML attribute slots and the XML
expression-element/CDATA path use the same native reference consumer; unlisted
XML attribute values stay literal.

The convenience adapters now supply per-occurrence runtime inputs over original
owner/node handles. Nearest owning overrides apply to descendants, explicitly
pending overrides shadow defaults, and clearing restores inheritance. Replacing
parent configuration invalidates outstanding resource execution; independent
saved query views remain usable.

Host `complete_with_exports` callbacks or local WASM public export selectors
select original declarations before activation. Empty, ambiguous and foreign-owner
exports remain incomplete. Exposure and context bindings create no crossing or
replacement authority. URL parts stay loader conventions, without ID scans.

`ReferenceResourceExecution::query_snapshot` saves the input's latest namespace
completion or a loaded owner's explicitly prepared completion; before preparation
it uses the captured ready name forest. `prepare_loaded_names` and the local WASM
`prepareReferenceResourceNamespaces` explicitly run the shared lifecycle after
destination inputs are supplied. They preserve original capture, separate grants,
request/destination bounds and cumulative preparation accounting. Context changes
discard the current completion and require another explicit preparation.
Saving/querying snapshots performs no import or namespace evaluation. Pending
names remain excluded; earlier views retain owners independently of disposal.

Evidence: `reference_host_adapters.rs` verifies occurrence inheritance and stale
completion, public export admission, separate grants, query lifetime, XML native
attribute consumption, combined enclosed schema/namespace overrides and parsed
block-prelude runtime restoration. `reference_syntax_adoption.rs` checks XML alias
ownership, element-local intent, entity frames, malformed/reserved targets,
quote/comment-aware slot boundaries and bounded nested preludes. The local WASM
worker fixture exercises the same adapter APIs in two heaps. Five additional
native cases cover explicit loaded-name resume, pending occurrence shadowing,
crossing/cancellation/stale-input guards, cumulative work limits and marked XML
reload/snapshot identity and entity diagnostics. Both native reference slots and
general expression slots are consumed after reload, while unlisted attributes
stay literal and target descendants stay authored. Worker reload starts with
independent contexts and grants and preserves entity source spans. Query-name
readiness alone does not establish selected-schema compilation readiness.
Validation now installs current loaded-owner completions for compilation and
restores prior views afterward. Pending declaration names defer selector execution
and model activation, with request/destination metadata work bounds. Six further
native cases verify unprepared/prepared/replaced/denied admission, independent
pending roots, actual compiled declarations, borrowed declaration owners, bounded
name scans and invocation restoration. Both WASM heaps exercise all four admission
states and retain independent saved query views.
The maintained target passes 324 native checks in 36 suites and both real WASM
worker fixtures after element consumer adoption (2026-10-07). Its 12 new native
cases cover preserved/generated IDs, original source attributes/owners, explicit
source lifecycle contexts/grants, repeated slots sharing destination work,
cardinality, mixed values, nested chains, missing/ambiguous placements, conflicts
and cancellation. The dedicated browser story checks native command activation,
typed action endpoints, per-instance IDs, stable rerender and local-name
compatibility in worker and fallback modes. The runtime unit target passes 588
tests; typecheck and lint pass. Actionable follow-ups retain their scenarios in
[todo.md](todo.md#deferred-cem-element-reference-consumption).

The next consumer checkpoint passes 327 native checks in 36 suites, both real
WASM worker fixtures, 588 runtime unit tests, 54 Node Edge/SSR cases and dedicated
browser worker/fallback, provider and hydration stories. Shared source-owner tests
use different `datadom` inputs without authored target mutation. WASM tests verify
missing context/grants, destination accounting across slots, warning/ignore
incompleteness and stale handle disposal. Browser tests cover failed replacement,
async disposal, stable reconnect, popup rebinding/local names, competing submenu
panels/cycles and restored authored/generated IDs. Typecheck and lint pass (the two
existing non-null assertion warnings remain).

## Granted placement checkpoint (2026-10-07)

Eight native cases verify separate source/placement grants, local precedence and
ambiguity, unique external grants, repeated targets, metadata/destination bounds,
ID conflicts, cancellation and transaction-only prepared admission. Both real WASM
heaps exercise committed and prepared metadata, then reject stale/denied snapshots.

The browser host fixtures cover worker/fallback activation, immediate revocation,
disconnect and fresh reconnect, stale group rejection, producer-owned reservations,
ID conflicts, nested prepared producer readiness and activation-time revocation
with rollback that strips detached routes before retry. Five authority unit cases
cover copied snapshots, dependency retention after partial revocation and activation
rollback. Node SSR fixtures retain original source owners, keep plans private until
coordinated commit and export hints without tokens/grants. Hydration preserves IDs
but strips foreign routes before fresh admission; stale hints grant nothing.

The maintained native gate passes 336 checks in 36 suites and both WASM fixtures.
Runtime units pass 594 checks; all 56 Node SSR and 12 Edge browser cases pass.
Typecheck and lint pass with the two existing non-null assertion warnings.
The full browser run passes 412/417: the workflow gallery and three icon-button
cases reproduce with the pre-placement runtime; the single-frame form-isolation
failure does not reproduce in either focused current/baseline run. Their actionable
follow-ups are recorded in [todo.md](todo.md#browser-failures-observed-during-placement-verification).
No aggregate browser pass is claimed.

## Separately deferred work

Enclosed child overrides now use existing host-bound native schema/namespace
properties or wrapping schema elements. XML native attributes require explicit
element-local metadata; ordinary attributes remain literal. Opening block
directives use literal payloads, with native typed prelude payloads conditional
on a separate request. General datatype compilation, executable native attribute
`@type`, conversion and
equality remain the separate datatype workstream. The
[cem-element reference-to-ID mode](cem-element-reference-ids-design.md) now
implements the first native consumer contract. Browser/worker foreign-source lifecycle handoff, popup/submenu endpoints and
dedicated native SSR/Edge/hydration fixtures are implemented. Granted cross-instance placements and popup/menu focus/geometry consumption
are implemented. Additional surface adapters and a future ARIA profile remain actionable in
[todo.md](todo.md#deferred-cem-element-reference-consumption).

The debug CEMB codec is not a stable production binary format. External AST-owner
declaration graphs are not admitted by this single-arena reload bundle. Neither
limitation is hidden by flattening records or inventing context IDs.
