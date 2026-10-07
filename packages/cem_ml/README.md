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

## Native document I/O

`import::import_data_bytes` resolves external content types into retained CEM
AST documents. XML, JSON, YAML and CSV semantics belong to this boundary;
query/render consumers receive native nodes.

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
