# cem-ml-transform-cem-ql

`cem-ml-transform-cem-ql` is the Rust adapter that connects CEM-ML's stable
transform-template contract to CEM-QL compilation, evaluation, and native CEMT
rendering.

## Public boundary

The crate sits above both [`cem-ml`](../cem_ml/README.md) and
[`cem-ql`](../cem_ql/README.md). CEM-ML retains transformation lifecycle,
artifact, schema, source-map, and adapter ownership; CEM-QL retains expression,
template, render-plan, and compiled-artifact ownership. Keeping the integration
in this crate prevents a dependency cycle and gives native hosts one explicit
adapter registration boundary.

CEM-document artifacts enter queries through the shared retained CEM tree view,
using their original AST allocation. They support native unary `#` construction
and flat `children`/`attributes` sequences, as lifecycle data inputs do. Query
preparation does not resolve authored references. Node identity follows the
original AST allocation even when a collection exposes it through multiple
structural projections. Native document metadata can be read through
`input.kind`; the former record view's automatic metadata bindings are not
installed by native ingress.

Rust callers needing the former record shape can explicitly call
`cem_document_record_query_stream(document)`. Its element children and attributes
remain array-wrapped records, and those records are not unary `#` operands.
This compatibility function retains the original owner without serialization.
Artifact ingress checks native tree structure and does not fall back to the
record projection. Artifacts lacking original source text retain authored source
maps and URI provenance; the bridge does not reconstruct text or coordinates.
The internal generic-node/target-view interfaces do not adopt new public query
reference type or target-access syntax.

Shared query preparation now accepts `QuerySourceOwner::Cem` alongside
`QuerySourceOwner::Lifecycle`. CEM sources retain the original parser arena and
captured lexical snapshots through the normal CEM pipeline; the input view and
constructed reference targets share that native owner. Capture does not compile
or evaluate source expressions. Hosts can attach those snapshots to a subsequent
explicit reference consumer using their supplied contexts and crossing grants.

`CemQlNativeItemsOwner::source_owner()` exposes this checked owner distinction.
`lifecycle_owner()` now returns `Option`: external sources return their original
lifecycle owner, while CEM sources expose their retained tree/capture through the
source-owner variant. Query adapters read `QueryPreparationRequest.source_owner`
instead of the former mandatory lifecycle field. CSS selector/XPath admission
retains its existing lifecycle contract; this change adds CEM input for CEM-QL.

An explicit consumer can pass `QuerySourceOwner::NamespaceCompleted { source,
completion }` to shared preparation, or to
`cem_ml::query::run_query_with_source_owner(request, owner)`. The retained-owner
runner uses the request's declared identity, scope, query and limits without
loading or reparsing its input bytes. The CEM-QL adapter checks original AST
allocation identity, then binds the completion's selected native roots as `input`.
Root order and an empty forest are preserved; there is no whole-document fallback.
Native axes stay in that forest, completed names stay execution-specific, and the
explicit `source` field retains authored names and original metadata. Unary `#`
preserves these native execution views and their original source owners.

`source_owner()` retains the supplied completion and source tree. This ingress
does not resolve namespace declaration values, create grants or activate scopes.
It exposes ready selected names for query/inspection; evaluating authored captured
expressions still requires their separate lexical handoff and all pending prefix
bindings. Ordinary `run_query` keeps its original loading/capture behavior. CSS
selector and XPath adapters retain their lifecycle-only admission contract.

The adapter supports CEM-native templates, standalone CEM-QL expression
templates, and the [strict typed XSLT 3.0 profile](../../docs/xslt-runtime-lowering.md). It is infrastructure for
hosts and embedders, not an application UI or an alternate query-language
implementation.

XSLT execution uses compiled bundles with retained native document input,
recursive named/matched templates, explicit parameters, modes, and policy-resolved
import/include closures. Legacy version/namespace shortcuts and implicit
parameters are rejected. Separate legacy conversion tools remain available.
The historical adapter type/ID names are retained for host registration;
standard XSLT media types now select this executable adapter.

## Imported document bindings

Lifecycle XML, JSON, YAML and CSV inputs and explicitly encoded JSON documents
enter through `cem_ml::import`. `input` is one retained CEM document node using
`cem_ql::eval::imported_cem_tree`; secondary labels bind the same node capability.
JSON members no longer become query fields or top-level variables, arrays are
not implicitly flattened, and XML no longer exposes an `events` query binding.
Use CEM node navigation or a declared XPath function library. Duplicate keys,
lexical values, node/source identity and native ownership survive import.

For example, a generic-data property is selected through
`seq:where(input.children.children, fn(p) => p.attributes.value == "title")`;
its scalar text is at `.children.children.value`. Explicit numeric/boolean
conversion belongs in the authored query. Named parser inspection and requested
output serialization retain their separate contracts.

## Schema package lifecycle compilation

Install `schema_packages::CemQlSchemaPackageCompiler` explicitly with
`register_cem_ql_schema_package_compiler` when a native host has the runtime
inputs needed for element, attribute, behavior, diagnostic, constraint or
field-contract declaration references. Its preparation callback receives the package/schema identities,
manifest origin, retained schema source and `request.lexical_scopes`, and
returns a fresh `CemQlSchemaDeclarationHost` plus request traversal limits.
Register that source in the host, supply its current expression context and
effective policies, register target owners, and grant directed scope crossings
as required. Call `attach_captured_lexical_scopes` with the saved bindings to
prepare occurrence contexts and policies; existing callbacks remain valid when
they supply their own associations. No context root ID is needed.

Call `cem_ml::real::load_schema_package_manifest_into_context` at the chosen
lifecycle stage. Engine requests containing `schema_package_manifests` use the
same compilation hook on their private enriched context. Ordinary adapter
registration does not install this compiler; source-only loading keeps
unevaluated declaration references pending.

`EngineContext.schema_package_sources` retains the latest valid source arena
by resolved URI and byte revision, together with lexical snapshots from the same
AST allocation. `get_lexical_scopes(uri)` exposes those snapshots and machine
diagnostics. Unchanged reads reuse both; changed bytes replace them together.
Successful descriptor extraction borrows the retained AST. This registry caches
source structure and bindings, with runtime selections supplied per invocation.
Independent context snapshots share immutable source ownership and compile
against their own runtime inputs. An incomplete or failed refresh preserves the
last active schema, converters and artifacts while keeping its candidate model
inspectable. Ready candidates publish together. Replacement ownership is
checked before invoking the compiler; supplying query inputs does not grant
package replacement authority.

Preparation failures retain their diagnostics and receive a hard compilation
error if none was supplied. Invalid traversal limits and a compiler returning a
foreign schema identity also prevent publication. Native expression failures
keep their original diagnostic code and source attribution.

## Input validation lifecycle bindings

An explicitly installed `EngineContext.input_validation_stage` receives
`InputValidationRequest.lexical_scopes` for ordinary CEM inputs and XML inputs
with a ready consuming schema. It shares the source tree's original AST owner
and preserves occurrence bindings after child frames close. Register the source and target owners, grant directed crossings,
and call `attach_captured_lexical_scopes` with current contexts and policies before
running retained validation. Missing runtime inputs remain pending. XML uses its
specialized importer, preserving aliases, entity/CDATA source maps and literal
attributes. Its retained tree also keeps the original native XML owner. Already
parsed XML is reused; custom-schema XML is parsed once. Source-only and other
specialized XML-family validators retain their own paths. Other parser paths
currently supply `None`.

Native engine fixtures cover schema directives, host and wrapping switches,
sibling defaults, namespace rebinding and restoration through CEM and XML.
They verify explicit completion of pending contexts, independent executions over
one saved owner, vendor grants and request/destination traversal limits. Default
namespace/schema bindings and named inline declaration inheritance, child shadowing
and restoration are covered too. Equal authored IDs in different vendors still
require distinct directed grants. Named inline declaration source handles remain
tracked in `docs/todo.md`; retained metadata does not supply runtime inputs.

## Resumable native validation

An installed stage can override `InputValidationStage::start_resumable` and return
`CemQlInputValidationSession::new(owned_request, prepared_host, runtime_inputs)`.
Prepare the host with the original source owner, captured names and occurrence
bindings. Implement `SchemaValidationSessionInputs` to supply current body/local
contexts, loaded destination policy/context, optional explicit MIME hints, public
exports and loaded-owner lexical handoff/crossing grants. The adapter grants no
crossings and does not assign IDs to runtime contexts.

The session discovers URI dependencies only from entered regions. The native
engine coordinator schedules its typed requests on I/O workers, releases the CPU
slot while reads run and resumes the same retained session on CPU. Nested blocked
controls are requested only after their parent region becomes usable. Missing
contexts, denied crossings and failed reads leave validation incomplete without an
inherited-model fallback. Resource correlation, cancellation, import byte limits
and retained payload memory accounting span all rounds.

Return `None` from `start_resumable` for the existing synchronous path. Portable
synchronous engines continue calling `validate`. Registration, parsing and query
preparation remain passive. Native CEM/XML fixtures cover queued nested loads and
retained schema declaration references, wrapping/sibling/prelude controls,
foreign-owner URI bases and structural diagnostics, explicit public parts and
multi-request batch ordering across queue policies. Failed peers remain incomplete
while independently ready peers activate. The deferred syntax decisions stay in
[`docs/todo.md`](../../docs/todo.md).

## Verification

Use Nx for the publishable crate gates:

```bash
yarn nx run cem_ml_transform_cem_ql:lint
yarn nx run cem_ml_transform_cem_ql:test
yarn nx run cem_ml_transform_cem_ql:build
```

The surrounding transformation contract is documented in the
[CEM-ML acceptance criteria](../../docs/cem-ml-ac.md) and the
[CEM-QL implementation design](../../docs/cem-ql-stack-design-impl.md).
