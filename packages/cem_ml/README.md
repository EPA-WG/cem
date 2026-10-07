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
consumption, completion handoff and lifecycle activation remain consumer work.

`NamespaceNameCompletion::binding_namespace_uri` accepts an original declaration
handle and returns its completed URI or pending dependency. It follows captured
default aliases, preserves empty literal resets and rejects foreign owners and
ordinary nodes. An inherited declaration can be outside the selected query forest;
reading its metadata does not expose it through axes or create an expression context.

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
