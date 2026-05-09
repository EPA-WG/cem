# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in [`../roadmap.md`](../roadmap.md).

## Immediate - Graceful `packages/cem-dom` Removal

`@epa-wg/cem-dom` is deprecated. `@epa-wg/cem-ml` and `@epa-wg/cem-ml-cli` are the forward path for parser/runtime
and CLI work. The old TypeScript implementation is not a compatibility target and can be removed completely.

The removal goal is **functional coverage**, especially CLI option behavior and output contracts. Do not preserve
`cem-dom` command syntax for its own sake; preserve capabilities, option semantics, diagnostics, reports, fixture
workflows, and exit-code behavior where they still matter.

### Removal Gates

- [x] Freeze the useful `cem-dom` CLI capability contract into `cem-ml-cli` docs before deleting source files:
      command capabilities, option semantics, report fields, diagnostic fields, fixture defaults, default formats,
      fail-level behavior, reserved workflow behavior, and exit codes.
- [x] Treat the former package-local CLI docs and [`cem-ml-cli-parity-matrix.md`](cem-ml-cli-parity-matrix.md) as
      migration inputs only. The normative functional contract lives in
      [`cem-ml-cli-contract.md`](cem-ml-cli-contract.md).
- [ ] Verify `cem-ml-cli` covers the former CLI functionality, regardless of exact syntax:
      `parse`, `validate`, `check`, `inspect`, `convert`, `trace`, `bench`, `fixture validate`, `fixture roundtrip`,
      `help`, `version`, and reserved `transform`, `schema`, and `plugin` workflows.
- [ ] Verify option-function coverage in `cem-ml-cli`: fail level, output format, report destinations, output file,
      schema/content-type/base URI recording, quiet/verbose/no-color, zero hard violations, preserve source offsets,
      convert input/output formats, inspect views, benchmark iterations/budget/profile/cold-cache, and fixture defaults.
- [ ] Add Rust-side contract tests for option behavior and output shape before relying on `cem-ml-cli` in CI. Tests
      should assert behavior, JSON/report fields, diagnostics, and exit codes rather than copying `cem-dom` syntax.
- [ ] Move fixture validation report ownership to `packages/cem_ml_cli/dist/cem-ml.report.{json,md}` and round-trip /
      benchmark reports to `cem-ml`-named outputs.
- [x] Remove `packages/cem-dom` from the Nx workspace and package graph: project config, package metadata, source,
      tests, docs that only describe the old implementation, generated `dist` artifacts, README package-map entries,
      docs index entries, and package references from component docs.
- [ ] Keep or move only migration-relevant contract material. Anything tied to the old TypeScript parser/validator
      implementation can be deleted.
- [x] Verify workspace discovery and Rust targets after removal:
      `yarn nx show projects`, `yarn nx run cem_ml:build`, `yarn nx run cem_ml:test`,
      `yarn nx run cem_ml_cli:build`, and `yarn nx run cem_ml_cli:test`.

## Phase 2 - Schema-Defined Parser And Document Runtime (`@epa-wg/cem-ml` / `@epa-wg/cem-ml-cli`)

Bring the existing fixtures in `examples/semantic/` into a layered schema-defined pipeline:

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
- [x] Remove the deprecated `packages/cem-dom` sub-project.
- [ ] Wire `build`, `lint`, `test`, and future `validate-fixtures` Nx targets through `cem_ml` / `cem_ml_cli`.
- [ ] Update the root README package map, `docs/index.md`, and related package docs to name `@epa-wg/cem-ml` and
      `@epa-wg/cem-ml-cli` as the active parser/runtime and CLI packages.

### Planning And Contract Reconciliation

- [ ] Update parser/runtime acceptance criteria so Tier A matches the layered runtime contract: byte decoding,
      tokenization, normalized events, schema machine, typed AST, source-map stacks, transform, and diagnostics.
- [x] Replace the old DOM library plan with [`cem-ml-library-plan.md`](cem-ml-library-plan.md), making Rust ownership
      explicit and no longer treating `@epa-wg/cem-dom` as an active package.
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

- [ ] Implement an HTML/XML tokenizer profile that emits start, attribute, text, comment, processing-instruction, and
      end events without constructing the implementation DOM.
- [ ] Normalize tokenizer output into shared event categories: open scope, close scope, name, value, separator, mode
      switch, error, and transform.
- [ ] Preserve byte spans and source-map frames on every token and normalized event.
- [ ] Add fixtures proving HTML and XML-like inputs normalize into the same CEM event model where their semantic shape
      is equivalent.

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

- [ ] Implement validation checking unknown elements/attributes, invalid state combinations, missing
      accessible names, broken `id`/`for`/`aria-*` references, unsafe content, unsupported handoffs, and non-streamable
      schema features.
- [ ] Emit `cem-ml.report.md` and `cem-ml.report.json`, mirroring the `validate-platforms.mjs` report
      convention.
- [ ] Add a `cem-ml-cli` fixture validation target that runs validation across `examples/semantic/*.html` and fails
      non-zero on hard violations.
- [ ] Ensure validation diagnostics include `{ uri, line, column, byteOffset, code, severity, message, sourceMap }`.

### Transform

- [ ] Author a transform pipeline from validated semantic CEM AST/DOM to light-DOM custom-element markup compatible
      with `@epa-wg/custom-element`.
- [ ] Add a library/CLI transform helper that runs the transform over a fixture and returns the rendered HTML string.
- [ ] Preserve transform source-map frames for generated custom-element markup.
- [ ] Snapshot the transform output for each fixture under `test/__snapshots__/`.

### Verification

- [ ] All five fixtures decode, tokenize, normalize, schema-validate, build a typed AST, validate clean, transform, and
      render successfully end to end.
- [ ] Every generated node in fixture output traces back to original source bytes or to the transform that generated it.
- [ ] `yarn build` includes `cem-ml` / `cem-ml-cli` build plus fixture validation when the real parser is enabled;
      report shows zero hard violations.
- [ ] Document the round trip in `cem-ml-cli` docs with a worked example using `login.html`.
