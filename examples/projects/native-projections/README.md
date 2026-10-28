# Native DOM, AST and events

Use this ordinary Studio project from **Content types and schemas → Schema
authoring** or **Queries and transformations** to compare source parsing,
retained structural inspection and explicit projection exports. Clone it before
editing. The shared source editor, command/result inspector, source navigation,
structured data view and exact-byte downloads provide the complete workspace;
there is no special example page or application UI JavaScript.

## Authored identity reference

These are resource identities, including expected artifacts. The configurations
below determine their supported uses. An identity alone does not grant an
operation.

| Resource role | Content type | Schema |
| --- | --- | --- |
| Original CEM source | `application/cem` | `https://cem.dev/ns/cem-ml/1` |
| AST binary artifacts | `application/vnd.cem.ast+cem-bin` | `https://cem.dev/ns/projection/ast/1` |
| AST JSON source/artifact buffers | `application/vnd.cem.ast+json` | `https://cem.dev/ns/projection/ast/1` |
| DOM binary artifacts | `application/vnd.cem.dom+cem-bin` | `https://cem.dev/ns/projection/dom/1` |
| DOM JSON source/artifact buffers | `application/vnd.cem.dom+json` | `https://cem.dev/ns/projection/dom/1` |
| Event binary artifacts | `application/vnd.cem.events+cem-bin` | `https://cem.dev/ns/projection/events/1` |
| Event JSON source/artifact buffers | `application/vnd.cem.events+json` | `https://cem.dev/ns/projection/events/1` |
| XML data | `application/xml` | `https://cem.dev/ns/data/xml/1` |
| Owned plain-text inspection source | `text/plain` | No registered source schema |

## Original source and human presentation

`data/dispatch.cem` is the authored UTF-8 source. `validate-source` checks it
before creating any projection. `parse-ast` presents native nodes and source
frames as CEM-ML; `parse-events` presents ordered events, trivia and original
byte ranges as CEM-ML. Human presentation does not default to JSON. The AST
view can retain a coarse original source frame, while events expose individual
lexical ranges: `Café Noord` is an 11-byte value starting at byte 147. A native
source range describes its original buffer. Navigation separately requires an
admitted source endpoint; printed coordinates never authorize an unrelated
active editor or a JSON property's numeric value.

`data/dispatch.xml` independently expresses the same article text with XML
namespace declarations, a comment and CDATA. `validate-xml` checks XML source.
`inspect-xml` inspects its retained imported CEM tree: XML namespaces, lexical
comment/CDATA nodes and original URI, byte, line and column coordinates survive.
The CDATA range begins at byte 169 and covers 23 original bytes, including its
XML delimiters. Structural inspection is neither a second XML parser nor a
claim that these lexical source views are identical.

`inspect-text` demonstrates the additional native `text/plain` loader input in
`data/projection-notes.txt`. It presents one owned text node through the same
CEM-ML inspection path. Plain text has no registered schema package here; the
example claims neither standalone schema validation nor a general formatter.

## Formatting and HTML/XML conversion

`format-xml` formats the original XML resource through its native document view.
`render-html` and `render-xml` convert the original CEM resource into actual
markup. These configurations publish the final writer bytes and native source
ranges together. Use **Results → Provenance** to open the original resource and
select its UTF-8 range, including after saving, cloning or offline reload.
Generated indentation and markup delimiters have no invented source endpoint;
escaped output is not a character-for-character copy of its original range.
HTML output uses the shared guarded preview. No application-specific parser or
JavaScript navigation implementation is involved.

## Named JSON and binary exports

| Configuration | Output resource | Contract |
| --- | --- | --- |
| `export-dom-json` | `projections/dispatch.dom.json` | Explicit native DOM debug/interchange JSON. |
| `export-ast-json` | `projections/dispatch.ast.json` | Explicit semantic AST document JSON, accepted by the registered source validator. |
| `export-events-json` | `projections/dispatch.events.json` | Explicit native ordered event JSON. |
| `export-dom-binary` | `projections/dispatch.dom.cem-bin` | Exact native DOM binary bytes. |
| `export-ast-binary` | `projections/dispatch.ast.cem-bin` | Exact native AST binary bytes. |
| `export-events-binary` | `projections/dispatch.events.cem-bin` | Exact native event-stream binary bytes. |

Each JSON target names `application/vnd.cem.<kind>+json`; each binary target
names `application/vnd.cem.<kind>+cem-bin`. Both use the corresponding explicit
`https://cem.dev/ns/projection/<kind>/1` schema for `dom`, `ast` or `events`.
The native command creates these outputs directly from the original source.
No JSON parse/serialize/reparse operation transports an internal AST or supplies
an evaluator. The original native source owner remains the source of truth.

All six exports have exact native CLI fixtures. Shared worker JSON publication
currently uses compact serialization of the same complete native JSON value,
while CLI output is indented. This known abstract-JSON publication boundary
does not imply cross-host JSON whitespace parity; original ranges and every
field are compared, and each host retains its own exact repeatable bytes.
Binary output is byte-identical across hosts and stable across
independent project roots and remains downloadable as bytes; it is not decoded
as UTF-8 or installed as a live document. Studio keeps binary input resources
read-only and does not run configurations on those inputs. There are therefore
no unfunctional binary-input entries in this project.

## Source validation is not projection import

`validate-dom-json` and `validate-events-json` check the registered source shape
of the corresponding exported JSON buffers. `validate-ast-json` instead checks
the separate authored document object in `projections/authored.ast.json`.
It deliberately has no invented source map or byte ranges.

`validate-ast-export` validates the actual `projections/dispatch.ast.json` export.
The native writer emits `{ "kind": "document", "children": [...] }`, with
namespace fields, text `data`, native reference records and attribute
`valueNodes`, plus original ranges and maps. Reference targets remain opaque
non-owning handles: export never follows cycles or grants their authority.
Older array exports and text records containing only `value` fail the unchanged
registered source-shape validator. Re-export them from the original source.

CEM-ML Parse/Inspect presentation and its optional `parse --format ast-json`
CEMT subject remain a distinct inspection contract. Use explicit `convert`
AST export for registered semantic interchange. Binary bytes and runtime
ownership are unchanged. Successful JSON source validation admits a retained passive tree; it does not
rehydrate an execution owner.

The six native projection MIME identities now share a bounded retained importer.
`parse-dom-input`, `inspect-ast-input` and `inspect-events-input` load JSON
artifacts once at that boundary; the latter presents **event records**, not a
replayed document. `import-dom-html`, `import-ast-xml` and `import-events-cem`
convert their common trees with native writers and artifact-owned source links.
DOM/AST names, attributes, values and passive references are retained. Event
expressions, typed preludes and mode switches stay literal record fields.
Serialized schema IDs are discarded; reference context/targets are not restored.

Binary imports validate every record, version, tag, UTF-8 string, length, range,
ID and owning edge, rejecting truncation, cycles, unreachable nodes and trailing
bytes. Limits are 16 MiB, 65,536 materialized nodes/JSON values and 64 levels.
CEMPROJ versions 1 and 2 are supported; v2 adds attribute value nodes, references
and typed preludes. Sealed JSON envelopes additionally require payload bytes,
contiguous record chunks, matching BLAKE3 hashes, boundaries and ownership links.
Metadata-only envelopes cannot supply a source. Bare binary has no checksum, so
structural validation does not detect arbitrary otherwise-valid value edits.
CEM, XML, HTML and named DOM/AST JSON or binary exports are supported;
executable event replay and runtime recovery are separate ownership contracts.

Studio still keeps binary sources read-only and non-runnable, with exact-byte
downloads. The native binary library/CLI contract does not silently open Studio's
separate owned-command admission boundary.

Exported `byteRange` and `sourceMap` fields describe a projection's values. Once
loaded as JSON source, their numbers confer no association with the original
CEM buffer and no right to navigate it. This project's source navigation uses
native diagnostic ranges and admitted buffer identities separately. Shape
validation grants no resource read, code execution or HTML preview authority.

Native CEM-ML Parse/Inspect, XML formatting and CEM-to-HTML/XML outputs publish bounded artifact-owned
source references through the common command result. Results → Provenance opens
the admitted original resource and its native UTF-8 range. Writer spans and input
identity are retained natively; Studio never reads AST data fields to invent a
link. A representative output span accompanies each distinct source range; it
is not a promise of character-for-character mapping through formatting or
escaping. Rerun after saving changes: stale project revisions cannot be followed.

Explicit JSON and binary exports retain their separate data/codec contracts and
do not gain source-navigation authority from printed fields. Summary or generated
content without a named native boundary has no fabricated endpoint. The
`repair-source` diagnostic remains a separate original-range navigation path.

## Failure, repair and saved copies

`repair-dom-json`, `repair-ast-json` and `repair-events-json` validate the three
`failures/invalid.<kind>.json` sources. Each has one original shape error:
`cem.projection.dom.json_shape`, `cem.projection.ast.json_shape` or
`cem.projection.events.json_shape`, respectively. Restore `"kind": "unknown"` to
`"kind": "document"` for DOM/AST or `"kind": "open"` for events. The repaired
buffers exactly match their valid counterparts. These native shape diagnostics
retain the owned input URI and currently have null text coordinates; no
character ranges are fabricated.

`repair-source` validates `failures/unclosed.cem`. Its missing outer brace
produces three original errors: `cem.ast.unclosed_scope`,
`cem.tokenizer.unterminated_node` and `cem.schema.unclosed_scope`. The tokenizer
points to EOF at line 8, column 1, byte 160; UTF-16 offset 159 differs because
`é` occupies two UTF-8 bytes. Append `}` and a newline to restore the exact
original source. The other diagnostics keep their native ranges or absent
coordinates.

Change `Café Noord` to `Café Haven` in a saved copy of `data/dispatch.cem`, then
rerun Parse and the explicit exports. Their native values, original source
byte ranges and exact downloaded bytes follow the saved buffer. Stored export
resources are baseline fixtures; they do not silently change when the source
is edited. A second clone has an independent project/root identity. Saved
source edits, repairs and output operations remain available after offline
reload, while the original catalog resources stay read-only and unchanged.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. Existing user
projects/drafts, source bounds, ownership, resolver grants and preview policy
are preserved.
