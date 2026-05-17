# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).

## Phase 2 - Schema-Defined Parser And Document Runtime (`@epa-wg/cem-ml` / `@epa-wg/cem-ml-cli`)

Bring the canonical fixtures in `examples/cem-ml/` and the HTML parity fixtures in
`examples/semantic/` into a layered schema-defined pipeline:

```text
ByteSource
  -> EncodingDecoder
  -> SchemaTokenizer
  -> EventNormalizer
  -> SchemaMachine
  -> InterpreterAstBuilder
  -> BinaryAstEncoder
  -> ImplementationInterpreter / Transform
```

Acceptance criteria: [`cem-ml-ac.md`](cem-ml-ac.md). Plan: [`cem-ml-library-plan.md`](cem-ml-library-plan.md).
Component vocabulary: [`component-mvp.md`](component-mvp.md). Research input:
[`../parsing-algorithms-research.md`](../parsing-algorithms-research.md).

### Package Direction

- [x] Scaffold `packages/cem_ml` and `packages/cem_ml_cli`.
- [x] Wire `build`, `lint`, `test` Nx targets through `cem_ml` / `cem_ml_cli`. `validate-fixtures` is plan-gated to
      Phase 12 in [`cem-ml-cli-plan.md`](cem-ml-cli-plan.md) §10/§12 — add only when the real parser engine exists.
- [x] Update the root README package map, `docs/index.md`, and related package docs to name `@epa-wg/cem-ml` and
      `@epa-wg/cem-ml-cli` as the active parser/runtime and CLI packages.
- [x] Scaffold `cem-ml-cli` clap command surface for the planned feature summary in
      [`cem-ml-cli-contract.md`](cem-ml-cli-contract.md): `parse`, `validate`, `check`, `inspect`, `convert`, `trace`,
      `bench`, `fixture validate`, `fixture roundtrip`, `help`, `version`, and reserved `transform`, `schema`, and
      `plugin` workflows (reserved commands exit 2). Parser-backed handlers are stubs awaiting Phase 11
      [`cem-ml-cli-plan.md`](cem-ml-cli-plan.md).
- [x] Wire option-function coverage in `cem-ml-cli` clap layer: fail level, per-command format enums (`parse`,
      `validate`/`check`, `trace`, `bench`), `--show` inspect views, convert input/output layer formats,
      report destinations (`--report-json`/`--report-md`), schema/content-type/base URI recording,
      quiet/verbose (mutually exclusive)/no-color, zero hard violations, preserve source offsets,
      benchmark iterations/budget/profile/cold-cache (with `>=1` range validators), and fixture-default empty inputs.
      Backed by 17 unit tests in `packages/cem_ml_cli/src/cli.rs`.
- [x] Scaffold the `CemMlEngine` trait boundary in `cem-ml` (engine/diagnostics/report modules + `NotImplementedEngine`
      + feature-gated `FakeEngine`), route every parser-backed CLI command through it, and add Rust-side feature tests
      for option behavior and output shape: JSON/report fields (`generatedAt`, `summary.*Count`, `options.*` with
      `failLevel`/`schema`/`contentType`/`baseUri`), diagnostics, deterministic timestamp, `--out` vs stdout split,
      `--report-json`/`--report-md` writers, fixture default inputs, fail-level semantics, `--zero-hard-violations`,
      `--quiet`, and exit codes (0 success/stub, 1 hard failure, 2 reserved, 6 I/O).
- [x] Honor `cem-ml-cli-contract.md` §Report Ownership filename conventions: fixture-validate / validate / check write
      `cem-ml.report.{json,md}`, fixture-roundtrip writes `cem-ml.roundtrip.report.{json,md}`, bench writes
      `cem-ml.bench.report.json`. Basenames are applied when `--report-json` / `--report-md` resolve to a directory;
      explicit file paths are honored verbatim. Covered by 4 dispatch tests in `packages/cem_ml_cli/src/dispatch.rs`
      (`fixture_validate_with_dir_uses_default_basename`, `fixture_roundtrip_with_dir_uses_roundtrip_basename`,
      `bench_with_dir_uses_bench_basename`, `report_explicit_file_path_overrides_basename`).

### Planning And Contract Reconciliation

- [x] Added `AC-F-10` to [`cem-ml-ac.md`](cem-ml-ac.md) binding Tier A to the eight-layer runtime contract
      (`ByteSource`/`EncodingDecoder` → `SchemaTokenizer` → `EventNormalizer` → `SchemaMachine` →
      `HandoffStack` → `InputDomAstBuilder` → `BinaryAstEncoder` interface → `Interpreter`/Transform) plus the
      cross-cutting `SourceMapStack` + `Diagnostic` contracts. Verification fixtures `AC-F-V-7` and `AC-F-V-8`
      assert the public boundary at compile time.
- [x] Scaffolded the first public runtime interfaces in `packages/cem_ml/src/`: `source` (`ByteSource`,
      `EncodingDecoder`, `DecodedChunk`, `ByteRange`, `SourceId`, `Encoding`), `source_map` (`SourceMapFrame`,
      `SourceMapStack`, `TransformKind`), `tokenizer` (`SchemaToken`, `SchemaTokenizer`), `events`
      (`NormalizedEvent`, `EventNormalizer`, `QName`, `HandoffRecord`), `schema` (`SchemaFrame`, `SchemaMachine`,
      `SchemaVersionIdentity`), `handoff` (`HandoffStack`), `parser` (`CemAstNode`, `InputDomAstBuilder`,
      `NameSlot`, `ExpandedName`), `ast` (`BinaryAstEncoder` interface stub), `interpreter` (`Interpreter`,
      `OutputTarget`, `TransformOutput`). Verified by `layered_runtime_contract_types_are_importable` unit test.
- [x] Added `AC-F-11` documenting Tier A deferrals: binary AST chunk compression (Layer 8 body), multi-content
      plugin runtime (G-PLUG), full WHATWG DOM API compatibility, thread pools / bounded queues / external-I/O
      scheduler (G-EXT), and published Rust/WASM output artifacts (AC-C-*). Each deferral preserves its interface
      boundary in the public crate so future tiers can fill the body without re-shaping Tier A code.

### Byte Source And Encoding Decoder

- [x] `cem_ml::source` byte-source boundary: `BytesSource`, `StringSource`, `FileSource` implement the chunk-pull
      `ByteSource` trait with absolute byte offsets and a configurable `chunk_size` (default `MAX_SOURCE_CHUNK_BYTES`
      = 64 KiB). Async-stream wrapper around the same adapters is documented as a Phase 11 follow-up once the executor
      choice is finalized.
- [x] `cem_ml::source::decode` boundary: `Utf8Decoder` is a streaming UTF-8 decoder with bounded carry-over (up to 3
      bytes for an in-progress sequence), BOM detection (UTF-8 / UTF-16LE / UTF-16BE), preservation of absolute byte
      offsets on every decoded scalar, and `LineIndex` for byte-offset → 1-based (line, column) projection.
- [x] Source-layer diagnostics with `byteOffset` projection: `cem.byte.invalid_utf8` (Error) on orphan continuation
      bytes / truncated sequences / invalid leads; `cem.byte.invalid_xml_char` (Warning, gated by
      `DecodeConfig.strict_xml_chars`) for scalars outside the XML 1.0 `Char` production;
      `cem.byte.unsupported_encoding` (Error) for UTF-16 / Latin-1 paths until full decoders land;
      `cem.byte.io_error` (Fatal) for transport failures.
- [x] Tests in `packages/cem_ml/src/source/` (23 unit tests total) cover: chunk boundaries (1-byte chunks splitting a
      2-byte UTF-8 sequence; chunk_size smaller than total length), BOM (UTF-8 skipped + selection recorded; UTF-16LE
      detected + flagged), invalid byte sequences (orphan continuation, truncated at EOF), restricted XML chars (NUL
      flagged when strict, ignored when relaxed), `LineIndex` projection (single line, multi-byte column advance,
      multi-line documents, newline-at-offset edge case), and `FileSource` round-trip through a tmp file.

### Tokenizer And Event Normalizer

- [x] Canonical CEM-ML curly tokenizer in `packages/cem_ml/src/tokenizer/cem.rs`. Handles `{name @attrs | content...}`
      with explicit `|` and relaxed (implied) content boundaries, `$` expression nodes (`{$ expr}` and `{$ | expr}`),
      anonymous typed scopes (`{@type=... | ...}`), directives (`@doc`, `@ns`, `@default`, `@schema`), line
      (`// ...`) and block (`/* ... */`) comments, rich-content enclosures (triple backticks, body verbatim),
      quoted-string + bare + AVT-span attribute values, and qualified attribute names (`cem:screen`).
- [x] Bare `{...}` text interpolation in CEM-ML content is rejected with `cem.tokenizer.bare_brace_text` and an
      `Error` token; `{...}` cem-ql spans are accepted only in attribute-value mode (passed through verbatim wrapped
      in braces so consumers see this is a span, not a literal).
- [x] HTML / XML tokenizer profile boundaries exist (`tokenizer::html::HtmlTokenizer`, `tokenizer::xml::XmlTokenizer`)
      and emit `cem.tokenizer.profile_not_implemented` until Phase 11 of `cem-ml-cli-plan.md` lands the WHATWG-state
      HTML tokenizer + XML 1.0 profile; downstream consumers can program against the trait today.
- [x] Event normalization in `packages/cem_ml/src/events/cem.rs` (`CemEventNormalizer`) lowers every token into the
      shared event categories `OpenScope` / `CloseScope` / `Name` / `Value` / `Trivia` / `Separator(ElementBoundary)`
      / `ProcessingInstruction` / `ModeSwitch` / `Error`. `@type="..."` attributes (incl. on anonymous scopes) emit a
      `ModeSwitch` alongside their `Name` + `Value` so the schema machine sees content-type handoffs without
      rescanning. Directives lower to `OpenScope @<name>` + `Value` + `CloseScope`.
- [x] Byte spans + source-map stacks are preserved on every token (`SchemaToken.byte_range`,
      `SchemaToken.source_map`) and propagate into every event. Tokens carry a `TransformKind::CemTokenizer` frame;
      the test `source_map_frames_carry_through_to_events` asserts that frames survive the lowering.
- [x] Tokenizer fixtures (15 unit tests in `tokenizer::cem::tests`) cover: simple node, attribute value forms (quoted,
      bare, boolean, AVT span), nested nodes, relaxed content boundary, `$` expression node (relaxed + explicit
      boundary), anonymous typed scope, directives, line + block comments, rich-content enclosure, qualified
      attribute name, byte-range absoluteness, bare-brace rejection, and a `login.cem` fixture parse + an
      all-canonical-fixtures smoke test that all 5 `examples/cem-ml/*.cem` files tokenize without hard violations.
- [ ] CEM-ML schema-scoping forms (`@schema` prelude, `{cem:schema @cem:name=...}` inline, `{cem:schema @src=...}` /
      `@select=...` switches, host-node `@cem:schema-src` / `@cem:schema-select`, scope-chain `cem:name` shadowing).
      Status: the tokenizer surfaces the underlying `Directive` and node/attribute tokens; semantic interpretation
      lives in the schema-machine block (`docs/cem-ml-ac.md` §AC-F-2 details, `cem-ml-stack-design.md` §13.1) and
      lands with that layer.
- [ ] Namespace binding rebinding for repeated prefix names and the blank/default binding. Status: tokens carry
      lexical names (`cem:screen`) without expansion; rebinding/expansion is owned by `cem_ml::schema::namespace`
      (`NsContext`, `NamespaceBinding`) in `cem-ml-stack-design-impl.md` §3.4.1 and tracked with the schema-machine
      block.
- [ ] Schema-scoping fixtures (inline declarations, sibling/wrapping switches, host-node switches, src/select
      exclusivity, nested `cem:name` shadowing). Blocked on schema-scoping form support above.
- [ ] Namespace rebinding fixtures (unprefixed HTML, unprefixed SVG, default-namespace rebinding back to HTML in one
      CEM-ML document with XML parity output). Blocked on namespace-binding support above.
- [ ] Fixtures proving canonical CEM-ML, HTML parity, and XML-like inputs normalize into the same CEM event model.
      Blocked on Phase 11 HTML/XML parity tokenizer profiles above.

### Schema Machine

- [x] Authored the active CEM schema source at [`../packages/cem_ml/schema/cem-core.md`](../packages/cem_ml/schema/cem-core.md).
      The CEM annotations now use the schema-qualified `cem:` namespace form per AC-S-9 (e.g. `cem:screen`,
      `cem:form`, `cem:action`, `cem:badge`, `cem:card`, `cem:list`, `cem:row`, `cem:thread`, `cem:message`) — not
      HTML `data-cem-*`. The five canonical fixtures already use this form. Cross-references
      [`component-mvp.md`](component-mvp.md) component IDs and state matrix.
- [x] Allowed state attribute set defined per annotation in `cem-core.md` and mirrored in
      `packages/cem_ml/src/schema/vocab.rs` (`AnnotationDef.allowed_states`). State is exposed via `cem:state="..."`
      with single or space-separated values.
- [ ] Compile schema markdown → XHTML via the existing docs pipeline. Status: schema markdown is authored and lives
      under `packages/cem_ml/schema/cem-core.md`; wiring it into the workspace `build:docs` Nx target is a follow-up
      (the Rust `CompiledSchema` is constructed programmatically in `vocab.rs::CompiledSchema::cem_core` until the
      markdown-driven compiler lands).
- [x] Compiled CEM schema rules into streaming schema frames: `packages/cem_ml/src/schema/machine.rs`
      (`CemSchemaMachine`) consumes the `NormalizedEvent` stream, pushes/pops `SchemaFrame`s at open/close-scope
      boundaries, validates annotation values against `AnnotationDef.allowed_values`, validates `cem:state` values
      against both the State Matrix and the active annotation's allowed states, and reports unbalanced opens/closes
      at finalize. Diagnostics: `cem.schema.unknown_annotation`, `cem.schema.unknown_annotation_value`,
      `cem.schema.disallowed_state`, `cem.schema.state_not_allowed_for_role`, `cem.schema.unclosed_scope`,
      `cem.schema.unbalanced_close`. Verified by an `all_canonical_fixtures_schema_validate_clean` test (5 fixtures,
      zero hard schema violations) plus per-rule unit tests.
- [x] Non-streamable schema features are explicit: `CompiledSchema.non_streamable_constraints` lists any rule that
      would require unbounded buffering (`AttributeOrderNonAdjacent`, `CrossScopePredicate`,
      `FullDocumentBuffering`). Tier A authors zero such rules; the machine emits
      `cem.schema.unsupported_constraint` at finalize for every constraint present, so authoring a non-streamable
      rule is detectable instead of silently degrading the parser. Verified by
      `non_streamable_constraints_emit_unsupported_constraint`.

### Scoped Handoffs And Embedded Content

- [x] Tier A `HandoffRecord` in `packages/cem_ml/src/events.rs` carries `content_type`, `schema_id`, `source_span`,
      `inherited_context: InheritedContext { schema_id, namespace_uri, parent_close_byte_offset }`, and
      `return_condition: ReturnCondition { ParentScopeClose | MatchingCloseTag(String) | EndOfStream }`. The schema
      machine fills `parent_close_byte_offset` from the active parent frame's `source_span.end()` when the handoff
      opens.
- [x] Embedded-content validation behavior: `packages/cem_ml/src/schema/machine.rs::on_mode_switch` drives a
      `HandoffStack` (`packages/cem_ml/src/handoff.rs`). Tier A supported content types
      (`text/html`, `text/css`, `text/javascript`, `application/json`, `text/xml`, `application/xml`) emit
      `cem.handoff.child_parser_deferred` (Info) noting the child parser lands in Phase 11; unknown types emit
      `cem.handoff.unsupported_content_type` (Error). Embedded `<style>` / raw-text `<script>` / XML CDATA / JSON
      string subdocuments / CSF-like field interpretation are the child-parser bodies deferred to Phase 11; the
      handoff *boundary* is enforced now.
- [x] Diagnostic-only handling for unsupported content types: the region is preserved as opaque text bounded by the
      parent scope's close, no `cem.schema.unclosed_scope` fires, and validation continues for the surrounding
      document. Verified by `unsupported_content_type_emits_error_but_region_is_bounded`.
- [x] Tests proving a child parser cannot consume past the parent-owned close condition:
      `child_parser_cannot_consume_past_parent_close` (synthetic `HandoffStack::within_bounds` check at the close
      boundary), `handoff_records_carry_inherited_context_with_parent_close_offset` (step-by-step machine inspection
      confirming `parent_close_byte_offset == parent.source_span.end()`), and `nested_scopes_pop_only_owned_handoffs`
      (an inner scope's handoff pops on inner close, leaving the outer scope unaffected and `handoffs_at_eof == 0`).

### AST, DOM Helpers, And Source Maps

- [ ] Implement `cem_ml::parser` as event stream to typed CEM AST, with semantic roles, state, labels, references,
      scope IDs, and unresolved reference slots.
- [ ] Implement query helpers in `cem_ml::query` for roles, state lookups, validation messages, label resolution,
      reference traversal, and source-map lookup.
- [ ] Attach source-map stacks to every AST node, generated node, diagnostic, and transform result.
- [ ] Unit test each fixture's parsed shape and source trace back to original byte offsets.

### Binary AST And Chunk Boundary Design

- [ ] Specify an uncompressed debug binary AST representation with dictionaries for node kinds, schema ids, strings,
      source-map frame shapes, scope slots, and typed values.
- [ ] Define subtree chunk metadata: root id, parent anchor, dictionary ids, local node/edge tables, source-map deltas,
      child links, external references, and integrity hash.
- [ ] Implement a minimal deterministic encoder used by tests only; compression profiles can remain deferred.
- [ ] Add round-trip tests from AST to debug binary encoding and back for the five fixtures.

### Validation

- [ ] Define a schema-owned semantic-rule catalog shape with rule id, owning schema/content type, trigger layer,
      required inputs, diagnostic defaults, and policy override hooks.
- [ ] Implement the first Tier A semantic-rule catalog for CEM UI projections over HTML/SVG/ARIA: accessible names,
      ARIA role/attribute compatibility, `id`/`for`/`aria-*` reference integrity, and SVG-in-HTML accessibility
      boundaries.
- [ ] Implement generic CEM semantic rules for invalid component state combinations, state-transition constraints,
      template/slot/schema reference integrity, and schema-owned open-content policy.
- [ ] Implement unsafe-content policy checks for inline scripts, event handlers, unsafe URL-bearing attributes,
      `srcdoc`, imports, XML external entities/DTDs, and other policy-gated resource hooks.
- [ ] Keep semantic validation extensible so CSS, JS, XML, JSON, plugin-loaded content, and future runtime content add
      rules through the same registry model.
- [ ] Implement structural validation checking unknown elements/attributes, unsupported handoffs, and non-streamable
      schema features.
- [ ] Emit `cem-ml.report.md` and `cem-ml.report.json`, mirroring the `validate-platforms.mjs` report
      convention.
- [ ] Add a `cem-ml-cli` fixture validation target that runs validation across `examples/cem-ml/*.cem` and
      `examples/semantic/*.html` parity fixtures and fails non-zero on hard violations.
- [ ] Add fixture-pair tests proving each `examples/cem-ml/*.cem` file and matching `examples/semantic/*.html` parity
      file produce the same hard-violation result and compatible diagnostics.
- [ ] Ensure validation diagnostics include `{ uri, line, column, byteOffset, code, severity, message, sourceMap }`.

### Transform

- [ ] Author a transform pipeline from validated semantic CEM AST/DOM to light-DOM custom-element markup compatible
      with `@epa-wg/custom-element`.
- [ ] Add a library/CLI transform helper that runs the transform over a fixture and returns the rendered HTML string.
- [ ] Preserve transform source-map frames for generated custom-element markup.
- [ ] Snapshot the transform output for each fixture under `test/__snapshots__/`.

### Cross-Surface Conversion

- [ ] Define exact CEM-ML ↔ XML/HTML conversion rules for namespaces, default namespace changes, comments,
      whitespace, typed scopes, rich content, `$` expression nodes, attribute cem-ql spans, and source maps.
- [ ] Add conversion tests proving canonical CEM-ML fixtures can project to XML/HTML parity forms and back without
      losing schema event identity or source-map traceability.

### Verification

- [ ] All five canonical CEM-ML fixtures and their HTML parity fixtures decode, tokenize, normalize, schema-validate,
      build a typed AST, validate clean, transform, and render successfully end to end.
- [ ] Canonical/parity fixture tests compare normalized event streams, validation results, canonical CEM-ML snapshots,
      and rendered light-DOM custom-element output.
- [ ] Every generated node in fixture output traces back to original source bytes or to the transform that generated it.
- [ ] `yarn build` includes `cem-ml` / `cem-ml-cli` build plus fixture validation when the real parser is enabled;
      report shows zero hard violations.
- [ ] Document the round trip in `cem-ml-cli` docs with a worked example using `login.html`.

### Authoring Tooling

- [ ] Publish a machine-readable CEM-ML lexical grammar and keep it synchronized with tokenizer fixtures.
- [ ] Add syntax highlighting coverage for nodes, attributes, namespaces, content markers, `$` expression scopes, rich
      content, comments, and diagnostics.
- [ ] Add a tree-sitter grammar or equivalent editor parse grammar that round-trips with tokenizer fixtures.
- [ ] Add formatter and Prettier-like rules for indentation, canonical `|` insertion, attribute ordering, quote/rich
      enclosure normalization, and comment/whitespace preservation.
- [ ] Add lint diagnostics for unbound prefixes, invalid relaxed-boundary use, suspicious content-type switches,
      noncanonical delimiter choices, and forbidden bare `{...}` text interpolation.
