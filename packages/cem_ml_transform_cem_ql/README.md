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

For namespace references, explicitly opt in with
`SchemaValidationSessionInputs::namespace_lifecycle_enabled`. Supply original
pre-declaration inputs/local policy through `namespace_context`; use
`namespace_inspected` to retain the matching original-owner completion for shared
query ingress or later context preparation. Each CPU invocation coordinates native
namespace dependencies, then validates only its ready roots under those completed
names. Missing namespace inputs finish incomplete for caller retry and issue no
host-input wait request. Ready schema URI controls still use the existing I/O
suspension path; loaded-owner preparation, grants and request correlation retain
their existing contracts. Other owners' namespace stages remain explicit in
`prepare_loaded`. Sessions without this opt-in retain their previous behavior.

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

## Executable schema attribute datatypes

The adopted [datatype compilation design](../../docs/cem-datatype-compilation-design.md)
defines the implementation and provenance contracts. Installing
`CemQlSchemaPackageCompiler::with_datatypes` now automatically binds typed attribute
declarations after its callback supplies original executable descriptors.
`with_datatype_discovery` adds literal QName discovery using original namespace
captures, declaring `uses` aliases and explicit exports. Native `@type={#datatype}`
requires one named original type and the usual context, crossing and budget checks.

The callback registers validation, lexical preparation and facet capabilities
explicitly. Names never imply built-in implementations. Binding checks local
facets, original declaring-scope diagnostics and literal defaults before publishing
the package. Incomplete candidates preserve the last active schema/converters/artifacts.
Validation uses the installed consumer for lexical input and authorized retained
node sequences, with separate invalid and incomplete outcomes and no implicit
conversion. Use retained input validation when a capability requires a candidate;
source-less lexical validation cannot manufacture one. The
[pretyped provenance contract](../../docs/cem-datatype-compilation-design.md#adopted-pretyped-attribute-input-provenance)
has native issuance, read-only inspection and explicit combined consumption implemented.
Its extension requires a
sealed native preparation result retaining the original lexical text, item spans,
candidate access and exact binding/invocation identity. Bare typed values, public
preparation reports and canonical exports cannot supply that evidence. Typed rules
and local lexical facets still require fresh validation in the live invocation;
the core also admits an opaque preparation handle through the registered consumer.
Explicit declaration overrides use
the separate host transaction described below.

An embedding can use
`cem_ql::preparation_evidence::AttributePreparationInvocation::new` with an exact
`BoundAttributeFacets`, original `PreparationInput`, runtime and finite limits,
then call `prepare`. Complete preparation returns opaque evidence alongside its
public report. Read-only evidence getters retain original text, source maps,
candidate access, native values and item spans; `verify(&invocation)` checks live
authority. Public reports cannot construct evidence, and issuance runs no typed
rules or local facets and supplies no acceptance verdict. Invocation clones share
remaining allowances. Cancellation, query failure, explicit `close` or dropping
all invocation handles expires authority. Call `close` before input, context,
grants or package publication changes when using this standalone API. The registered
core handoff provides the lifecycle checks described below.

Call `BoundAttributeFacets::validate_pretyped(&invocation, evidence.as_ref(),
facet_context, max_facet_model_bytes)` for explicit combined consumption. It checks
exact live evidence, runs fresh typed rules and enumerations, then applies local
facets to the original text and list count. The inspection phase retains evidence
and a fresh datatype report. Missing, foreign or expired evidence is incomplete;
a present empty list still validates. Consumption calls no preparation, conversion
or serialization capability. All calls share the invocation's remaining budgets,
including inspection, validation and local diagnostics; supplying another runtime
or a new budget is not part of this API. Close the invocation before facet context
changes as well as input, runtime, grant or publication changes.

For the core handoff, keep an `AttributeDatatypeContext` for the current host
context/grants and supply it in `AttributeDatatypeInput::context`. Call the active
`CompiledAttributeDatatype::prepare` with original lexical input, then pass its
handle back as `AttributeDatatypeValue::Prepared`. The engine checks its private
issuer and original binding/source/operation, executes fresh validation and shares
the issuing invocation's budgets. Core CEM-ML retains the opaque native handle
without depending on CEM-QL values. Arbitrary payloads and stale handles remain
incomplete; there is no implicit preparation fallback.

Call `context.advance()` when external context roots or effective grants change,
and `context.close()` when the host invocation ends. Dropping all context owners,
operation cancellation/completion, changed input/facet context and committed
package or datatype override replacement expire preparation authority. Failed
replacement candidates preserve active receipts. Retained old models cannot reuse
retired prepared authority. Reloaded source requires fresh preparation; neither
the handle nor its context identity is serialized.

## Explicit list serialization

List conversion and lexical export are separate requests. An embedding can select
`cem_ql::datatype_shipped::list_serializer` for an original `NameList` or
`WildcardNameList` declaration through
`DatatypeImplementations::select_list_serializer`. Custom lexical formats implement
`NativeListSerializer` and register their scalar item representation and candidate
requirement against the exact original list source. Selected unavailable serializers
keep package candidates incomplete, preserving the previous complete publication.

After compilation, call `ExecutableDatatype::serialize_list` with a typed
`ValidationInput`, current `ValidationRuntime` and finite `SerializationLimits`.
All effective item/list restrictions run before serialization. Use `result.text`
only when `result.accepted == Some(true)`. Shipped serializers preserve item
spellings, order and duplicates, separating items with one ASCII space; original
token views, source maps and lexical spans remain unchanged. Scalar result types,
automatic attribute input, conversion and native document presentation stay on
their existing APIs. See the [list export contract](../../docs/cem-datatype-compilation-design.md#explicit-list-serialization-adopted-2026-10-09).

## Executable schema function bindings

`CemQlSchemaPackageCompiler::with_scalar_datatypes` installs the coordinated
function/datatype path. Its callback returns a `ScalarPackagePlan` with original
`ValueContractSource` owners and declaring aliases, explicit public exports,
`FunctionAdmission` entries, datatype roots and exact implementation registrations.
Each admission names an original behavior and a `FunctionProfile`: validation,
conversion, equality or constant preparation. Diagnostic validation uses the
validation profile with its explicit `LegacyAcceptance` mapping.

The compiler assembles admitted behavior collections before forward function
lookup, checks literal or native singleton selections and signatures, then
registers executable queries. Set up reference contexts and directed grants in
the compiler's initial host callback. Function and datatype compilation share a
finite allowance; attribute activation receives the remainder. The model retains
its `FunctionBindings` snapshot, and successful registration clears only the
matching original behavior's native readiness guard. Unadmitted slots remain
pending. Failed attempts preserve the previous complete schema, bindings,
converters and artifacts; retries use the retained source and current context.

This builder replaces a preceding datatype-only builder, and a later
`with_datatypes`/`with_datatype_discovery` replaces it. No ordinary adapter
installation, source execution label or matching function name enables this path.
See the [scalar composition contract](../../docs/cem-scalar-composition-design.md)
and [package fixtures](tests/schema_package_lifecycle/functions.rs).


## Whole-list inheritance

Use `@list-base` to derive a complete list contract:

```cem
{type @name=names @kind=list @base=identifier}
{type @name=pair @list-base=names @min-items=2 @max-items=2}
```

The host registers the root list and scalar item capabilities. `pair` inherits
them without another registration. A native `@list-base={#selectedList}` must
resolve to one authorized list declaration. `kind` may be omitted or `list`;
`base` and `list-base` cannot be combined. Root list `base` retains item semantics.

Derived lists share the exact item contract, tokenizer, lexical preparer and
facet profile. Bounds intersect and rules accumulate; each item is validated once.
Item enumerations, token provenance and sealed pretyped consumption remain intact.
Converters and list serializers inherit unless explicitly replaced by compatible
host registrations. Tokenizers and preparers support explicit checked replacement
as described below; checked facet replacement retains every ancestor profile. Missing ancestor
capabilities keep package activation pending.

Explicit override grants may name an original `list-base` slot to rebind it;
unlisted edges remain pinned. The transaction recompiles every affected derivative
before publishing. See the [whole-list design](../../docs/cem-datatype-compilation-design.md#whole-list-inheritance-adopted-and-implemented-2026-10-09).

## Checked inherited lexical capabilities

An original derived datatype may opt into
`PreparationBinding::CheckedReplacement(registered_preparer)`. A whole-list
implementation may use `TokenizerBinding::CheckedReplacement(registered_tokenizer)`.
Ordinary `Ready` selection continues to reject implicit inherited replacement.

The runtime checks every ancestor on the same original input before executing the
selected implementation. Preparers must produce the same immutable primitive
representation; tokenizers must produce exactly the same decoded spans. A child
may reject more inputs, but cannot widen base admission or change the prepared
value. Base failure stops the child, and no earlier result is used as fallback.
Each callback retains its original datatype owner and candidate requirements.

Shared preparation/input/diagnostic budgets include ancestor checks. The new
`TokenizationLimits::max_tokenizers` field bounds tokenizer invocations (default
256); use `..Default::default()` in limit literals when not overriding it.
Sealed native preparation records successful checks once; pretyped consumers
reuse the prepared value and revalidate all effective restrictions. Defaults and
package publication use the same path. See the [checked replacement contract](../../docs/cem-datatype-compilation-design.md#checked-lexical-capability-replacement-adopted-and-implemented-2026-10-09)
for exact representation, provenance and budget rules.

## Checked inherited facet profiles

Use `FacetProfileBinding::CheckedReplacement(registered_profile)` to select a
derived profile while retaining every ancestor's original registration and
acceptance checks. All families must use the same representation. Ordinary `Ready`
selection still rejects inherited replacement, and missing ancestor profiles keep
readiness pending.

Local fields must compile under every profile. A URI-to-string replacement keeps
URI admission, but a local `uriHosts` field makes that combination invalid because
the string family does not support it. No field is silently discarded. The
composite contract checks the original input oldest-first and stops on rejection
or incomplete work. List profiles use the prepared count; node profiles never
atomize. `facet_profile()` returns the selected registration, while
`facet_profiles()` (or bound facets' `profiles()`) retains the complete sequence.

Model and lexical-input byte limits cover the complete profile chain. Sealed
consumption revalidates it each time; defaults and package readiness use the same
path. See the [facet replacement contract](../../docs/cem-datatype-compilation-design.md#checked-inherited-facet-profile-replacement-adopted-and-implemented-2026-10-09).

## Retained enumeration constants

A scalar datatype may use `{constant @value="In progress"}` children instead of
whitespace-token `@values`. Each complete decoded `@value` is one interpreter
input; empty strings and embedded whitespace are preserved. Mixing both forms
in one declaration is invalid. Native child references may select original
retained constant declarations under the current host's scope grants and limits.

Constant interpretation and equality still require explicit registration.
Preparation checks one immutable scalar against the datatype's rules and
inherited vocabularies. Original constant/value owners, decoded spans and
interpreter/equality bindings survive inheritance. Source/query interpreters use
the same fixed roles, with the original `@value` as candidate. All constants share
compilation limits; `max_retained_value_bytes` separately bounds retained output
text. Incomplete or invalid preparation preserves the active package.

See the [retained constant contract](../../docs/cem-datatype-compilation-design.md#retained-scalar-enumeration-constants-adopted-and-implemented-2026-10-09)
for source admission, reference selection and budget details.

## Typed-only external attribute values

Hosts may register `FacetFamily::TypedScalar(primitive)` or
`FacetFamily::TypedList(primitive)` for original scalar/list declarations. Every
ancestor and list item must also admit typed ingress; lexical profiles, preparers
and tokenizers cannot be bypassed. Scalar local value facets and literal defaults
are rejected. Lists additionally allow count facets. Typed datatype rules,
enumerations and inherited bounds still run.

For a bound native consumer, use `ExternalTypedProducer::new`. For an active core
contract, obtain `typed_admission()` and pass it to
`ExternalTypedProducer::from_native`. Produce an explicitly present immutable
sequence, then pass its `native_handle()` as `AttributeDatatypeValue::ExternalTyped`
under a live `AttributeDatatypeContext`. An absent sequence differs from an empty
list. The source attribute is attribution/context, not a claim of lexical origin.

Every use validates afresh. Closing a producer expires its values; committed
package replacement expires its core admission. A live external value may enter
a fresh live context without pretending to be a preparation receipt. Opaque
payload claims, foreign bindings and serialized handles confer no authority.
See the [typed-only contract](../../docs/cem-datatype-compilation-design.md#typed-only-external-attribute-admission-adopted-and-implemented-2026-10-09)
for the complete omitted-field, representation, budget and lifecycle rules.

## Explicit datatype overrides

An embedding that owns a ready datatype model and its admitted name catalog can
use `cem_ql::datatype_overrides::DatatypeOverrideRegistry`. The host explicitly
supplies the public consuming scope/name, exact expected and replacement sources,
and any original inheritance/list-item dependency slots to rebind. The replacement
must already be present in the original catalog; its implementations and crossing
grants are supplied independently.

```rust,ignore
let mut overrides = DatatypeOverrideRegistry::new(catalog, active_model)?;
let grant = overrides.authorize(&request)?; // embedding host's authority decision
let prepared = overrides.prepare(&[request], &[grant], &host, options)?;
overrides.activate(&mut host, prepared)?;
let ready_model = overrides.model().clone();
```

Preparation builds a private catalog, host, datatype graph and attribute model.
Publication checks the generation, current operation and exact host input snapshot.
Failed or stale candidates preserve the preceding publication. Native selections
stay pinned unless an exact datatype dependency slot is included in the request;
literal attribute consumers see the new public binding after activation. Continue
the same registry for later overrides so its original dependency policy survives.

The ready model can enter the embedding's existing package readiness/publication
boundary. Ordinary compiler registration and package manifests do not issue these
grants. No authored override syntax is introduced. Inherited lexical replacement
requires the separate checked capability selection above; facet replacement remains
deferred. See the [override contract](../../docs/cem-datatype-compilation-design.md#explicit-declaration-overrides-adopted-2026-10-09)
and [native fixtures](../cem_ql/tests/datatype_validation/overrides.rs).
