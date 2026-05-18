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

- [x] Event stream → typed CEM AST: `packages/cem_ml/src/parser/builder.rs` (`CemAstBuilder<E: EventNormalizer>`).
      Drives a flat `CemDocument` arena (`parser/document.rs`) where every node carries an `AstNodeId` index. Builds
      `Document` / `Element` / `Attribute` / `Text` / `Whitespace` / `Comment` / `ProcessingInstruction` / `Error`
      variants. `Element` records attributes (including CEM annotations) and children separately. `id=` attributes
      populate `id_table`; `for=` / `aria-labelledby=` / `aria-describedby=` / `aria-controls=` resolve through it
      and unresolved slots get queued. Semantic roles surface via the existing `cem:` namespace prefix carried on
      attributes; state surfaces via `cem:state="..."` (single or space-separated values).
- [x] Query helpers in `packages/cem_ml/src/query.rs`: `find_by_id`, `find_by_local_name`, `elements`,
      `attributes_in_prefix`, `cem_annotations`, `elements_with_annotation`, `state_of`, `resolve_reference`,
      `origin_byte_range`, `source_map_frames`, `validation_messages`.
- [x] Source-map stacks on every AST node: the builder pushes a `TransformKind::CemAstBuilder` frame onto the
      tokenizer-rooted stack inherited from the active parent, so every Element/Attribute/Text/Trivia/Error node
      walks origin-first back to its byte range. Unresolved reference slots carry their attribute's source-map stack.
- [x] Per-fixture parse-shape + byte-offset tracing tests (12 unit tests in `parser::builder::tests`): document root
      at node 0, nested-child linkage, attribute value recording, boolean attribute (no value), `id`+`for` round
      trip through `id_table`, unresolved-reference Warning, `cem_annotations` filters out `cem:state`,
      `state_of` parses single + space-separated values, `elements_with_annotation` enumerates every screen,
      `fixture_login_cem_parses_into_expected_shape` asserts `main` + `cem:screen` + `cem:form` presence,
      `every_canonical_fixture_parses_without_ast_hard_violations` exercises all 5 fixtures, and
      `origin_byte_range_traces_to_source_bytes` round-trips a text node's byte range back through the source
      bytes to confirm the slice contains the expected content.

### Binary AST And Chunk Boundary Design

- [x] Uncompressed debug binary format specified in `packages/cem_ml/src/ast/format.rs` with magic `CEMB`, version `1`,
      and 4 dictionaries: strings (length-prefixed UTF-8), source ids (`SourceId.0` raw), transforms (tag + optional
      string-payload index), and source-map frames (source-id-dict + span-kind + ranges + transform-dict). Node kinds
      are stable `u8` tags (`Document`/`Element`/`Attribute`/`Text`/`Whitespace`/`Comment`/`ProcessingInstruction`/
      `Cdata`/`RawText`/`Error`). Typed values currently surface as strings; the format is forward-compatible for
      typed-value tags via the `FLAGS_NONE` reserved field.
- [x] Subtree chunk metadata (`ChunkMetadata`) records: `root_id`, optional `parent_anchor`, `dictionary_ids` (Tier A
      always `[0]`), `local_node_start` + `local_node_count`, `source_map_deltas`, `child_links`, `external_references`,
      and an `integrity_hash` (FNV-1a 64-bit over the byte prefix). Tier A emits a single whole-document chunk; the
      deltas/links/external-refs lists are always empty until the Phase 11 streaming encoder lands.
- [x] Minimal deterministic encoder in `packages/cem_ml/src/ast/encode.rs` (`DebugBinaryEncoder`): strings/source-ids/
      transforms/source-map frames are interned by first-seen order during a deterministic pre-walk, then nodes /
      edges / `id_table` / unresolved-slots / chunk metadata sections write in fixed order. The encoder is gated as
      `#[doc(hidden)]` — it's not a compatibility-stable production format. Mirror `DebugBinaryDecoder` in
      `packages/cem_ml/src/ast/decode.rs` verifies the trailing FNV-1a hash before parsing and produces a
      `CemDocument` byte-identical (after re-encode) to the original.
- [x] Round-trip tests (8 unit tests in `ast::tests`): magic + version header inspection, empty input, simple element,
      nested element with `id`/`for` references, encoder determinism (same input → byte-identical output), integrity
      hash protects against single-byte tampering (`IntegrityMismatch` returned), chunk-metadata field assertions,
      and `every_canonical_fixture_round_trips` exercising all 5 `examples/cem-ml/*.cem` fixtures end-to-end with a
      byte-stability re-encode check.

### Validation

- [x] Schema-owned semantic-rule catalog shape: `packages/cem_ml/src/validation.rs` defines `RuleDescriptor { id,
      owning_scope, content_type, trigger_layer, required_inputs, default_severity, policy_overridable }`, `RuleId`,
      `TriggerLayer { Tokenizer | SchemaMachine | Document | CrossDocument }`, `RuleInput { CemDocument |
      SchemaFrames | NormalizedEvents | Policy }`, and the `SemanticRule` trait. `RuleRegistry::with_tier_a_rules()`
      registers the Tier A catalog; `RuleContext { document, upstream_diagnostics }` is the bound input every rule
      receives.
- [x] First Tier A semantic-rule catalog covering CEM UI projections over HTML / ARIA in
      `packages/cem_ml/src/validation/rules.rs`: `cem.ref.unresolved_reference` (`for` / `aria-labelledby` /
      `aria-describedby` / `aria-controls` / `aria-owns` against `id_table`), and `cem.a11y.accessible_name_missing`
      (interactive elements `button` / `a` / `select` / `textarea` need text, `aria-label`, `aria-labelledby`, or
      `title`). Full ARIA role/attribute compatibility matrix + SVG-in-HTML accessibility boundaries are scoped to
      the next validation iteration that lands a complete ARIA table.
- [x] Generic CEM rule: `cem.state.invalid_combination` rejects disallowed state pairs on the same element
      (`disabled+loading`, `disabled+active`, `disabled+hover`, `disabled+focus-visible`, `disabled+selected`,
      `empty+loading`). Template/slot/schema reference integrity + state-transition constraints + schema-owned
      open-content policy remain as follow-up rules that bolt onto the same `RuleRegistry`.
- [x] Unsafe-content policy: `cem.unsafe.javascript_url` (case-insensitive `javascript:` URL match on `href` / `src`
      / `action` / `formaction` / `xlink:href` / `ping` / `data`) and `cem.unsafe.event_handler_attribute` (any
      `on<alpha>...` attribute). `srcdoc` / imports / XML external entities / DTD hooks remain follow-up rules in
      the same registry once the parity tokenizers expose those constructs.
- [x] Extensibility through one registry: adding a new rule (CSS / JS / XML / JSON / plugin-loaded content) is
      `impl SemanticRule + RuleRegistry::register()`. `RuleContext` carries the document + accumulated upstream
      diagnostics so rules can short-circuit when an upstream layer already failed.
- [x] Structural validation: unknown / disallowed CEM annotations and `cem:state` values surface via the schema
      machine's `cem.schema.unknown_annotation` / `cem.schema.unknown_annotation_value` /
      `cem.schema.disallowed_state` codes (registered earlier in the schema-machine block). Unsupported handoffs
      surface as `cem.handoff.unsupported_content_type`. Non-streamable schema features surface as
      `cem.schema.unsupported_constraint`. `validation::run()` is the single entry point that runs every layer.
- [x] Diagnostics include `{ uri, line, column, byteOffset, code, severity, message, sourceMap, node }`. The new
      `Diagnostic.source_map` field is serialized as `sourceMap` per the contract; rules attach the node's
      `SourceMapStack` and project `byteOffset` from its origin frame. `line` / `column` remain projections via
      `LineIndex` (Tier A reporters compute them on demand).
- [ ] Emit `cem-ml.report.md` and `cem-ml.report.json` mirroring `validate-platforms.mjs`. Status: the CLI dispatch
      layer's report writers already emit JSON + Markdown files at the contract-named locations
      (`packages/cem_ml_cli/dist/cem-ml.report.{json,md}` when a directory destination is given); exact byte parity
      with `validate-platforms.mjs` is a separate docs/parity follow-up that should land alongside the
      `validate-fixtures` Nx target.
- [ ] `cem-ml-cli` `validate-fixtures` Nx target. Plan-gated to Phase 12 of
      [`cem-ml-cli-plan.md`](cem-ml-cli-plan.md); landing this requires the parser-enabled engine + the markdown
      report parity above.
- [ ] Fixture-pair tests proving canonical CEM-ML and HTML parity fixtures produce the same hard-violation result.
      Blocked on the Phase 11 HTML parity tokenizer; the Tier A canonical fixtures already validate clean end-to-end
      via `every_canonical_fixture_validates_clean`.

### Transform

- [x] Light-DOM transform pipeline in `packages/cem_ml/src/interpreter/light_dom.rs` (`LightDomInterpreter`). Renders
      each AST element as its native HTML open/close tag, preserves CEM annotations as `cem:*` attributes on the
      host element (compatible with `@epa-wg/custom-element` consumers attaching behavior to existing nodes), emits
      void-element open-only forms for the HTML5 void set, drops `@`-prefixed directive scopes, HTML-escapes text
      and attribute values, and sorts attributes deterministically by `(namespace, local_name)` so output is
      byte-stable.
- [x] Library transform helper `interpreter::light_dom::render_html(input: &str) -> TransformOutput` runs
      tokenize → normalize → AST build → render in one call and merges every layer's diagnostics into the output.
      The CLI side already routes through `dispatch::run_inspect` / `run_convert` for the underlying boundary; a
      `cem-ml transform` subcommand is reserved (`Command::Transform`) and will plug into this helper when the
      reserved subcommand is unfrozen.
- [x] Transform source-map frames preserved on generated markup: every emitted byte run is recorded as an
      `OutputSpan { output_range, origin: SourceMapStack }`. The origin stack carries the originating AST node's
      frames plus an appended `TransformKind::InterpreterRender` frame whose span covers the output bytes, so any
      generated byte projects back to a source byte range. Verified by `output_spans_cover_emitted_bytes`,
      `output_spans_carry_interpreter_render_frame`, and `output_spans_trace_back_to_source_byte_range`.
- [x] Snapshot tests at `packages/cem_ml/tests/transform_snapshots.rs` with one snapshot per fixture under
      `packages/cem_ml/tests/__snapshots__/{login,registration,profile,assets-list,message-thread}.html`.
      Regenerate via `CEM_ML_UPDATE_SNAPSHOTS=1 cargo test -p cem-ml --test transform_snapshots`. Default run
      compares byte-for-byte and fails with a regenerate hint on mismatch.

### Cross-Surface Conversion

- [ ] Define exact CEM-ML ↔ XML/HTML conversion rules for namespaces, default namespace changes, comments,
      whitespace, typed scopes, rich content, `$` expression nodes, attribute cem-ql spans, and source maps.
- [ ] Add conversion tests proving canonical CEM-ML fixtures can project to XML/HTML parity forms and back without
      losing schema event identity or source-map traceability.

### Verification

- [x] All five canonical CEM-ML fixtures decode → tokenize → normalize → schema-validate → AST build → validate
      → transform → render with zero hard violations end-to-end. Driven by
      `packages/cem_ml/tests/end_to_end.rs::every_canonical_fixture_runs_through_every_layer`, which checks each
      layer's diagnostics independently. HTML parity-fixture end-to-end remains blocked on the Phase 11 HTML
      tokenizer (`packages/cem_ml/src/tokenizer/html.rs` is a stub today).
- [x] Canonical fixture pipeline tests in `packages/cem_ml/tests/end_to_end.rs` compare open/close-event balance,
      schema-validation outcome, light-DOM render byte stream (against
      `packages/cem_ml/tests/__snapshots__/{stem}.html`), and re-run determinism. The HTML parity-fixture comparison
      half remains blocked on the Phase 11 HTML tokenizer; the canonical-side checks are runnable today and exercise
      every layer per fixture.
- [x] Every generated node in fixture output traces back to original source bytes or to the transform that generated
      it. `every_output_span_traces_to_source_or_to_a_transform_frame` walks every `OutputSpan.origin` and asserts
      the stack contains at least one of `CemTokenizer` / `HtmlTokenizer` / `XmlTokenizer` / `EventNormalizer` /
      `CemAstBuilder` / `InterpreterRender` frames. The byte-coverage half of
      `every_canonical_fixture_runs_through_every_layer` further asserts every emitted output byte is covered by at
      least one source-map span.
- [ ] `yarn build` includes `cem-ml` / `cem-ml-cli` build plus fixture validation. Status: `nx build` / `nx test` /
      `nx lint` targets are already wired through `cem_ml` / `cem_ml_cli` (see Package Direction block above), so the
      Rust build is part of `yarn build` once the workspace bundles those targets. The `validate-fixtures` Nx target
      that runs fixture validation end-to-end remains plan-gated to Phase 12 of
      [`cem-ml-cli-plan.md`](cem-ml-cli-plan.md) (the parser-enabled engine has to replace `NotImplementedEngine` in
      the CLI before fixture validation is meaningful through `cem-ml-cli`).
- [x] Worked example documented at [`../packages/cem_ml_cli/docs/worked-example.md`](../packages/cem_ml_cli/docs/worked-example.md):
      walks the canonical `login.cem` fixture through every Tier A layer, shows the rendered light-DOM HTML
      snapshot, documents the source-map trace, and lists the intended CLI commands gated on the Phase 11
      parser-enabled milestone.

### Authoring Tooling

- [ ] Publish a machine-readable CEM-ML lexical grammar and keep it synchronized with tokenizer fixtures.
- [ ] Add syntax highlighting coverage for nodes, attributes, namespaces, content markers, `$` expression scopes, rich
      content, comments, and diagnostics.
- [ ] Add a tree-sitter grammar or equivalent editor parse grammar that round-trips with tokenizer fixtures.
- [ ] Add formatter and Prettier-like rules for indentation, canonical `|` insertion, attribute ordering, quote/rich
      enclosure normalization, and comment/whitespace preservation.
- [ ] Add lint diagnostics for unbound prefixes, invalid relaxed-boundary use, suspicious content-type switches,
      noncanonical delimiter choices, and forbidden bare `{...}` text interpolation.
