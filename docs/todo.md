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
- [ ] Wire `build`, `lint`, `test`, and future `validate-fixtures` Nx targets through `cem_ml` / `cem_ml_cli`.
- [ ] Update the root README package map, `docs/index.md`, and related package docs to name `@epa-wg/cem-ml` and
      `@epa-wg/cem-ml-cli` as the active parser/runtime and CLI packages.
- [ ] Verify `cem-ml-cli` covers the planned feature summary from [`cem-ml-cli-contract.md`](cem-ml-cli-contract.md):
      `parse`, `validate`, `check`, `inspect`, `convert`, `trace`, `bench`, `fixture validate`, `fixture roundtrip`,
      `help`, `version`, and reserved `transform`, `schema`, and `plugin` workflows.
- [ ] Verify option-function coverage in `cem-ml-cli`: fail level, output format, report destinations, output file,
      schema/content-type/base URI recording, quiet/verbose/no-color, zero hard violations, preserve source offsets,
      convert input/output formats, inspect views, benchmark iterations/budget/profile/cold-cache, and fixture defaults.
- [ ] Add Rust-side feature tests for option behavior and output shape before relying on `cem-ml-cli` in CI. Tests
      should assert behavior, JSON/report fields, diagnostics, and exit codes.
- [ ] Move fixture validation report ownership to `packages/cem_ml_cli/dist/cem-ml.report.{json,md}` and round-trip /
      benchmark reports to `cem-ml`-named outputs.

### Planning And Contract Reconciliation

- [ ] Update parser/runtime acceptance criteria so Tier A matches the layered runtime contract: byte decoding,
      tokenization, normalized events, schema machine, typed AST, source-map stacks, transform, and diagnostics.
- [ ] Define the first public runtime interfaces: `ByteSource`, `DecodedChunk`, `SchemaToken`, `NormalizedEvent`,
      `SchemaFrame`, `CemAstNode`, `SourceMapFrame`, `Diagnostic`, and `Interpreter`.
- [ ] Decide and document Tier A deferrals for compression, multi-content plugins, full WHATWG DOM compatibility,
      thread pools, and Rust/WASM outputs.

### Byte Source And Encoding Decoder

- [ ] Implement the `cem_ml::source` byte-source boundary for in-memory byte buffers, strings, files, and async byte
      streams.
- [ ] Implement the `cem_ml::source::decode` boundary preserving absolute byte offsets, decoded scalar ranges, and
      derived line/column positions.
- [ ] Add UTF-8 validation and XML/HTML-compatible character diagnostics for fixture inputs.
- [ ] Unit test chunk boundaries, BOM handling, invalid byte sequences, and line/column projection.

### Tokenizer And Event Normalizer

- [ ] Implement the canonical CEM-ML curly tokenizer profile for `{name @attributes | content...}`, relaxed content
      boundaries, `$` expression nodes, anonymous typed scopes, directives, comments, and rich-content enclosures.
- [ ] Implement CEM-ML schema-scoping forms: `@schema` prelude shorthand, `{cem:schema @cem:name=...}` inline
      declarations, `{cem:schema @src=...}` / `@select=...` switches, host-node `@cem:schema-src` /
      `@cem:schema-select`, and scope-chain `cem:name` shadowing.
- [ ] Implement namespace binding rebinding for repeated prefix names and the blank/default binding, preserving the
      expanded namespace identity active at each source position.
- [ ] Reject bare `{...}` text interpolation in CEM-ML content and accept `{...}` cem-ql spans only in
      template-aware attribute-value mode.
- [ ] Implement HTML/XML tokenizer profiles as secondary parity paths that emit start, attribute, text, comment,
      processing-instruction, and end events without constructing the implementation DOM.
- [ ] Normalize tokenizer output into shared event categories: open scope, close scope, name, value, separator, mode
      switch, error, and transform.
- [ ] Preserve byte spans and source-map frames on every token and normalized event.
- [ ] Add tokenizer fixtures for nested CEM-ML nodes, explicit and relaxed content boundaries, `$` expression nodes,
      attribute `{...}` cem-ql spans, comments, and rich-content enclosures.
- [ ] Add schema-scoping fixtures covering inline declarations, sibling-position switches, wrapping switches,
      host-node switches, `src`/`select` exclusivity, and nested `cem:name` shadowing.
- [ ] Add namespace rebinding fixtures covering unprefixed HTML, unprefixed SVG, and rebinding the default namespace
      back to HTML in one CEM-ML document with XML parity output.
- [ ] Add fixtures proving canonical CEM-ML, HTML parity, and XML-like inputs normalize into the same CEM event model
      where their semantic shape is equivalent.

### Schema Machine

- [ ] Author the active CEM schema source covering the vocabulary used by the existing five fixtures:
      `data-cem-screen`, `data-cem-form`, `data-cem-action`, plus implied field/list/thread shapes. Cross-reference
      component IDs from [`component-mvp.md`](component-mvp.md).
- [ ] Define the allowed state attribute set against the [`component-mvp.md`](component-mvp.md) state matrix.
- [ ] Compile schema markdown to XHTML via the existing docs pipeline.
- [ ] Compile CEM schema rules into streaming schema frames that track required names, multiplicity, ordering,
      allowed states, references, and expected closes.
- [ ] Mark non-streamable schema features explicitly and report them instead of silently buffering unbounded input.

### Scoped Handoffs And Embedded Content

- [ ] Define Tier A handoff records with content type, schema id, source span, inherited context, and parent-owned
      return condition.
- [ ] Add validation behavior for embedded `style`, `script`, XML CDATA/schema-tagged text, CSF-like fields, and JSON
      string subdocuments.
- [ ] For Tier A, diagnostic-only handling is acceptable for unsupported child parsers if the parent bounds and reports
      the embedded region correctly.
- [ ] Add tests that a child parser cannot consume past the parent-owned close condition.

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
