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
automatic runtime preparation remains pending.

`import::import_xml_ast_with_lexical_scopes(document, schema)` captures standalone
XML reference occurrences during the existing import pass. The returned
`ScopedXmlCemImport` contains the scoped original AST, semantic metadata and event
correspondence; pass its owner and semantics to `RetainedCemTree::from_shared`.
The XML parser's expanded namespace names select CEM schema behavior, including
authored aliases and local namespace declarations. Capture preserves entity/CDATA
payload provenance and final schema diagnostics. XML attributes remain literals.
The same explicit QL lifecycle handoff consumes these associations; automatic
runtime preparation remains pending.

`EngineContext.schema_package_sources` captures these bindings alongside each
valid package source tree. Its `get_lexical_scopes(uri)` accessor and the installed
compiler's `SchemaPackageCompilationRequest.lexical_scopes` share the same original
AST owner. Unchanged reads reuse both; changed sources replace both together.
Descriptor extraction borrows that tree, while compiler callbacks supply current
contexts, policies and grants through the explicit QL handoff. Machine diagnostics
remain inspectable and do not replace the existing package publication gates.
Ordinary engine input loading still needs this capture and lifecycle handoff.

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
