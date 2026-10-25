# Native DOM, AST and events

Use this ordinary Studio project from **Content types and schemas → Schema
authoring** or **Queries and transformations** to compare source parsing,
retained structural inspection and explicit projection exports. Clone it before
editing. The shared source editor, command/result inspector, source navigation,
structured data view and exact-byte downloads provide the complete workspace;
there is no special example page or application UI JavaScript.

## Original source and human presentation

`data/dispatch.cem` is the authored UTF-8 source. `validate-source` checks it
before creating any projection. `parse-ast` presents native nodes and source
frames as CEM-ML; `parse-events` presents ordered events, trivia and original
byte ranges as CEM-ML. Human presentation does not default to JSON. The AST
view can retain a coarse original source frame, while events expose individual
lexical ranges: `Café Noord` is an 11-byte value starting at byte 147. A native
source range selects its admitted original buffer, never an unrelated active
editor or a JSON property's numeric value.

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

## Named JSON and binary exports

| Configuration | Output resource | Contract |
| --- | --- | --- |
| `export-dom-json` | `projections/dispatch.dom.json` | Explicit native DOM debug/interchange JSON. |
| `export-ast-json` | `projections/dispatch.ast.json` | Explicit native parser-AST array export; see the validation mismatch below. |
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

The current native AST JSON export is an **array**, while the registered AST
JSON source validator requires a **document object**. Revalidating the actual
export with that validator fails with `cem.projection.ast.json_shape` (`$ must
be an object`). The example preserves that real boundary; it does not wrap the
array, invent a repair or advertise an AST JSON round trip.

These native projection lifecycle adapters are output-only. JSON validation does
not make a projection a supported generic Parse/Inspect/convert/query input.
Generic imported-projection inspection/conversion currently follows unsupported
or erroneous fallback paths, so those operations are not configurations here.
Likewise native binary source validation currently checks the envelope header,
not complete payload/chunk integrity; a successful header check is not a claim
of safe replay. Binary and JSON projection resources remain data.

Exported `byteRange` and `sourceMap` fields describe a projection's values. Once
loaded as JSON source, their numbers confer no association with the original
CEM buffer and no right to navigate it. This project's source navigation uses
native diagnostic ranges and admitted buffer identities separately. Shape
validation grants no resource read, code execution or HTML preview authority.

Native Parse/Inspect currently prints original coordinates but does not publish
owned source-map references through the common command result. Its Provenance
panel therefore has no artifact-source links. Those printed fields must not be
reinterpreted as navigation metadata. Use the native `repair-source` diagnostic
in Results → Diagnostics to select the original CEM range before repairing it.
Publishing native inspection provenance is a separate engine follow-up.

## Failure, repair and saved copies

`repair-dom-json`, `repair-ast-json` and `repair-events-json` validate the three
`failures/invalid.<kind>.json` sources. Each has one original
`cem.projection.<kind>.json_shape` error. Restore `"kind": "unknown"` to
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
