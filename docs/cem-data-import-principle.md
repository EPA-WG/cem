# External data enters through CEM AST import

This is the normative external-data boundary, directed by the user on
2026-09-17. It applies to XML, XHTML, HTML, JSON, YAML, CSV, CSS and future document formats.

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

`cem_ml::import` owns the native XML/XHTML/HTML/JSON/YAML/CSV/CSS data-reader parsing and mapping.
Its typed `import_string` API also serves XPath `parse-xml`, `json-to-xml` and
the explicit `urn:cem:import` CSV/YAML functions. The string profile owns BOMs,
XML declaration handling, JSON codepoint/duplicate/escape policy and URI metadata.
Typed import failures preserve categories and diagnostics; XPath maps them to
standard function errors without decoding documents or reading error prose.
CSV byte and string imports share parser selection at this boundary. An admitted
context supplies its original parser contract; an optional runtime without the
required CSV source reports a source-admission refusal. This includes CEM-QL
`data:read` and `data:parse`. String imports retain their separate 32 KiB and
4096-value limits. Package acquisition and admission remain host-owned.
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

JSON, YAML and JSON Schema source Parse/Inspect consume this retained import,
without passing original data bytes through the CEM tokenizer or compiling
query-looking strings. JSON Schema retains its original schema owner while
mapping its native JSON values through generic-data import; its dialect
admission remains enforced. AST/events presentation identities describe the
output vocabulary, while root metadata identifies the selected source
schema/MIME, original URI and ranges. Events describe the imported common tree,
not lexical JSON/YAML tokens. Native syntax/admission errors and import limits
refuse structural publication; diagnostic inspection remains available. Named
DOM/AST/events JSON exports stay explicit output boundaries. CLI failure policy
still determines exit status; strict source examples select `--fail-level validate`.

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
| HTML | Retain the registered parser's decoded values, namespaces, boolean attributes, raw text, recovery depths and original lifecycle owner/ranges. Preserve hard validation policy and bounded node/depth limits. | The same CEM navigation, XPath queries and source provenance; HTML mirrors remain literal attributes. |
| XML | Decode references and normalize literal text/attribute whitespace before merging; retain lexical nodes and namespace provenance. | CEM navigation, logical text, names, values and identity. |
| XHTML | Select the XHTML lifecycle before generic `+xml` handling; apply its XML-backed root, namespace, head/body and entity policy, then reuse XML semantic import without replacing the original owner. | The same retained CEM navigation, queries and templates, with original XHTML identity, expanded names, foreign content and source coordinates. |
| SVG | Select the dedicated SVG owner before generic `+xml`; apply stored family and XML safety facts, then reuse XML semantic import under the same bounds. | Expanded-name queries, retained nodes and source maps with SVG attribute case and original ranges; no external-resource or script grant. |
| MathML | Retain the generic/presentation/content MIME identity and selected profile; apply existing family/XML hard facts without promoting annotation warnings or replacing the owner. | The same CEM navigation and native templates with original namespaces/ranges. Source validation does not sanitize arbitrary markup or provide algebra/rendering conformance. |
| JSON | Select generic-data or standard JSON-to-XML vocabulary; preserve member order, null and numeric spelling under the declared import policy. | The same node operations, including nodes retained inside XPath arrays/maps. |
| JSON Schema | Retain the native schema document and its JSON owner; enforce source/dialect facts and ordinary JSON import limits before mapping to generic-data nodes. | Structural source operations preserve schema/MIME and original ranges; `$defs`/`$ref` are inert data, without instance validation or reference resolution. |
| YAML | Parse native YAML and import generic-data nodes; enforce the bounded profile and reject unsupported aliases, anchors and explicit tags. | The same node operations and source coordinates. |
| CSV | Parse native rows and headers; import generic-data nodes with string-valued fields. | The same node operations; numeric casts remain explicitly authored. |

The CEM-QL data reader and XPath's retained lifecycle-AST entrypoint converge on
import. No XML/JSON/YAML/CSV parser-AST traversal remains in that reader or the
XPath evaluator. The native source owner is retained as provenance, not
consulted to evaluate a path or compute a value. Import rejects parser ASTs
carrying hard errors, including partial XML, YAML and CSV documents.

Native CEM XPath ingress consumes original lexical namespace capture through a
ready name-completion view over the same arena. Literal bindings require no
execution. Pending values or completion metadata from another owner are rejected;
queries do not manufacture grants, evaluate namespace slots or mutate source
names. CEM-QL's ordinary source-oriented ingress retains its existing behavior.
CSS-selector input navigation also consumes a retained CEM element view. Its
literal-element import profile preserves inert XML CEM payloads, HTML boolean
attributes and decoded names/values without executing expressions. Original
`html:event:N` / `xml:event:N` handles and source event order are compatibility
metadata supplied by import; they are never a parser-AST navigation path. The
view retains the same HTML/XML-family lifecycle owner, including XSLT and XML
Relax NG wrappers, and refuses compact Relax NG, non-element formats, hard
source policy failures and shared byte/node/depth limits.

`RetainedCemTree::node_source_map` supplies semantic-node maps to XPath and CSS;
`source_node_map` supplies original source-arena maps to CEM-QL. Both retain
existing source boundaries and append the retained input URI only when a map
has ranges and no boundary. Source text nodes keep individual lexical ranges;
semantic text runs keep the complete merged ranges. Query source maps remain
separate from selected input maps. Neither exporters nor browser adapters
manufacture input frames or remove URI facts to make hosts agree.

The Login project uses an authored XPath mapping to compare its HTML and CEM
field contracts; no evaluator branches interpret HTML attributes or parser ASTs.

CSS imports use the registered CSS semantic vocabulary, retaining the native
token owner and source locations. `text/css` supports stylesheet,
`mode=scoped-style-block`, and `mode=style-attribute` roots. Import is a
non-executing retention boundary: unresolved import/URL policy facts remain on
the owner but do not prevent retaining references. All other hard CSS diagnostics
still reject import. Retention authorizes no fetch or style installation; the
resolver/loader must enforce resource policy before either. CSS imports are
bounded to 4096 source events and 64 nested component-value levels in addition
to the shared byte limits. Browser style adoption and scoped loading remain
tracked separately in [todo.md](todo.md).

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

Native Markdown lowering and the typed HTML recovery converter explicitly mark
their generated HTML as a passive projection before common retained-tree import.
That private native profile retains resource/script policy facts but defers their
rejection for non-executing inspection and queries; it grants no fetching or
execution. Authored HTML source parsing remains strict, including a reparse of
exported projection bytes. Event-handler policy and document/node/depth limits
still apply. Source URIs, metadata and source-map annotations never confer this
profile. Standalone Markdown inspection/query positions describe generated HTML;
original Markdown metadata/body origins are carried by the explicit site graph.

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

### Constructed native stage handoff

Constructed CEM values are already inside the native boundary. A stage explicitly
targeting `application/cem` with the CEM-ML schema may pass its retained native
value allocation to an XSLT consumer through `OwnedXdmDocument`. This creates one
semantic XDM document for absolute XPath over empty, single-root or multi-root
output, without calling an external parser or presentation writer. Original
reference owners, expanded names, typed attribute contracts and source provenance
remain retained. Normal CEM consumers keep their existing flat native sequence.

Native forests record each node's semantic root during validation. Absolute paths
and order axes use that root rather than the arena's inaccessible storage sentinel.
The document projection checks node/depth/byte bounds and cancellation, retains
an accounted memory permit, and rechecks consuming-scope admission on reuse.
Original source URI/key/ranges are provenance, not a fabricated document URI or
source line index. Cache eviction does not invalidate selected native nodes;
actual owners determine lifetime. Encoded XML remains text and is not silently
reimported. Serialized ASTs, plain records and presentation trees do not establish
native ownership. Studio's CEM transformation project demonstrates this through
its ordinary `build-xslt-report` configuration. Credential recovery refusals,
automatic-recovery defaults and checkpoint limits remain unchanged.
