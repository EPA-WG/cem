# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Tasks

- [ ] Implement schema package loading and input-file validation for supported
      schemas, in schema package creation order. Definition of done for each
      schema: resolve content type to schema URL, load the schema package,
      select an explicit lifecycle parser/adaptor, validate source bytes
      against the schema-owned document model, surface diagnostics through
      `validate`/`check`, and add focused Rust coverage.
  - [x] Establish schema-owned validation examples and a reusable CLI fixture
        harness before implementing per-schema validators.
        For each schema package, add a few popular real-world use cases as
        checked-in example files under
        `packages/cem_ml/schema-packages/{schema-name}/v1/examples/`.
        Link those files from that schema package's `README.md`, document the
        matching CLI validation command, and use the same example files in CLI
        validation tests for that file type.
  - [x] Organize CLI validation coverage so schema sub-projects can reuse it:
        keep CLI argument-plumbing tests in `packages/cem_ml_cli/src/dispatch.rs`,
        move schema example validation into a table-driven integration test
        such as `packages/cem_ml_cli/tests/schema_validation_examples.rs`, and
        have that suite read schema-owned examples instead of duplicating inline
        `write_fixture` strings.
  - [x] Define the example fixture contract: each schema starts with at least
        valid basic, valid realistic/nested, and invalid diagnostic examples;
        every example declares expected content type, schema URL, validation
        command, expected pass/fail result, and expected diagnostic codes when
        failing.
  - [x] CEM-ML generic document/content model (`application/cem`).
  - [x] CEM-ML schema definition
        (`application/vnd.cem.schema+cem`).
  - [x] CEM-ML schema package manifest
        (`application/vnd.cem.schema-package+cem`, `package.cem`).
  - [x] CEM-ML native template
        (`application/vnd.cem.template+cem`).
  - [x] CEM-ML transform template
        (`application/vnd.cem.transform+cem`, `.cemt`).
  - [x] CEM-QL module/query resources
        (`application/vnd.cem.query+cem-ql`, `text/cem-ql`).
  - [x] JSON (`application/json`, `text/json`).
  - [x] JSON Schema (`application/schema+json`).
  - [x] CEM DOM projection
        (`application/vnd.cem.dom+cem-bin`,
        `application/vnd.cem.dom+json` debug view).
  - [x] CEM AST projection
        (`application/vnd.cem.ast+cem-bin`,
        `application/vnd.cem.ast+json` debug view).
  - [x] CEM events projection
        (`application/vnd.cem.events+cem-bin`,
        `application/vnd.cem.events+json` debug view).
  - [x] YAML/YML (`application/yaml`, compatibility aliases).
  - [x] CSV (`text/csv`).
  - [x] Markdown/MD markup (`text/markdown`).
  - [x] XML (`application/xml`, XML aliases).
  - [x] Relax NG schema
        (`application/relax-ng+xml`, `application/relax-ng-compact-syntax`).
  - [x] XHTML (`application/xhtml+xml`).
  - [x] SVG (`image/svg+xml`).
  - [x] MathML (`application/mathml+xml`, presentation/content aliases).
  - [x] XSLT/XSL legacy/custom-element compatibility
        (`application/xslt+xml`, `text/xsl`, custom-element aliases).
  - [x] HTML (`text/html`).
  - [x] CSS/scoped style content (`text/css`).

- [ ] Prepare AST-to-schema output export for all supported schema packages,
      with CEMT as the primary output producer. First review the output
      transformation design in
      [`../packages/cem_ml/schema-packages/README.md`](../packages/cem_ml/schema-packages/README.md)
      and the encoding proposal in
      [`../packages/cem_ml/docs/cemt-encoding-proposal.tmp.md`](../packages/cem_ml/docs/cemt-encoding-proposal.tmp.md).
  - [ ] Promote the CEMT encoding proposal into canonical docs: CEMT owns
        output production for schema-owned exports, including transformation,
        encoding, formatting, terminal/HTML color output, source-map span
        creation, final artifact identity, content-type-specific encoders,
        formatters, colorizers, writer primitives, and small transformation
        helpers. Clarify that encoding means syntax/context encoding, separate
        from byte character encoding and transport content encoding.
  - [ ] Define the CEMT `encode(subject, target, options?)` function and
        expression binding. `target` must carry `contentType`, `schema`,
        `category`, and optional context. `options` must cover canonical,
        preserve, pretty, and fragment modes; explicit `encoder`, `formatter`,
        `colorizer`, and `profile` selectors; charset; line ending; quote
        policy; indent; namespace policy; and source-map policy.
    - [x] Add the initial `encode` binding resolver that infers or accepts the
          subject type, resolves declared `encoding-function` metadata by
          target identity, category, explicit encoder/profile selector, and
          host capability, then wraps implementation output as a typed
          `EncodedArtifact`.
    - [x] Lower `{$ encode(...) }` CEMT expression calls from template bodies
          into typed module metadata, including subject expression, target
          identity/category/context, options, and invalid-call diagnostics.
    - [x] Add the runtime-facing encode evaluation bridge for lowered
          expressions: simple subject value lookup, registry resolution,
          implementation-output callback, typed artifact wrapping, and
          diagnostics for unresolved subjects or missing encoders.
    - [x] Preserve lowered encode expressions and output function descriptors on
          compiled CEMT artifacts so render-time adapters and engine hooks can
          evaluate and compose encoded artifacts.
    - [x] Add the first render-stage encode execution hook: host encoder
          implementation registry, render value bindings from primary/secondary
          inputs, compiled output-function resolution, evaluated artifact
          composition, and text output replacement.
  - [ ] Define encoded artifact identity and insertion rules. Results are not
        plain strings: they carry produced kind (`text`, `bytes`, `tokens`,
        `chunks`, or `diagnostics`), target content type, schema URL, encoding
        category/context, charset or binary framing identity, formatter profile,
        color profile/capability, fragment/document mode, canonicalization mode,
        source-map spans, and a double-encoding guard. Template insertion must
        reject incompatible target identity or context.
    - [x] Add the initial `EncodedArtifact` contract for CEMT output values,
          including target identity, category/context, fragment/document mode,
          canonicalization, source-map policy, renderer bridge, insertion
          compatibility validation, and double-encoding diagnostics.
    - [x] Add evaluated encode-artifact insertion validation helpers and
          context builders so renderer code can reject incompatible artifacts
          before writer composition.
  - [ ] Add CEMT declaration vocabulary for `encoding-function`,
        `format-function`, and `color-function` with registry-validatable
        metadata: `name`, `category`, `subject`, `produces`, `content-type`,
        `schema`, `canonical`, `streamable`, and typed params with required and
        default metadata. Helpers must be declared by schema package metadata and
        called from CEMT templates, not implemented as opaque host-side string
        filters.
    - [x] Add the first structural CEMT schema slice for
          `encoding-function`, `format-function`, `color-function`, params, core
          metadata, and schema-owned validation examples.
  - [ ] Add custom encoding, formatting, and color function support. Custom
        functions must use the same typed artifact semantics as built-ins and
        declare package-qualified names, visibility, implementation source
        (`cemt`, `native`, or `external`), profile, optional extension target,
        required capability, determinism, raw-output trust, fallback behavior,
        subject/output identity, params, and diagnostics. Registry lookup must
        resolve by owner package, name, content type, schema, category, subject
        type, and profile, and standard functions must not be shadowed unless
        explicitly aliased.
    - [x] Parse CEMT output function declarations into typed module
          descriptors and add registry resolution by function identity,
          content type, schema, category, subject, profile, and host capability.
  - [ ] Add shared encoder functions for context-specific escaping and binary
        framing across CEM, CEMT, XML, HTML, JSON, YAML, CSV, Markdown, CSS,
        CEM-QL, RELAX NG compact syntax, AI context projection, and CEM binary
        projection output categories.
    - [x] Register initial built-in HTML text and double-quoted attribute
          encoders for CEMT render-time execution, including category/context
          mismatch coverage.
    - [x] Register initial built-in JSON string, value, and document encoders
          for CEMT render-time execution, including JSON output identity and
          context/category mismatch coverage.
    - [x] Register initial built-in XML text and attribute-value encoders
          for CEMT render-time execution, including XML output identity and
          context/category mismatch coverage.
  - [ ] Add shared formatter functions for indentation, line endings, ordering,
        wrapping, YAML scalar style, namespace declaration placement, and
        canonical output profiles.
  - [ ] Add writer primitives and CEMT bindings for syntax tokens, styled token
        streams, byte streams, sealed binary chunks, source-map span emission,
        and source-map preservation/generated/none policies.
    - [x] Add the initial encoded text-artifact composition primitive: validate
          insertion compatibility, reject non-text/non-string artifacts,
          concatenate compatible text, and shift child output spans into the
          composed output byte range.
  - [ ] Add schema helper APIs for target syntax rules, void/empty element
        policy, raw-text/RCDATA modes, namespace repair, identifier validity,
        field/header policy, fragment/document handling, and charset/final byte
        writer boundaries.
  - [ ] Define CEMT color output support for terminal ANSI/SGR output and HTML
        color output. Style roles include diagnostics, source gutters and
        highlights, syntax tokens, diff hunks, and status states. Terminal
        profiles must support `none`, `ansi-16`, `ansi-256`, `truecolor`, and
        `auto`, no-color/forced-color policy, reset discipline, optional
        hyperlinks, and plain-text fallbacks. HTML profiles must support
        class-based output, explicit inline-style mode, CSS custom-property
        palettes, accessible contrast policy, non-color cues, escaped text and
        attributes, and fragment-safe output.
  - [ ] Define subject handling for scalar values, local/qualified names,
        namespace URIs, identifiers, structured values, CEM AST nodes, CEM DOM
        nodes, XML/HTML nodes, token streams, normalized parser/transform
        events, sealed binary chunks, attributes/slots, and fragments. Raw
        target syntax must be schema-gated and never the default.
  - [ ] Extend schema package metadata so each supported schema can declare
        source identity, output syntax, destination content type and schema,
        CEMT serializer template, template content type/schema, entrypoint,
        streamability, lossiness, readiness, encoding category, formatter
        profile, color output profile, native producer fallback symbol, fallback
        reason, and parity expectations.
  - [ ] Pair every native output producer with a CEMT implementation. Native
        producers are allowed for performance and clarity, but must be
        cross-checked against the schema-owned CEMT producer with shared
        fixtures and diagnostics. Parity metadata must support byte-exact,
        token-equivalent, parse-equivalent, and diagnostic-equivalent
        comparison modes, and drift must surface as a parity diagnostic before a
        native fast path is promoted.
  - [ ] Implement CEMT output safety rules: context-specific categories must not
        be conflated, encoded artifacts must not be silently re-encoded,
        compatible-artifact concatenation must validate target identity and
        category, character encoding must be selected at the final byte-writer
        boundary, color must use semantic roles and non-color fallbacks, terminal
        output must reset styles at artifact boundaries, HTML color output must
        escape text/attribute content before styling, and source maps must be
        produced as part of the encoding result.
  - [ ] Add diagnostics for unknown encoder, unsupported category, unsafe raw
        insertion, context mismatch, unsupported charset, charset mismatch,
        double encoding, unknown formatter, unsupported terminal color
        capability, inaccessible HTML palette, ambiguous custom function
        resolution, missing custom function capability, unavailable custom
        fallback, non-determinism in a canonical profile, incompatible custom
        subject type, incompatible produced kind, lossy output, incompatible
        artifact insertion, and CEMT/native parity mismatch.
  - [ ] Add AI-facing context projection support as a task-shaped view over the
        canonical AST/DOM/events/schema/token metadata, not a replacement for
        canonical projections. Cover `ai-context-pack`, `ai-entity-graph`,
        `ai-semantic-tokens`, `ai-context-fragment`, and
        `ai-embedding-record`.
  - [ ] Define AI context profile controls and safety: budgets for nodes,
        tokens, characters, depth, diagnostics, and source excerpts; stable IDs
        and source ranges; `summary`, `navigation`, `refactor`,
        `token-authoring`, `diagnostic`, and `embedding` profiles; lossiness
        metadata; lazy expansion refs to canonical projections; host/tool
        metadata; data/instruction boundary preservation; diagnostics for unsafe
        data/instruction mixing, unsupported profile, missing expansion target,
        and budget-driven omission; and task fixtures/evals for retrieval, edit
        precision, and token-budget value.
  - [ ] Add encoding category coverage and examples for every content-type
        family listed in the proposal: CEM-ML syntax, CEMT source, XML family,
        HTML, JSON family, YAML, CSV, Markdown, CSS, terminal color text, HTML
        color output, CEM-QL, RELAX NG compact syntax, AI context projections,
        and CEM binary projections.
  - [ ] Keep content-type-to-content-type conversion planning separate from
        AST-to-schema output production.

- Adopt the schema package content registry design as the active CEM-ML
  conversion goal:
  [`cem-ml-schema-content-registry-design.md`](cem-ml-schema-content-registry-design.md).
  Use the temporary transition plan in
  [`../packages/cem_ml/docs/schema-content-registry-transition.tmp.md`](../packages/cem_ml/docs/schema-content-registry-transition.tmp.md)
  to migrate the current runtime toward the design.
  - [x] Route direct CLI source validation selection through the schema
        registry content-type descriptors while preserving explicit schema
        mismatch rejection.
  - [x] Add `ConversionRegistry` direct-edge lookup between resolved source and
        target identities.
  - [x] Register CEMT converter descriptors as primary conversion edges with
        Rust fallback hooks for planned or adapter-unavailable edges.
  - [x] Execute selected converter descriptors through CEMT template adapters
        with Rust fallback execution when the CEMT edge is planned or no
        executable adapter is available.
  - [x] Load ready CEMT converter template assets and execute them end-to-end
        through the selected adapter before falling back to Rust hooks.
  - [x] Author packaged DOM-projection CEMT converter assets as schema-owned
        resources.
  - [x] Add CEMT dynamic element/attribute construction and packaged
        DOM-to-HTML converter smoke coverage.
  - [x] Add context-aware CEMT converter asset validation for packaged
        DOM-projection converter assets.
  - [x] Add XML target serialization for CEMT render-plan output.
  - [x] Add parity coverage; then promote the DOM-projection HTML/XML
        descriptors from planned to ready.
  - [x] Load DOM-projection CEMT converter descriptor metadata from
        `package.cem`, including `template-entrypoint`, instead of duplicating
        those CEMT edges in Rust.
  - [x] Move the remaining built-in Rust converter/debug edges into owning
        `package.cem` manifests and load all built-in conversion descriptors
        from package manifests.
  - [x] Add schema-package semantic validation for converter declarations:
        implementation hooks, exactly one `from`/`to` endpoint, known
        schema/content-type compatibility, boolean/readiness values, and
        positive planner cost.
  - [x] Load built-in schema registry descriptors from embedded
        `package.cem` manifests and schema `{uses}` declarations instead of a
        hand-maintained Rust descriptor table.
  - [x] Add local schema-package source consistency validation for
        `package.cem` schema URI, content type, and namespace declarations
        against referenced `schema/*.cem` sources.
  - [x] Add CLI integration coverage that validates every built-in
        `package.cem` manifest and checks schema package folders against the
        embedded built-in package catalog.

## Schema Package Implementation List

Implement schema packages for these content families:

- [x] CEM-ML generic document/content model.
- [x] CEM-ML schema definition.
- [x] CEM-ML schema package manifest (`application/vnd.cem.schema-package+cem`, `package.cem`).
- [x] create schema registry
- [x] CEM-ML template.
- [x] CEM-ML transform template (`application/vnd.cem.transform+cem`, `.cemt`).
- [x] use schema registry with transforms for parser/AST stream loading
- [x] CEM-QL module/query resources.
- [x] JSON.
- [x] JSON+JSON schema
- [x] CEM projection artifacts: DOM, AST, and events with primary CEM
      binary/stream encodings and optional JSON debug projections.
- [x] Define semantic DOM/AST/events projection schemas and migrate current
      registry-owned JSON projection exports
      (`https://cem.dev/ns/projection/dom-json/1`,
      `https://cem.dev/ns/projection/ast/1`,
      `https://cem.dev/ns/projection/events/1`) to optional debug/interchange
      views over primary CEM binary/stream artifacts.
- [x] Implement canonical CEM binary/chunk export adapters for
      `application/vnd.cem.dom+cem-bin`, `application/vnd.cem.ast+cem-bin`,
      and `application/vnd.cem.events+cem-bin`.
- [x] Add raw-byte CLI/file output for CEM binary artifacts.
- [x] Add native byte response APIs for CEM binary artifacts.
- [x] Remove JSON envelope dependency from internal binary projection routing;
      keep primary JSON metadata-only and full chunk envelopes
      compatibility/debug-only.
- [x] Implement parallel and multicast-capable projection stream routing over
      sealed CEM binary chunks.
- [x] YAML/YML.
- [x] CSV.
- [x] Markdown/MD markup.
- [x] XML.
- [x] Relax NG schema.
- [x] XHTML.
- [x] SVG.
- [x] MathML.
- [x] XSLT/XSL legacy/custom-element compatibility.
- [x] HTML.
- [x] CSS/scoped style content.
# [] custom schema creation instructions
# [] believes schema + registry
stop for sync up with author
## Current Verification Commands

- `yarn nx run @epa-wg/cem-theme:verify:phase13`
- `yarn nx run cem-elements:verify`
- `yarn nx run @epa-wg/cem-components:verify`
- `yarn nx run cem-elements:verify-edge-ssr`
- `yarn nx run @epa-wg/custom-element:verify`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
