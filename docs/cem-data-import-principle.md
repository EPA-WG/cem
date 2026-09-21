# External data enters through CEM AST import

This is the normative external-data boundary, directed by the user on
2026-09-17. It applies to XML, JSON, YAML, CSV and future document formats.

External document syntax is parsed, decoded and mapped into a retained typed
CEM tree at the import boundary. Downstream CEM-QL, XPath, transformations and
rendering consume that tree through shared node capabilities. They must not
resolve source-format semantics, inspect a format-specific parser AST, or
introduce a second JSON/YAML/CSV/XML evaluation path.

## Responsibilities

- Import owns format selection, parser invocation, accepted syntax, semantic
  decoding, mapping vocabulary, duplicate/null/escaping policy, original source
  ownership, source coordinates and import limits. Format registries and
  lifecycle loaders may select an importer; they must not implement another
  parser or mapping in the consumer.
- The retained CEM tree owns stable nodes, semantic metadata and source
  retention. Source-oriented AST fields remain available for inspection.
- XPath's shared node view owns logical navigation and XPath node semantics,
  using decoded names and values supplied by import. It does not test JSON
  element names to infer maps or apply XML decoding to YAML/CSV/JSON strings.
- Standard parsing functions delegate source ingestion to
  import and enforce their specified result/error contracts at their API
  boundary. A tree mapping is not an implicit native map/array conversion.
- Transport supplies bytes, media type, source identity and lifecycle control
  to import. Response records, DOM-to-record projections and serialized ASTs
  cannot substitute for the retained tree capability.

Selecting an import vocabulary is explicit. The existing generic-data mapping
and standard JSON-to-XML mapping are both legitimate import choices. Authored
queries naturally refer to the selected vocabulary; this does not authorize
format-specific branches in the evaluator.

Explicit JSON API/control envelopes, manifests, query-language literals and
requested export serialization are distinct boundaries. They remain allowed
when named and documented. They must not be used to smuggle an imported runtime
AST or document through generic records. This rule does not require replacing
the JSON wire protocol for compiler diagnostics or scalar host parameters.

## Current implementation and audit

`cem_ml::import` owns the native XML/JSON/YAML/CSV data-reader parsing and mapping.
Its typed `import_string` API also serves XPath `parse-xml`, `json-to-xml` and
the explicit `urn:cem:import` CSV/YAML functions. The string profile owns BOMs,
XML declaration handling, JSON codepoint/duplicate/escape policy and URI metadata.
Typed import failures preserve categories and diagnostics; XPath maps them to
standard function errors without decoding documents or reading error prose.
See the [bounded parsing and recovery profile](xslt-runtime-lowering.md#standard-parsing-and-recovery).
`parser::tree::RetainedCemTree` preserves source-oriented CEM nodes and provides
a format-independent semantic view. CEM-QL retains that capability; named XPath
functions accept imported native nodes, and XPath uses one node implementation.
The XML compatibility constructors delegate to import before evaluation.

Structural inspection uses `projection::cem_tree_inspection` over the same
`Arc<RetainedCemTree>`. Its typed AST-vocabulary rows preserve source arena IDs,
node kinds, expanded names, exact source CEM values and source-map stacks; the
stream retains its owner through formatter/colorizer/writer stages. Inspection
does not traverse external parser ASTs or decode source syntax. CDATA, empty
CDATA and whitespace remain separate source nodes even where XPath coalesces
or omits them. `source_node_range` exposes their original coordinates without
semantic canonicalization. This is structural presentation, not a second query
tree or an implicit JSON export. Original lexical XML spelling stays in the
import-owned provenance.

XML PI target/data fields are resolved at import for both inspection and
CEM-QL. Consumers use the imported target (`node.name` in CEM-QL) and `value`
directly, never split a raw PI body.
The source data preserves literal line endings while XPath uses the existing
normalized semantic value. AST/tree inspection delegates lifecycle ownership
to import and reports invalid input instead of presenting a partial fallback.

Import also computes versioned source-selection fingerprints from original
input bytes, source identity and semantic profile. Common tree accessors expose
opaque per-node keys and original line numbers to XPath and CEM-QL; consumers
never scan external syntax to recreate provenance. Source keys are independent
of XDM document identity and invalidate on edits/profile changes. Imports with
no original source fingerprint return no key. JSON-to-XML projection retains
parser coordinates directly, including byte-only input, and CSV/YAML document
roots retain source origins. Explicit CSV header options belong to this import
layer; they are not implemented by dropping or remapping rows in an evaluator.

| Audited input | Import responsibility | Shared consumer behavior |
| --- | --- | --- |
| XML | Decode references and normalize literal text/attribute whitespace before merging; retain lexical nodes and namespace provenance. | CEM navigation, logical text, names, values and identity. |
| JSON | Select generic-data or standard JSON-to-XML vocabulary; preserve member order, null and numeric spelling under the declared import policy. | The same node operations, including nodes retained inside XPath arrays/maps. |
| YAML | Parse native YAML and import generic-data nodes; enforce the bounded profile and reject unsupported aliases, anchors and explicit tags. | The same node operations and source coordinates. |
| CSV | Parse native rows and headers; import generic-data nodes with string-valued fields. | The same node operations; numeric casts remain explicitly authored. |

The CEM-QL data reader and XPath's retained lifecycle-AST entrypoint converge on
import. No XML/JSON/YAML/CSV parser-AST traversal remains in that reader or the
XPath evaluator. The native source owner is retained as provenance, not
consulted to evaluate a path or compute a value. Import rejects parser ASTs
carrying hard errors, including partial XML, YAML and CSV documents.

Native coverage must include all supported import formats, both JSON tree
vocabularies, node identity/order, source locations, null/empty values, XML
literal versus referenced whitespace, cache retention/disposal and limits.
`cem_import_boundary.rs` guards downstream node and query modules against direct
source-format AST access. Extend this guard and the import matrix when a new
format or downstream tree consumer is introduced. The `cem_ml:test` Nx inputs
include the guarded CEM-QL sources so changes there invalidate the guard's cache.

The migrated `cem-elements` HTTP loader reads bounded response bytes and delegates
MIME selection, decoding and CEM mapping to `cem_ml::import::import_data_bytes`.
The removed `parseHttpResourceData`/browser XML-record path has no compatibility
binding. The processing host retains native owners and passes explicit document
bindings to CEM-QL; snapshots carry lifecycle metadata with `data: null`. See the
[materialized loader contract](cem-data-loader-plan.md#materialized-loader-contract).
The standalone `custom-element/http-request.js` companion and the unused theme
copy are retired. Their examples install `cem-elements` and query retained CEM
nodes; package exports and IDE metadata no longer advertise the old companion.

`cem_ml_transform_cem_ql::lifecycle_query_stream` now delegates to shared import
and `cem_ql::eval::imported_cem_tree` for XML, JSON, YAML and CSV. Its former
JSON-member/array and XML-event query views are removed. Encoded JSON artifacts
also enter through shared byte import. Consumers query CEM nodes and explicitly
convert scalar text when needed; arrays are no longer flattened into a different
query data model. Duplicate properties, lexical values, provenance and native
owners remain retained. XPath and CEM-QL use the same imported tree capabilities.

`HtmlDomDocumentQueryView` is a view over an explicitly produced HTML DOM
artifact, and collection/source-map views expose transformation outputs. They
are output-artifact inspection contracts, not external-document ingestion.
Do not use these inspection views as substitutes for CEM data import.

The CEM-QL JSON API files and function-companion manifests encode explicit
control/value boundaries. `embedded.rs` parses a named waiver configuration.
These are not external document import implementations. Browser template
declaration loading is a separate authored-template input boundary; it does
not make DOM records an acceptable data-query model.

The browser's `local-storage @type=json` reads through shared CEM-ML import
and binds a native document in `datadom.slices.<name>`. It never exposes a
JavaScript JSON record. Explicit slice edits use CEM-ML's native JSON export;
control metadata and saved-state envelopes carry opaque CEM value artifacts.
See the [accepted write-back contract](browser-stabilization-review.tmp.md#accepted-decision-native-json-storage-write-back).

For implementation details and parity cases, see
[the shared XPath view design](cem-tree-xpath-view.md). The loader first publishes
a materialized CEM tree; progressive consumption of a CEM-ML AST stream is an
accepted later phase, recorded in the loader plan and [wishlist.md](wishlist.md#cem-ml-runtime).
