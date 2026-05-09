# Schema-Defined Streaming Parser Research

This memo compares algorithms and implementation patterns for schema-defined,
stream-based parsers across XML, Invisible XML, JSON, CSV/CSF, HTML, CSS,
TypeScript, and Rust. For this pass, CSF is treated as CSV-style delimited
text.

The practical recommendation is a layered parser runtime:

1. Byte source and encoding decoder.
2. Format tokenizer or scanner.
3. Normalized event stream.
4. Schema-compiled validator/parser state machine.
5. Scoped embedded-language handoff stack.
6. Interpreter AST builder with per-node source-map stacks.
7. Binary AST encoder with platform dictionaries.
8. Compressed subtree segment delivery.
9. Implementation interpreter.

This separates transport and encoding concerns from schema-driven tokenization,
keeps schema validation incremental, makes embedded regions explicit instead of
special-casing them inside every parser, and keeps implementation behavior such
as DOM construction or runtime execution out of the tokenizer.

The internal AST can also be represented as a binary graph/tree format after it
is built. Compression belongs on top of that binary representation, and the
compressed content should be segmented by subtree roots where possible. This
allows simultaneous delivery, broadcast, missing-chunk retry, and chunk-level
parallel parsing/preprocessing.

## Executive Recommendation

The strongest general architecture is to compile schemas into small streaming
machines that consume typed events. Nested structures should use push/pop scope
frames, and embedded content should switch parser schemas through explicit
handoff events.

For XML-like nested formats, model the stream as visibly nested events: start,
attributes, text, end. Validate element content with deterministic automata,
RELAX NG-style derivatives, or visibly pushdown automata. For JSON, validate
token streams with a stack of object/array frames and schema states. For
CSV/CSF, use DFDL-like schema-guided recursive descent over fields, records,
delimiters, and lengths. For HTML, use WHATWG parsing behavior as a
compatibility reference, but keep the schema-driven tokenizer separate from the
HTML DOM implementation that interprets events and constructs the final DOM.
For CSS, TypeScript, and Rust, use schema-aware lexical modes and scoped name
slots that unresolved references can point to before the defining entity is
seen.

LLVM/Clang and TypeScript are most useful as architecture references for byte
buffers, spans, diagnostics, lexical modes, delayed binding, incremental reuse,
and error recovery. They should not be copied as direct schema-validation
engines.

## Core Architecture

### 1. Byte Source And Encoding Decoder

The byte layer owns raw input, chunking, and encoding detection. It should
preserve byte offsets even when it decodes to Unicode scalar values or UTF-16
code units.

Rules:

- Keep absolute byte offsets for every token and event.
- Keep decoded scalar spans for Unicode-aware validation.
- Preserve raw slices where possible for diagnostics, round-tripping, and
  zero-copy string/token views.
- Attach source-map stacks to every AST element, not just diagnostics.
- Validate UTF-8 at ingress for formats that require UTF-8, such as JSON in
  common modern pipelines and Rust source.
- Allow XML-style entity-specific encodings where the format requires it.

LLVM's `MemoryBuffer` is a useful model for efficient source buffers: it
provides read-only access to bytes and guarantees a sentinel byte after the end
of the buffer for fast lexing. Clang's `SourceManager` shows how to separate
logical source locations from physical buffers, macro expansions, and diagnostic
ranges.

### 2. Tokenizer Or Scanner

The scanner converts decoded input into format-native tokens. It should be
schema-aware, streaming, and mode-aware. The schema is the source of truth for
which token forms, delimiters, lexical modes, embedded regions, and scope
boundaries are meaningful. Not every schema rule belongs in tokenization, but
the tokenizer must be driven by the schema subset that affects lexical
extraction.

Examples:

- XML scanner emits start element, attribute, text, comment, processing
  instruction, and end element events.
- JSON scanner emits structural tokens, strings, numbers, booleans, and null.
- CSV scanner emits record, field, delimiter, quote, escaped quote, and newline
  events.
- HTML scanner uses schema-selected tokenizer states such as data, RCDATA,
  RAWTEXT, script data, tag open, and attribute states.
- CSS scanner follows the CSS Syntax tokenization rules and leaves
  property-specific validation to downstream grammars.
- TypeScript/Rust scanners classify identifiers, keywords, literals, trivia,
  delimiters, and contextual tokens.

The scanner should know the schema rules that affect token extraction:
delimiters, escapes, lexical modes, embedded content boundaries, valid token
families, and source preservation requirements. Structural validation,
cross-reference binding, semantic constraints, and execution remain downstream
interpreter responsibilities.

### 3. Normalized Event Stream

A cross-format schema engine should not consume raw tokens directly. It should
consume a normalized event stream with a small set of event categories:

| Event category | XML/HTML example | JSON example | CSV/CSF example | Code example |
| --- | --- | --- | --- | --- |
| Open scope | start tag | object/array start | record start | block/module start |
| Close scope | end tag | object/array end | record end | block/module end |
| Name | QName, attribute name | property name | header/column name | identifier |
| Value | text, attribute value | string/number/null | field text | literal |
| Separator | element boundary | comma/colon | delimiter/newline | comma/semicolon |
| Mode switch | script/style/CDATA | string subdocument | typed column payload | template/JSX/attribute |
| Error | parse error | invalid token | malformed quote | recovery token |

This layer lets schema validation share algorithms across formats while still
retaining each format's scanner and recovery behavior.

### 4. Schema-Compiled State Machine

Schemas should compile to a runtime plan:

- Content models become finite automata, derivatives, or parser functions.
- Nested structures push frames containing schema state, namespace/context data,
  and expected close events.
- Object/record constraints track required names, seen names, multiplicity,
  ordering, and unevaluated fields.
- Value constraints validate scalar type, lexical form, range, pattern, and
  normalization.
- Handoff rules define when embedded content changes parser and schema.

The runtime should be able to stop early on fatal errors or continue in a
diagnostic mode that records recoverable errors.

The parser state machine builds an interpreter AST, not an executing runtime.
The AST can contain loaded code, declarations, entity references, schema
references, and embedded-language regions that are not yet executed or fully
bound. Unresolved references point to mutable scoped name slots. When a target
token, declaration, or entity is defined, the interpreter updates that slot, so
existing references observe the value through the shared binding.

Every AST element should carry a source-map stack. A source-map stack records the
owned context that produced the node and the current context transformations
that were applied after it was produced. This lets tooling traverse backward and
forward through each layer: original stream, decoded text, schema tokens,
normalized events, embedded content extraction, generated XML/DOM/AST nodes,
and later interpreter transformations. For binary streams, the stable source
coordinate is byte offset and length. For text-based code, expose line and
column as derived coordinates while keeping byte offsets as the ground truth.

### 5. Scoped Embedded-Language Handoff Stack

Embedded languages need first-class scope frames. A parent parser should emit a
handoff event with:

- child content type or language id;
- byte span or streaming sub-source;
- inherited context such as namespace, HTML tag name, attribute name, MIME type,
  or JSON pointer;
- child schema id;
- return condition, such as matching end tag, string end, block close, or fixed
  length.

Examples:

- HTML `<script type="module">` switches to JavaScript or TypeScript-like
  syntax.
- HTML `<style>` and `style=""` switch to CSS.
- XML CDATA can be raw text or a schema-defined embedded payload.
- JSON strings may contain schema-tagged subdocuments.
- CSV columns may have per-column schemas, such as dates, JSON, CSS tokens, or
  domain-specific expressions.
- TypeScript template literals, JSX, and tagged templates can switch lexical or
  parser modes.

## Algorithm Comparison

| Approach | Best fit | Streaming behavior | Strengths | Risks |
| --- | --- | --- | --- | --- |
| SAX event pipeline | XML | Push events as input is scanned | Low memory, mature Java stack, easy filters | Application owns state; awkward for random access |
| StAX cursor/pull API | XML | Consumer asks for next event | Backpressure-friendly, procedural state | Still XML-specific |
| Xerces XNI pipeline | XML and custom event pipelines | Scanner, filters, validators, handlers | Modular scanner/validator architecture; can build custom parsers | Internal API, more complex than SAX/StAX |
| XML Schema DFA/content model | XML | Validate element children incrementally | Efficient deterministic validation | XSD edge cases are complex: wildcards, UPA, substitution groups |
| RELAX NG derivatives | XML and grammar-like schemas | Residual schema updates per event | Elegant incremental validation; good diagnostics with residuals | Needs memoization to avoid growth |
| Visibly pushdown automata | Nested markup/data | Push on open, pop on close | Formal model for streaming nested data | Less natural for unordered object properties |
| Invisible XML grammar parsing | Text formats that should expose XML structure | Grammar parser emits XML-shaped events | Makes non-XML text usable by XML pipelines while keeping original syntax | Ambiguity and serialization marks need explicit source-map ownership |
| DFDL schema-guided recursive descent | CSV/CSF, fixed-width, binary, legacy records | Schema drives reads over stream | Strong fit for delimited and fixed layouts | Ambiguous layouts need unresolved field/reference tracking |
| Jackson-style token stream | JSON and related formats | Pull tokens in source order | Simple, low overhead, reusable across data formats | Schema logic must be layered separately |
| simdjson structural indexing | JSON | Finds structure and validates UTF-8 fast | Excellent byte-level throughput | More document-oriented than general schema runtime |
| WHATWG HTML parsing behavior | HTML | Tokenizer states plus DOM interpreter rules | Browser-compatible recovery and embedded text modes | Should be split into schema tokenizer and DOM implementation interpreter |
| CSS Syntax parser | CSS | Token stream to component values/rules | Separates syntax from property grammar | Full validation needs property/value grammars |
| Recursive descent with scoped name slots | TypeScript, Rust, CSS values | Token stream plus mutable scope bindings | Practical recovery and contextual grammar handling | Requires explicit scope slot lifecycle and override rules |
| Tree-sitter incremental parsing | Programming languages and mixed documents | Incremental parse trees, changed ranges | Excellent editor use case and language injections | Produces trees, not schema validation by itself |
| Binary AST with subtree compression | AST transport, cache, broadcast, and parallel preprocessing | Encoded AST chunks can stream independently | Shortens delivery, supports missing-chunk retry, and enables parallel subtree work | Needs stable ids, shared dictionaries, and cross-chunk reference handling |

## Unicode And UTF-8 Handling

| Stack | Source model | Unicode approach | Recommendation |
| --- | --- | --- | --- |
| XML | Parsed entities decode to Unicode characters | XML processors must accept UTF-8 and UTF-16; legal XML characters exclude surrogate blocks and selected noncharacters | Decode per entity, validate XML character ranges, keep raw entity offsets |
| Invisible XML | Text parsed by an ixml grammar | Grammar terminals include strings, character sets, ranges, and Unicode character classes | Preserve original text spans and map generated XML nodes back to grammar symbols and input ranges |
| JSON | Unicode text, commonly UTF-8 bytes | JSON Schema works on the JSON data model; fast parsers such as simdjson validate UTF-8 | Require valid UTF-8 for byte input unless a caller supplies decoded text |
| CSV/CSF | Dialect-defined text bytes | Encoding is external to RFC-style CSV syntax; delimiters can be bytes or characters | Decode before field parsing unless a schema declares byte-oriented delimiters |
| HTML | Byte stream decoded before schema tokenizer | WHATWG defines encoding detection, preprocessing, tokenizer states, and replacement handling | Use those rules as compatibility constraints while keeping DOM construction in the interpreter |
| CSS | Bytes to code points, then tokens | CSS Syntax defines byte-to-stylesheet parsing; Rust `cssparser` consumes `&str` and points to encoding helpers | Decode to UTF-8/Unicode before tokenization; keep byte spans for source maps |
| TypeScript | JavaScript string/source text | TypeScript assumes UTF-8 for files and detects UTF-8/UTF-16 BOMs; scanner works over source text/code units | Keep byte offsets externally, map to UTF-16 positions for TS-compatible diagnostics |
| Rust | UTF-8 `&str` | Identifiers use Unicode XID properties and NFC equality; `rustc_lexer` works on `&str` | Validate UTF-8 at ingress, normalize identifiers only at identifier comparison/interning |
| LLVM/Clang | Byte buffers and source locations | Lexer handles bytes and language-specific Unicode rules; source manager maps locations | Use byte spans as ground truth and layer Unicode validation by language |

### Design Policy

Use byte offsets as the stable storage format. Add decoded scalar spans and, when
needed, UTF-16 code unit positions as derived views. This avoids losing exact
source locations and lets the same parser runtime support browser-style
diagnostics, TypeScript-compatible positions, Rust UTF-8 source rules, and
zero-copy byte slices.

Identifier policies must be language-specific:

- XML names follow XML name character rules.
- TypeScript and JavaScript identifiers follow ECMAScript/TypeScript scanner
  behavior and source text positions.
- Rust identifiers follow Unicode XID rules and NFC normalization.
- CSS identifiers follow CSS Syntax escapes and tokenization.

Do not normalize all text globally. Normalize only where the language or schema
requires it, such as identifier equality or string constraints.

## Source-Map Stack Model

Source maps are part of the AST contract. They are not an optional diagnostic
side table, because schema-driven parsing can pass through multiple owned
contexts and transformations before an implementation consumes the result.

Each AST element should contain:

- `owned_source`: the source context that created this node.
- `current_source`: the current transformed context for this node.
- `map_stack`: ordered mappings from current context back to prior contexts.
- `range`: byte offset/length for uncompressed binary sources, bit
  offset/length for compressed binary sources, plus derived line/column for
  text sources.
- `transform`: tokenizer, schema rule, embedded-language extraction,
  serialization, DOM construction, lowering, or runtime preparation step.

The stack must support traversal across layers. For example, an HTML `style`
attribute can map from a CSS declaration AST node to the attribute value, then
to the HTML token, then to decoded source text, then to the original byte
stream. An Invisible XML result node can map from generated XML markup to the
ixml grammar symbol that produced it and the original unmarked text span.

For generated nodes with no direct input text, store the generating transform
and the nearest owning source range. For merged nodes, store multiple source
ranges in the same stack frame. For split nodes, each child gets the inherited
stack plus its narrowed current range.

Compressed binary content needs bit-level source maps. Entropy-coded payloads,
prefix codes, block headers, and compression metadata can have meaningful
boundaries that do not align to bytes. The source-map stack should therefore
support byte ranges for ordinary binary data, bit ranges for compressed binary
frames, and line/column projections for text contexts.

## Binary AST, Compression, And Segmentation

The interpreter AST should have a canonical binary representation. Text
serialization is useful for debugging and interchange, but binary AST is the
better internal transport and cache format because it can encode node kinds,
schema ids, scope slots, source-map stacks, string tables, and typed values
without repeated textual markup.

Compression should be applied above the binary AST encoding. The compression
model should exploit repeated root structure, shared schema definitions, common
node layouts, and repeated source-map frame shapes. This is similar to the
solid-archive advantage: a common dictionary/root tree improves compression over
many related nodes. The important difference is that the common dictionary
should be split from app-specific payloads:

- Platform dictionary: common AST node kinds, primitive value encodings, schema
  definitions, source-map frame schemas, standard content-type ids, and shared
  string/symbol tables.
- Application dictionary: app-specific schemas, domain node kinds, local symbol
  tables, and frequently repeated literals.
- Payload chunks: subtree AST nodes, local values, scope slots, references,
  source-map stack deltas, and embedded content ranges.

The platform dictionary can be installed or cached once by the runtime. App
payloads then carry only dictionary ids and app-specific deltas. This keeps
network payloads smaller and makes different documents share the same root AST
vocabulary.

### Subtree-Scoped Chunks

Chunk boundaries should align to subtree roots whenever possible. A subtree
chunk is independently decodable when it contains:

- subtree root id and parent anchor;
- required dictionary ids and versions;
- local node table and edge table;
- local scope slots and unresolved/external references;
- source-map stack frames or deltas;
- child chunk links and integrity hashes;
- compression frame metadata.

Subtree chunking makes the AST suitable for parallel streaming and parallel
preprocessing. Independent chunks can be parsed, decoded, validated against
their local schema state, decompressed, indexed, and partially interpreted on
different threads. A chunk or series of chunks can be delivered before the full
document arrives, which is useful for progressive rendering, editor indexing,
distributed validation, or broadcast to many consumers.

### Network Delivery And Recovery

Segmented compressed chunks improve delivery in three ways:

- Simultaneous delivery: multiple subtree chunks can be fetched or broadcast in
  parallel instead of waiting for one monolithic archive.
- Error resistance: failed chunks can be requested again by id/hash without
  retransmitting the whole AST.
- Prioritization: root, schema, viewport, or execution-critical chunks can be
  delivered before deep or inactive subtrees.

Each chunk should carry an integrity hash and a dependency list. The receiver
can build a partial AST with placeholders for missing children and fill the
corresponding scope slots when chunks arrive. This mirrors the live scoped name
slot model used for unresolved references: absent chunks are represented by
stable anchors, then replaced or completed as their data becomes available.

Chunk size should be transport-aware. The standard should not require one fixed
size, because packetization, congestion window, TLS record sizing, QUIC stream
framing, HTTP/3 framing, path MTU, and storage page size all affect the best
choice. Instead, define a negotiation guideline:

- Start with a conservative default chunk target suitable for common HTTP/TCP
  delivery and OS page/cache behavior.
- During connection setup, allow peers to exchange preferred chunk ranges,
  maximum in-flight chunks, cache page size, compression support, and transport
  hints.
- During delivery, allow the sender to adjust chunk grouping when connection
  properties become known, such as observed RTT, loss, congestion window,
  negotiated QUIC/HTTP settings, or datagram PLPMTU.
- Cache successful client/server/network-location profiles on the client, keyed
  by server identity, route/network class, transport, and dictionary version.
- Keep subtree identity independent from transport segmentation, so a logical
  subtree chunk can be split across packets or grouped with adjacent chunks
  without changing AST identity.

Broadcast delivery couples chunking to network behavior. A chunk size that is
optimal for one client may be poor for another. Broadcast systems should use a
baseline chunk size that most clients can receive efficiently, then support
repair requests for missing/error chunks and optional unicast refinement for
clients with different path properties.

### Compression Strategy

The binary AST format should define an uncompressed representation and canonical
compression profiles. The standard should not enforce compression as a required
layer, but it should provide canonical guidelines and a reference
implementation so producers can make interoperable choices. Reference server
chunking, compression selection, and cache policy are part of the platform
layer as reference implementation, not app-specific parser behavior.

Recommended profiles:

- `none`: uncompressed binary AST chunks. Required for debugging,
  deterministic tests, memory mapping, and environments where compression cost
  exceeds network/storage savings.
- `canonical-fast`: independently compressed subtree chunks using a fast,
  dictionary-capable method such as Zstandard.
- `canonical-dense`: denser compression for cold storage or batch transfer,
  such as Brotli or a stronger Zstandard level, with less emphasis on random
  access latency.
- `solid-archive`: optional whole-document or chunk-group compression for cold
  storage only, where retry, random access, and parallel decode are not
  priorities.

A web server can use uncompressed binary AST chunks as an intermediate form:

```text
Interpreter AST
  -> canonical uncompressed binary subtree chunks
  -> negotiated compressed/cacheable transport chunks
```

This is valuable when the same AST payload may be served repeatedly or to
clients with different network and compression capabilities. The server can
validate chunk boundaries once, assign stable subtree ids, reuse source-map and
dictionary indexes, and then generate `none`, `canonical-fast`,
`canonical-dense`, or transport-specific chunk groupings without rerunning the
schema tokenizer and AST builder. The uncompressed intermediate can itself be
cacheable in the reference server so compression and transport packaging become
platform services. For one-off payloads, direct compressed chunk generation may
be cheaper because it avoids storing the larger intermediate representation.

Do not compress the whole document as one opaque blob if parallel delivery or
error recovery matters. Prefer dictionary-based compression plus independently
compressed subtree chunks:

- Use the platform dictionary as a shared static compression context.
- Use the app dictionary as a per-application or per-document compression
  context.
- Compress each subtree chunk independently enough that it can be retried,
  cached, and decoded without the whole archive.
- Allow optional chunk groups when nearby chunks benefit strongly from a shared
  local dictionary.
- Keep source-map stacks compressible by encoding them as frame ids plus range
  deltas.
- Preserve bit-level source-map ranges for compressed binary content. Byte
  offsets are enough for uncompressed binary AST chunks, but compressed streams
  and entropy-coded payloads can address meaningful boundaries at bit
  granularity.

The tradeoff is compression ratio versus independence. A single solid archive
usually compresses better, but it blocks random access, retry of only missing
data, and parallel chunk decoding. Network delivery and chunking are coupled:
smaller chunks improve repair granularity and scheduling, while larger chunks
usually improve compression ratio and reduce per-chunk overhead. Subtree-scoped
chunks should be the default for networked or interactive runtimes; full solid
compression is only suitable for cold storage or batch transfer where random
access and recovery are less important.

### Compressed AST As DOM Storage

Compressed binary AST chunks can also be a useful DOM storage representation,
not only a wire format. Many DOM and XPath-like workloads walk tree hierarchy
sequentially: descend to children, scan siblings, test node names, read
attributes, and project text. A chunked binary tree can support those paths
efficiently when each chunk has a compact node table, child edge table,
attribute table, and local string/symbol table.

The storage format should support two access modes:

- Indexed compressed access: read chunk headers, dictionaries, node tables, and
  local indexes without inflating the whole subtree.
- Lazy materialization: decompress only the subtree chunks required for the
  current query, edit, render, or source-map traversal.

For XPath-like queries, keep enough uncompressed or separately indexed metadata
in the chunk header to skip irrelevant subtrees: node kind summaries, expanded
name ids, namespace ids, attribute-name summaries, text-presence flags, and
child chunk links. Sequential hierarchy traversal then remains efficient while
large inactive subtrees stay compressed.

### File Mapping And Direct Storage Paths

Subtree-compressed chunks are well suited to file-backed storage. If chunks are
written as independent file extents with page-aligned offsets, the runtime can
use memory mapping to load only required DOM subtrees. The OS page cache then
handles dynamic loading and eviction without asking the implementation to copy
or compute the whole DOM in memory.

Platform notes:

- Linux: `mmap` maps files into process address space; `splice` can move data
  between file descriptors through pipes without copying through user space, and
  `io_uring` includes splice operations. Newer Linux zero-copy receive paths can
  reduce kernel-to-user copies for network receive, but true socket-to-file
  direct storage remains hardware, kernel, filesystem, and API dependent.
- Windows: `CreateFileMapping` and `MapViewOfFile` provide file-backed mapped
  views. `TransmitFile` is optimized for file-to-socket transfer through the OS
  cache manager. Winsock Registered I/O can reduce overhead for socket buffers,
  but there is no general portable user-space guarantee that received socket
  bytes are written directly to a file without touching RAM.
- macOS: `mmap` maps files or devices into memory, and `sendfile` sends file
  data to sockets. General socket-to-file receive still normally goes through
  kernel/user buffers or file cache paths rather than a portable direct-to-disk
  API.

The design should therefore allow an optimized receive-to-storage path but not
require it. A receiver may store network segments directly into a chunk file or
cache file when the platform can do so safely; otherwise it should fall back to
ordinary buffered receive and write. The file format should make both paths
equivalent by requiring chunk hashes, offsets, lengths, dictionary ids, and
source-map bit ranges in the chunk index.

### Cross-Chunk References

Cross-chunk references should use stable ids and scoped slots, not raw offsets.
A node may reference a declaration, schema entity, source-map frame, or child
subtree that is not yet delivered. The receiver creates a slot for that id and
fills or overrides it when the defining chunk arrives.

This keeps chunk-level decoding independent while preserving AST semantics. It
also lets preprocessing run in parallel: local validation and indexing can
finish before every external reference is present, while consumers that need a
missing value can wait on the specific slot.

## Embedded And Scoped Content

| Parent format | Embedded region | Detection | Child parser | Return condition |
| --- | --- | --- | --- | --- |
| HTML | `<script>` | tag name plus `type` attribute | JavaScript/TypeScript or raw text | matching `</script>` in script-data state |
| HTML | `<style>` | tag name | CSS | matching `</style>` |
| HTML | `style=""` | attribute name/schema | CSS declaration parser | attribute quote end |
| XML | CDATA/text | element schema or CDATA marker | raw text, XML fragment, or domain parser | CDATA end or element close |
| JSON | string subdocument | schema keyword/content type | JSON, CSS, regex, DSL, etc. | JSON string end after unescape policy |
| CSV/CSF | typed field | column schema/header | scalar parser or embedded document parser | delimiter/newline outside quotes |
| TypeScript | JSX | lexical context | JSX/HTML-like parser | JSX close/expression boundary |
| TypeScript | template literal | backtick/tag context | template or tagged child parser | template tail/backtick |
| CSS | `url()`, custom functions | function token/name | URL/string/schema parser | function close |

The handoff stack should make these boundaries explicit. A child parser should
never infer the parent close condition independently unless the embedded language
specification requires it, as HTML script data does.

## Format Notes

### XML And The Java Stack

XML is the strongest precedent for schema-driven streaming. SAX pushes events,
StAX pulls cursor or iterator events, and Xerces XNI models parsing as a
pipeline of scanner, filters, validators, and handlers. XNI is especially useful
as an architectural reference because it separates parser configuration,
components, features/properties, scanners, and document event pipelines.

For schema validation, XSD content models can be compiled to finite automata.
This is efficient for ordered child content and occurrence constraints. RELAX NG
derivatives are also attractive: after each event, compute the residual schema
that remains to be matched. This gives a natural streaming algorithm and can
improve diagnostics because the residual describes what was expected next.

Nested XML can also be modeled as a visibly pushdown language: start tags push
schema frames, end tags pop them, and text/attributes update the current frame.
This is a good formal foundation for a general nested event runtime.

### Invisible XML

Invisible XML, or ixml, is directly relevant because it describes non-XML text
with a declarative grammar and makes the implicit structure explicit as XML.
The input text stays in its original syntax, while the parser produces
XML-shaped structure according to grammar rules and serialization marks.

In this architecture, ixml is a schema-driven tokenizer/parser profile:

- The ixml grammar defines terminals, nonterminals, character sets, insertions,
  hidden symbols, attributes, and element serialization.
- The tokenizer and parser emit normalized XML-shaped events rather than
  constructing implementation-specific DOM nodes.
- Every generated element, attribute, text node, and insertion carries a
  source-map stack back to the ixml grammar rule and original input range.
- Insertions and hidden grammar symbols need source maps that distinguish
  generated structure from consumed source text.
- Ambiguous parses should remain explicit until the implementation interpreter
  chooses a policy such as reject, choose first, expose forest, or require a
  disambiguating schema rule.

Invisible XML is a bridge between DFDL-style schema-defined text parsing and
XML event pipelines. It is especially useful for formats whose source syntax is
not XML but whose downstream tooling benefits from XML-like events, XPath-like
addressing, or XML serialization.

### JSON

JSON parsing should be token-stream based. Jackson's core streaming API is a
practical model: a parser exposes tokens in input order and higher layers build
data binding or validation on top. simdjson is a byte-level performance model:
first identify structural characters and validate UTF-8 quickly, then navigate
or consume values.

JSON Schema is more complicated than simple structural validation because modern
drafts include dynamic references, applicators, annotations, and unevaluated
locations. A streaming validator can still handle many schemas incrementally,
but full JSON Schema support may require buffering for object-level constraints,
unordered properties, annotation collection, and cross-property dependencies.

Recommended approach:

- Use a token stream as the parser interface.
- Keep object/array frames with active schema states.
- Track required properties, seen properties, additional/unevaluated property
  state, and item index.
- Permit bounded buffering only where the schema requires sibling or annotation
  information.

### CSV/CSF And DFDL

CSV and CSF should be treated as dialects of delimited records. The scanner must
handle delimiter, quote, escape, record separator, comments, and malformed quote
recovery according to the selected dialect.

DFDL is the best schema-driven model for delimited, fixed-width, binary, and
legacy data formats. It uses an annotated XML Schema subset to describe logical
data and physical representation. Its logical parser is schema-guided recursive
descent over the representation described by the schema. In this architecture,
ambiguous or unresolved field references point to scoped name slots in the
interpreter AST. The slot value is set or overridden when the referenced token or
entity is defined.

Recommended approach:

- Compile record schemas into field readers.
- Make delimiter and quote behavior dialect properties.
- Allow fixed, delimited, prefixed, and explicit length field modes.
- Store unresolved field/entity references as pointers to current-scope slots.
  Set or override the slot when the referenced token or entity is defined.
- Emit normalized record/field events into the same validation runtime used by
  other formats.

### HTML

HTML compatibility still has to respect WHATWG parsing behavior, but the
architecture should not collapse tokenization and DOM construction into one
component. The schema-driven parser tokenizer extracts events and switches
lexical states. The HTML DOM implementation is an interpreter over those events:
it applies insertion modes, manages the stack of open elements, handles active
formatting elements, resolves foreign content behavior, and constructs the final
DOM.

This separation matters because the same tokenizer/event layer can feed other
interpreters besides a DOM builder: validators, source analyzers, transform
pipelines, or partial document loaders. The useful schema lesson from HTML is
scoped mode switching. A start tag can change tokenizer state and downstream
interpretation, but the schema should define which transitions are valid and
which child parser should own the embedded region.

### CSS

CSS Syntax defines how to turn bytes into tokens and component values. It does
not validate every property and value by itself. Libraries such as Rust
`cssparser` implement the syntax layer, while Lightning CSS demonstrates a
second layer that parses many properties into typed value structures.

Recommended approach:

- Treat CSS Syntax as the scanner and component-value parser.
- Compile property schemas into value grammar validators.
- Use lexical modes for functions, blocks, strings, URLs, and escapes.
- Preserve source spans for source maps and diagnostics.

### TypeScript, Babel, Oxc, And typescript-eslint

TypeScript's parser is a production recursive-descent parser over a scanner. It
uses contextual parsing, recovery, and syntax tree construction optimized for
incomplete code and typechecking. The scanner is mode-aware for JSX, template
literals, regular expressions, comments, trivia, and contextual tokens.

typescript-eslint shows the adapter pattern: TypeScript's AST is converted to
ESTree-compatible nodes and optionally connected to TypeScript programs for type
information. Babel and Oxc show another direction: plugin/configuration-driven
JavaScript/TypeScript parsing for transformation, tooling, and high throughput.

Recommended lessons:

- Keep scanner and parser states explicit and reusable.
- Keep references attached to scoped name slots, then set or override the slot
  when the declaration, token, or entity is defined.
- Separate syntax parsing from semantic/type validation.
- Build AST adapters as separate layers rather than changing the core parser.
- Cache source files, tokens, and parse results when editor/incremental use
  matters.

### Rust

Rust is a useful Unicode and lexer reference. Rust source is UTF-8, and
identifiers use Unicode XID properties with NFC-based equality. The
`rustc_lexer` crate separates low-level lexing from rustc-specific spans,
diagnostics, interning, and parser token conversion.

Recommended lessons:

- Keep raw lexing reusable and side-effect-light.
- Attach language-specific diagnostics later.
- Treat identifier equality and normalization as semantic token processing, not
  whole-file text rewriting.

### LLVM And Clang

LLVM/Clang is the best source-location and byte-buffer reference. LLVM's
`StringRef` and `MemoryBuffer` patterns emphasize cheap views over owned bytes.
Clang's `SourceManager` maps logical source locations to buffers and handles
diagnostic ranges across more complicated cases such as macro expansion.

Recommended lessons:

- Keep source identity, buffer identity, and byte ranges separate from AST
  nodes.
- Store cheap slices/views where lifetimes are known.
- Make diagnostics a first-class consumer of parser span data.
- Use sentinel/padding techniques only in the byte layer where memory ownership
  is controlled.

### Tree-sitter

Tree-sitter is most relevant for editor scenarios and embedded languages. It
provides incremental concrete syntax trees and supports multi-language parsing
through included ranges and injections. It is not a schema validator, but its
incremental invalidation model and language-injection pattern are directly
useful for scoped embedded content.

Recommended lessons:

- Represent embedded regions as ranges with child language identities.
- Reparse changed ranges instead of whole documents in editor contexts.
- Keep concrete syntax when refactoring, formatting, or diagnostics need exact
  source structure.

## Proposed Runtime Model

```text
ByteSource
  -> EncodingDecoder
  -> SchemaTokenizer
  -> EventNormalizer
  -> SchemaMachine
  -> InterpreterAstBuilder
  -> BinaryAstEncoder
  -> ChunkCompressor
  -> SegmentBroadcaster / Cache / ImplementationInterpreter / Diagnostics
```

The schema machine runs a stack of frames:

```text
Frame {
  schema_id
  language_id
  state
  source_span
  source_map_stack
  binary_node_id
  chunk_id
  expected_close
  namespace_or_context
  seen_names_or_fields
  diagnostics
}
```

State transitions:

- `open(event)`: validate the event, push child schema frame if nested.
- `value(event)`: validate scalar content and update current state.
- `separator(event)`: update sequence, record, or property state.
- `handoff(event)`: create child parser from source range or stream.
- `close(event)`: validate nullable/complete state and pop frame.
- `error(event)`: record diagnostic and run recovery strategy.
- `transform(event)`: append a source-map stack frame when a token, event, AST
  node, generated XML node, DOM node, or lowered runtime node changes context.
- `encode(node)`: assign binary node ids, dictionary references, source-map
  deltas, and chunk ownership.
- `segment(subtree)`: close a subtree-root chunk with dependencies, hashes, and
  compression metadata.

## Practical Algorithm Choices

Use these defaults unless a schema or format requires otherwise:

| Problem | Default algorithm | Reason |
| --- | --- | --- |
| XML element content | DFA from content model | Fast, deterministic, proven in XSD implementations |
| XML flexible grammar | RELAX NG derivatives | Residual schema is good for streaming and diagnostics |
| Non-XML text to XML events | Invisible XML grammar parser | Declarative grammar can expose implicit structure as XML-shaped events |
| Nested events | Visibly pushdown frame stack | Natural fit for open/close structures |
| CSV/CSF records | DFDL-style schema-guided recursive descent | Handles delimiters, lengths, and field typing |
| JSON structures | Token stream plus schema frame stack | Low memory and compatible with Jackson/simdjson style |
| HTML | Schema tokenizer plus DOM implementation interpreter | Preserves browser-compatible behavior while separating token extraction from DOM construction |
| CSS values | CSS Syntax tokens plus property grammars | Mirrors browser-grade parser layering |
| TypeScript/Rust syntax | Schema-aware recursive descent with scoped references | Practical for contextual programming languages |
| Mixed-language documents | Handoff stack with included ranges | Keeps parent and child grammars scoped |
| Binary AST transport | Dictionary encoded subtree chunks with independent compression | Enables parallel delivery, retry, cache reuse, and subtree preprocessing |

## Risks And Mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Full JSON Schema needs non-streaming features | Some schemas require buffering or delayed decisions | Mark streamable subset, allow bounded buffering, report non-streamable schema features |
| Unicode positions differ by ecosystem | Diagnostics can point to wrong columns | Store byte offsets and derive code point/UTF-16 columns on demand |
| Source maps are lost across transformations | Diagnostics and tooling cannot traverse back to the original source | Store source-map stacks on every AST element and append a frame for each context transform |
| Compressed source maps stop at byte boundaries | Bit-packed compression metadata cannot be traced precisely | Support bit offset/length ranges for compressed binary frames |
| Whole-AST compression blocks parallelism | One corrupt or missing byte can stall the whole document | Compress subtree-root chunks independently and use hashes for missing-chunk retry |
| Shared dictionaries drift by platform version | Chunks decode differently across runtimes | Version platform and app dictionaries and include required dictionary ids per chunk |
| Cross-chunk references arrive late | Parallel chunks decode before dependencies exist | Represent dependencies as scoped slots and fill them when defining chunks arrive |
| Network chunk size is fixed too early | Broadcast or high-loss paths get poor repair and scheduling behavior | Negotiate chunk ranges, adjust grouping in flight, and cache path profiles per client/server/network |
| Embedded content boundaries are ambiguous | Child parser can consume parent syntax | Parent owns return condition and passes bounded child source |
| HTML recovery is format-specific | Generic parser produces browser-incompatible trees | Use WHATWG states for HTML rather than XML-style recovery |
| Derivative states or scoped slots can grow | Memory/time blowups | Memoize residuals, intern schema states, and compact scope tables as slots are finalized |
| CSV dialect variance | Incorrect field splitting | Make dialect part of schema and scanner configuration |

## References

- TypeScript parser source: <https://github.com/microsoft/TypeScript/blob/main/src/compiler/parser.ts>
- TypeScript scanner source: <https://github.com/microsoft/TypeScript/blob/main/src/compiler/scanner.ts>
- typescript-eslint parser: <https://typescript-eslint.io/packages/parser/>
- Babel parser: <https://babeljs.io/docs/babel-parser>
- Oxc parser guide: <https://oxc.rs/docs/guide/usage/parser>
- LLVM programmer manual: <https://llvm.org/docs/ProgrammersManual.html>
- Clang SourceManager: <https://clang.llvm.org/doxygen/classclang_1_1SourceManager.html>
- LLVM MemoryBuffer: <https://www.llvm.org/docs/doxygen/classllvm_1_1MemoryBuffer.html>
- XML 1.0: <https://www.w3.org/TR/xml/>
- Invisible XML specification: <https://www.w3.org/community/reports/ixml/CG-FINAL-ixml-20231212/>
- Invisible XML current draft: <https://invisiblexml.org/current/>
- Xerces2 Java: <https://xerces.apache.org/xerces2-j/>
- Xerces XNI parser configuration: <https://xerces.apache.org/xerces2-j/xni-config.html>
- Java StAX API tutorial: <https://docs.oracle.com/javase/tutorial/jaxp/stax/api.html>
- RELAX NG derivative validation: <https://relaxng.org/jclark/derivative.html>
- Apache Daffodil: <https://daffodil.apache.org/>
- DFDL specification: <https://daffodil.apache.org/docs/dfdl/>
- Jackson core streaming abstractions: <https://github.com/FasterXML/jackson-core>
- simdjson basics: <https://simdjson.org/api/2.0.0/md_doc_basics.html>
- JSON Schema core draft 2020-12: <https://json-schema.org/draft/2020-12/json-schema-core>
- WHATWG HTML parsing: <https://html.spec.whatwg.org/multipage/parsing.html>
- CSS Syntax Module Level 3: <https://www.w3.org/TR/css-syntax-3/>
- Rust cssparser crate: <https://docs.rs/cssparser/latest/cssparser/>
- Rust identifiers reference: <https://doc.rust-lang.org/stable/reference/identifiers.html>
- rustc_lexer documentation: <https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lexer/index.html>
- Tree-sitter documentation: <https://tree-sitter.github.io/tree-sitter/>
- Zstandard compression and `application/zstd`: <https://www.rfc-editor.org/rfc/rfc8878.html>
- Brotli compressed data format: <https://www.rfc-editor.org/rfc/rfc7932>
- DEFLATE compressed data format: <https://www.rfc-editor.org/rfc/rfc1951.html>
- Datagram Packetization Layer PMTUD: <https://www.rfc-editor.org/rfc/rfc8899>
- HTTP/3 framing over QUIC streams: <https://www.rfc-editor.org/rfc/rfc9114.html>
- Linux `splice(2)`: <https://man7.org/linux/man-pages/man2/splice.2.html>
- Linux `mmap(2)`: <https://man7.org/linux/man-pages/man2/munmap.2.html>
- Linux io_uring zero-copy receive: <https://docs.kernel.org/networking/iou-zcrx.html>
- Windows `CreateFileMapping`: <https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-createfilemappinga>
- Windows `MapViewOfFile`: <https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-mapviewoffile>
- Windows `TransmitFile`: <https://learn.microsoft.com/en-us/windows/win32/api/mswsock/nf-mswsock-transmitfile>
- Windows Registered I/O request queues: <https://learn.microsoft.com/en-us/windows/win32/winsock/riorqueue>
- macOS `mmap(2)`: <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/mmap.2.html>
- macOS `sendfile(2)`: <https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/sendfile.2.html>
