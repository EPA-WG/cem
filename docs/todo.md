# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Tasks

- [ ] Complete the schema-package folder frame for
      `packages/cem_ml/schema-packages`: every `{package-id}/vN/` folder must be
      discoverable from `package.cem` with a `.cem` schema source, example
      references, CEMT formatter artifacts, and CEMT colorizer artifacts.
  - [ ] Extend the schema-package manifest and validators so package examples
        are declared with source path, content type, schema URL, expected
        pass/fail result, and expected diagnostics.
  - [ ] Require example loading to resolve the declared content type plus schema
        URL and validate the source bytes against that schema; filename
        extension inference is only a fallback hint.
  - [ ] Require baseline formatter profiles for each package:
        `compact` as default, `pretty`, and `tabular`; implement them as CEMT
        transforms that preserve source-map ranges.
  - [ ] Require baseline colorizer profiles for each package: `terminal`,
        `html`, and `md`; implement them as CEMT transforms over the formatted
        CEM tree with source-map range preservation.
  - [ ] Add package-folder validation that checks `package.cem`, `schema/`,
        `examples/`, `formatters/`, and `colorizers/` completeness for every
        built-in package.

- [x] Implement schema package loading and input-file validation for supported
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

- [x] Prepare AST-to-schema output export for all supported schema packages,
      with CEMT as the primary output producer. First review the output
      transformation design in
      [`../packages/cem_ml/schema-packages/README.md`](../packages/cem_ml/schema-packages/README.md)
      and the encoding proposal in
      [`../packages/cem_ml/docs/cemt-encoding-proposal.tmp.md`](../packages/cem_ml/docs/cemt-encoding-proposal.tmp.md).
  - [x] Promote the CEMT encoding proposal into canonical docs: CEMT owns
        output production for schema-owned exports, including transformation,
        encoding, formatting, terminal/HTML color output, source-map span
        creation, final artifact identity, content-type-specific encoders,
        formatters, colorizers, writer primitives, and small transformation
        helpers. Clarify that encoding means syntax/context encoding, separate
        from byte character encoding and transport content encoding.
    - [x] Promote the CEMT output producer contract into the schema package
          docs, make the transform package README canonical, and retain the
          temporary proposal as backlog and worked examples.
  - [x] Define the CEMT `encode(subject, target, options?)` function and
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
    - [x] Complete option-surface lowering for canonical, preserve, pretty, and
          fragment modes; explicit encoder, formatter, colorizer, and profile
          selectors; charset, line ending, quote, indent, namespace, and
          source-map policies; and preserve-mode encoded artifact identity.
  - [x] Define encoded artifact identity and insertion rules. Results are not
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
    - [x] Add binary framing identity to insertion contexts and reject
          mismatched binary framing for byte/chunk output artifacts.
  - [x] Add CEMT declaration vocabulary for `encoding-function`,
        `format-function`, and `color-function` with registry-validatable
        metadata: `name`, `category`, `subject`, `produces`, `content-type`,
        `schema`, `canonical`, `streamable`, and typed params with required and
        default metadata. Helpers must be declared by schema package metadata and
        called from CEMT templates, not implemented as opaque host-side string
        filters.
    - [x] Add the first structural CEMT schema slice for
          `encoding-function`, `format-function`, `color-function`, params, core
          metadata, and schema-owned validation examples.
    - [x] Require `canonical` and `streamable` output-function metadata and
          runtime-validate function params with explicit `type` plus
          `required` or `default` metadata.
  - [x] Add custom encoding, formatting, and color function support. Custom
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
    - [x] Add declaration diagnostics for unqualified custom output-function
          names and standard built-in function shadowing without explicit
          `extends` alias metadata.
    - [x] Resolve `extends` aliases as fallback output-function names while
          preserving exact-name precedence for standard functions.
    - [x] Enforce output-function visibility during registry and encode/color
          binding resolution so private owner-qualified custom functions only
          resolve for matching owner-scoped requests.
    - [x] Add `package` visibility for CEMT output-function declarations and
          resolve package-visible custom functions only for matching
          owner-scoped requests.
  - [x] Add shared encoder functions for context-specific escaping and binary
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
    - [x] Register initial built-in Markdown text and inline-code encoders for
          CEMT render-time execution, including Markdown output identity and
          schema/category mismatch coverage.
    - [x] Register initial built-in CSV field and record encoders for CEMT
          render-time execution, including CSV output identity and
          schema/category mismatch coverage.
    - [x] Register initial built-in CSS string and identifier encoders for
          CEMT render-time execution, including CSS output identity and
          content-type/category mismatch coverage.
    - [x] Register initial built-in YAML scalar and value encoders for CEMT
          render-time execution, including YAML content-type aliases and
          schema/category mismatch coverage.
    - [x] Register initial built-in CEM/CEMT source text encoders for
          `cem-document` and `cemt-module` output, including source identity
          validation and line-ending normalization.
    - [x] Register initial built-in CEM/CEMT token-level encoders for names,
          attribute values, content text, and CEMT expression string literals,
          including tokenizer-aware quote selection and rich-content fencing.
    - [x] Register initial built-in CEM-QL and RELAX NG compact source text
          encoders for `cem-ql-module` and `rnc-document` output, including
          authoring alias handling, source identity validation, and
          line-ending normalization.
    - [x] Register initial built-in CEM-QL and RELAX NG compact token-level
          encoders for selectors/patterns, strings/literals, and
          identifiers/names, including authoring alias handling and
          context/category mismatch coverage.
    - [x] Register initial built-in AI context projection JSON encoders for
          `ai-context-pack`, `ai-entity-graph`, `ai-semantic-tokens`,
          `ai-context-fragment`, and `ai-embedding-record` output, including
          projection-kind identity validation and JSON formatter integration.
    - [x] Register initial built-in CEM binary projection byte-stream encoders
          for DOM/AST/events `cem-bin-document` output with explicit binary
          framing identity validation.
  - [x] Add shared formatter functions for indentation, line endings, ordering,
        wrapping, YAML scalar style, namespace declaration placement, and
        canonical output profiles.
    - [x] Add initial JSON canonical/pretty formatter controls for CEMT
          render-time encoders, including indentation, LF/CRLF line endings,
          formatter profile identity, and profile mismatch diagnostics.
    - [x] Add initial XML formatter controls for CEMT render-time encoders,
          including LF/CRLF/preserve line endings, namespace-policy validation
          hooks, formatter profile identity, and profile mismatch diagnostics.
    - [x] Add initial YAML formatter controls for CEMT render-time encoders,
          including canonical/pretty flow formatting, LF/CRLF/preserve line
          endings, YAML scalar quote style, and profile mismatch diagnostics.
    - [x] Add canonical object ordering controls for JSON/YAML CEMT formatter
          profiles, including explicit lexical/preserve selectors and
          unsupported ordering diagnostics.
    - [x] Add initial Markdown text wrapping controls for CEMT formatter
          options, including wrap-column parsing, LF/CRLF output, and invalid
          selector diagnostics.
    - [x] Add namespace declaration placement controls for XML/CEMT formatter
          options and target syntax rules, including canonical defaults and
          non-namespace target diagnostics.
    - [x] Add canonical formatter profile identities for standard JSON, XML,
          YAML, and Markdown formatter aliases while preserving custom profile
          selectors.
  - [x] Add writer primitives and CEMT bindings for syntax tokens, styled token
        streams, byte streams, sealed binary chunks, source-map span emission,
        and source-map preservation/generated/none policies.
    - [x] Add the initial encoded text-artifact composition primitive: validate
          insertion compatibility, reject non-text/non-string artifacts,
          concatenate compatible text, and shift child output spans into the
          composed output byte range.
    - [x] Add initial writer value envelopes and validation for CEMT token,
          byte, chunk, and diagnostics artifacts, plus constructors that mark
          the produced kind before insertion validation.
    - [x] Add render text-boundary diagnostics for valid non-text writer
          artifacts that require a writer adapter before final insertion.
    - [x] Add the first default writer adapter: CEMT token streams with text
          payloads can compose into text output while preserving artifact
          identity, source maps, and output spans.
    - [x] Add token-level `outputSpan` support for CEMT token streams, with
          generated output ranges during token-to-text composition and
          source-map policy suppression for `none`.
    - [x] Add styled writer token metadata for CEMT token streams, including
          style envelope validation and default token-to-text adapter
          preservation of plain-text behavior.
    - [x] Add a default text-chunk writer adapter for CEMT chunk streams,
          preserving source maps/output spans for text chunks and rejecting
          byte-bearing chunks with adapter-failed diagnostics.
    - [x] Apply source-map `none` policy suppression to direct text artifact
          composition so text, token, and chunk writer paths enforce the same
          source-map/output-span boundary behavior.
  - [x] Add schema helper APIs for target syntax rules, void/empty element
        policy, raw-text/RCDATA modes, namespace repair, identifier validity,
        field/header policy, fragment/document handling, and charset/final byte
        writer boundaries.
    - [x] Add initial `TransformTemplateTargetSyntaxRules` helpers for CEMT
          encode targets, covering HTML void/raw-text/RCDATA elements, XML
          empty elements and namespace repair policies, JSON field names, CSV
          headers, fragment/document mode gates, charset defaults, and final
          newline/binary writer boundaries.
    - [x] Wire target syntax rules into encode binding resolution so invalid
          fragment/document mode, namespace policy, charset, and text/binary
          writer-boundary choices surface as CEMT diagnostics before artifact
          production.
    - [x] Add explicit JSON field-name and CSV header-name target syntax
          helpers so field/header policy checks are available without relying
          on the generic identifier helper.
  - [x] Define CEMT color output support for terminal ANSI/SGR output and HTML
        color output. Style roles include diagnostics, source gutters and
        highlights, syntax tokens, diff hunks, and status states. Terminal
        profiles must support `none`, `ansi-16`, `ansi-256`, `truecolor`, and
        `auto`, no-color/forced-color policy, reset discipline, optional
        hyperlinks, and plain-text fallbacks. HTML profiles must support
        class-based output, explicit inline-style mode, CSS custom-property
        palettes, accessible contrast policy, non-color cues, escaped text and
        attributes, and fragment-safe output.
    - [x] Add initial CEMT color output profile descriptors, semantic color
          roles, terminal capability selectors (`none`, `ansi-16`, `ansi-256`,
          `truecolor`, `auto`), HTML style modes, non-color fallback policy,
          reset/escape/fragment-safety validation, and focused tests.
    - [x] Wire color profile selectors into `color-function`/`colorizer`
          binding resolution, including terminal/HTML target inference,
          canonical profile identity, terminal color capability identity, and
          early diagnostics for unsupported profile selectors.
    - [x] Add explicit terminal hyperlink profile selectors with validation so
          hyperlinks resolve only for terminal capabilities that allow them
          while preserving the underlying color capability identity.
  - [x] Define subject handling for scalar values, local/qualified names,
        namespace URIs, identifiers, structured values, CEM AST nodes, CEM DOM
        nodes, XML/HTML nodes, token streams, normalized parser/transform
        events, sealed binary chunks, attributes/slots, and fragments. Raw
        target syntax must be schema-gated and never the default.
    - [x] Add initial typed subject candidate inference for CEMT encode inputs:
          explicit `subjectType` envelopes, local/qualified names, namespace
          URIs, identifiers, token streams, parser/transform event streams,
          sealed/binary chunk streams, attributes, slots, fragments, semantic
          AST/DOM/XML/HTML node hints, and structured `map` fallbacks.
    - [x] Add explicit raw-subject inference for schema-owned raw syntax
          envelopes, including `raw-html`/`raw-xml` candidates and subject-type
          incompatibility when callers try to bind raw functions from plain
          values.
    - [x] Add collection-level `attributes`/`attribute-list` and
          `slots`/`slot-list` subject candidates so collection encoders do not
          have to overload single attribute/slot subject types.
  - [x] Extend schema package metadata so each supported schema can declare
        source identity, output syntax, destination content type and schema,
        CEMT serializer template, template content type/schema, entrypoint,
        streamability, lossiness, readiness, encoding category, formatter
        profile, color output profile, native producer fallback symbol, fallback
        reason, and parity expectations.
    - [x] Add initial converter output-contract metadata to schema package
          manifests and runtime descriptors: `output-syntax`,
          `encoding-category`, `formatter-profile`, `color-profile`, and
          `parity`, with validation and DOM projection CEMT metadata examples.
    - [x] Require CEMT converters that declare native fallback symbols to also
          declare `fallback-reason`, and align schema-package `output-syntax`
          validation with runtime YAML support.
    - [x] Validate converter `lossiness` selectors against the schema-owned
          vocabulary (`lossless`, `serialization`, `syntax-normalized`,
          `debug-view`, and `recovery`) so package metadata cannot drift.
  - [x] Pair every native output producer with a CEMT implementation. Native
        producers are allowed for performance and clarity, but must be
        cross-checked against the schema-owned CEMT producer with shared
        fixtures and diagnostics. Parity metadata must support byte-exact,
        token-equivalent, parse-equivalent, and diagnostic-equivalent
        comparison modes, and drift must surface as a parity diagnostic before a
        native fast path is promoted.
    - [x] Add initial CEMT/native parity contract planning from converter
          metadata, including diagnostics for missing parity modes, missing
          paired native converters, and conservative output drift reporting
          under the declared parity mode.
    - [x] Make conversion parity comparison mode-aware for HTML/XML
          `parse-equivalent` outputs by comparing structural projections
          instead of byte-identical serialization.
    - [x] Make conversion parity comparison mode-aware for HTML/XML
          `token-equivalent` outputs by comparing tokenizer projections
          without source ranges or trivia.
    - [x] Make conversion parity comparison mode-aware for
          `diagnostic-equivalent` outputs by comparing normalized diagnostic
          projections without message text or array-order drift.
    - [x] Add shared conversion parity fixture execution so paired CEMT/native
          producers run against the same fixture inputs and expected diagnostic
          projections before drift is reported.
    - [x] Add package manifest metadata for converter parity fixtures, including
          package-relative fixture paths, input identity, and expected diagnostic
          code projections for DOM serializer parity.
    - [x] Resolve declared converter parity fixture paths into runner-ready
          byte inputs with fixture identity, input content identity, and
          expected diagnostic code projections.
    - [x] Add Rust DOM-projection parity fixture execution that decodes
          declared binary fixtures and renders HTML/XML oracle outputs for
          CEMT/native comparison.
    - [x] Add CEMT template-backed parity fixture execution that invokes
          executable template adapters for CEMT descriptors while keeping Rust
          DOM execution as the native side.
    - [x] Add registry-level declared converter parity evaluation that discovers
          contracts, loads package fixtures, and runs every fixture through the
          supplied CEMT/native executor.
    - [x] Add `cem-ml fixture parity` as a CLI/CI-facing command for declared
          converter parity fixture verification using the current Rust DOM
          oracle executor.
    - [x] Add `cem_ml_cli:validate-converter-parity` and include converter
          parity verification in the `cem_ml_cli:e2e` Nx target.
    - [x] Switch `cem-ml fixture parity` from Rust-only oracle comparison to a
          parity-local executable CEMT adapter plus native Rust oracle
          comparison for packaged DOM-projection converters.
    - [x] Move the DOM-projection parity CEMT adapter into `cem_ml` so CLI and
          library parity checks share the same executable adapter.
    - [x] Report ready native schema output producers that do not have a
          matching CEMT converter with the same source/target identity and
          fallback symbol.
  - [x] Implement CEMT output safety rules: context-specific categories must not
        be conflated, encoded artifacts must not be silently re-encoded,
        compatible-artifact concatenation must validate target identity and
        category, character encoding must be selected at the final byte-writer
        boundary, color must use semantic roles and non-color fallbacks, terminal
        output must reset styles at artifact boundaries, HTML color output must
        escape text/attribute content before styling, and source maps must be
        produced as part of the encoding result.
    - [x] Add conversion-level CEMT output safety contracts that lower package
          converter metadata into encoded-artifact insertion contexts, syntax
          rules, final-writer charset defaults, generated source-map policy, and
          safe color profile metadata.
    - [x] Enforce encoded-artifact source-map policy compatibility at insertion
          boundaries so generated-source-map output contexts cannot silently
          accept artifacts produced under a different source-map policy.
  - [x] Add diagnostics for unknown encoder, unsupported category, unsafe raw
        insertion, context mismatch, unsupported charset, charset mismatch,
        double encoding, unknown formatter, unsupported terminal color
        capability, inaccessible HTML palette, ambiguous custom function
        resolution, missing custom function capability, unavailable custom
        fallback, non-determinism in a canonical profile, incompatible custom
        subject type, incompatible produced kind, lossy output, incompatible
        artifact insertion, and CEMT/native parity mismatch.
    - [x] Add initial converter output-safety diagnostics for missing output
          syntax/category, unsupported encoding category, category/target
          context mismatch, and unsafe color profile metadata.
    - [x] Add output-function diagnostics for kind-specific unknown
          encoder/formatter/colorizer lookup, unavailable custom fallback, and
          non-deterministic functions selected for canonical output.
    - [x] Add charset diagnostics for unsupported charset selectors and
          mismatches between the target content type charset parameter and the
          requested writer charset.
    - [x] Add encoded-artifact insertion compatibility diagnostics that preserve
          specific identity/context/produced-kind/value-shape errors while also
          surfacing a stable artifact-insertion category code for renderers.
    - [x] Add unsafe raw insertion diagnostics so `raw` encode requests require
          an explicitly trusted output function declaration before binding.
    - [x] Add incompatible custom subject-type diagnostics for encode and color
          bindings when a matching function exists but accepts different
          semantic subject types.
    - [x] Add incompatible produced-kind diagnostics for encode and color
          bindings when a selected output function crosses text/binary writer
          boundaries.
    - [x] Add lossy-output diagnostics so functions declared `lossy` require an
          explicit `allowLossy` encode/color binding opt-in.
    - [x] Add specific color diagnostics for unsupported terminal capabilities
          and inaccessible HTML palettes while preserving generic color-profile
          validation for target-agnostic selectors.
    - [x] Add CEMT/native encode parity mismatch diagnostics for drift in
          diagnostic codes and encoded artifact identity/value/source-map/span
          fields.
    - [x] Assert ambiguous output-function resolution surfaces the stable
          renderer-facing diagnostic code and conflicting function names.
  - [x] Add AI-facing context projection support as a task-shaped view over the
        canonical AST/DOM/events/schema/token metadata, not a replacement for
        canonical projections. Cover `ai-context-pack`, `ai-entity-graph`,
        `ai-semantic-tokens`, `ai-context-fragment`, and
        `ai-embedding-record`.
    - [x] Add initial AST-backed AI context projection records with stable
          canonical refs, source maps, data/instruction boundaries, and view
          filtering for all five AI projection kinds.
    - [x] Expose AST-backed AI context projection values through standard CEMT
          JSON encoders for all five AI projection output categories.
    - [x] Assert all five AI projection kinds remain canonical AST views with
          `cem-ast` expansion refs, record-level source maps, and
          projection-specific facets.
  - [x] Define AI context profile controls and safety: budgets for nodes,
        tokens, characters, depth, diagnostics, and source excerpts; stable IDs
        and source ranges; `summary`, `navigation`, `refactor`,
        `token-authoring`, `diagnostic`, and `embedding` profiles; lossiness
        metadata; lazy expansion refs to canonical projections; host/tool
        metadata; data/instruction boundary preservation; diagnostics for unsafe
        data/instruction mixing, unsupported profile, missing expansion target,
        and budget-driven omission; and task fixtures/evals for retrieval, edit
        precision, and token-budget value.
    - [x] Add profile/budget request controls, usage and lossiness metadata,
          source ranges/excerpts, lazy canonical expansion refs, host/tool
          metadata, and projection diagnostics for unsupported profiles, missing
          expansion roots, budget omissions, and unsafe data/instruction mixing.
    - [x] Add task eval fixtures and metrics for retrieval recall/precision,
          edit-target source-map precision, and useful records per token budget.
  - [x] Add encoding category coverage and examples for every content-type
        family listed in the proposal: CEM-ML syntax, CEMT source, XML family,
        HTML, JSON family, YAML, CSV, Markdown, CSS, terminal color text, HTML
        color output, CEM-QL, RELAX NG compact syntax, AI context projections,
        and CEM binary projections.
    - [x] Add proposal-family conversion output-safety examples that validate
          category syntax, target content type/schema, produced kind, and
          insertion context for every listed family.
  - [x] Keep content-type-to-content-type conversion planning separate from
        AST-to-schema output production.
    - [x] Add conversion planning domains with separate content-conversion and
          schema-output selection/execution APIs, and scope CEMT output-safety
          contracts to schema output producers.

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
- [x] Add custom schema package creation instructions.

## Schema Package Content-Type Parts Checklist

Current inventory: every package below already has `package.cem`, a `.cem`
schema source under `schema/`, and checked-in examples under `examples/`.
Remaining work is to make example schema references explicit and add the
standard CEMT formatter/colorizer profile sets.

- [ ] `cem-ml/v1` (`application/cem`): [x] `package.cem`, [x] `.cem`
      schema, [x] examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `schema/v1` (`application/vnd.cem.schema+cem`): [x] `package.cem`,
      [x] `.cem` schema, [x] examples, [ ] example content-type/schema
      references, [ ] `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `schema-package/v1` (`application/vnd.cem.schema-package+cem`): [x]
      `package.cem`, [x] `.cem` schema, [x] examples, [ ] example
      content-type/schema references, [ ] `compact`/`pretty`/`tabular` CEMT
      formatters, [ ] `terminal`/`html`/`md` CEMT colorizers.
- [ ] `cem-native-template/v1` (`application/vnd.cem.template+cem`): [x]
      `package.cem`, [x] `.cem` schema, [x] examples, [ ] example
      content-type/schema references, [ ] `compact`/`pretty`/`tabular` CEMT
      formatters, [ ] `terminal`/`html`/`md` CEMT colorizers.
- [ ] `cem-transform/v1` (`application/vnd.cem.transform+cem`): [x]
      `package.cem`, [x] `.cem` schema, [x] examples, [ ] example
      content-type/schema references, [ ] `compact`/`pretty`/`tabular` CEMT
      formatters, [ ] `terminal`/`html`/`md` CEMT colorizers.
- [ ] `cem-ql/v1` (`application/vnd.cem.query+cem-ql`, `text/cem-ql`):
      [x] `package.cem`, [x] `.cem` schema, [x] examples, [ ] example
      content-type/schema references, [ ] `compact`/`pretty`/`tabular` CEMT
      formatters, [ ] `terminal`/`html`/`md` CEMT colorizers.
- [ ] `json/v1` (`application/json`, `text/json`): [x] `package.cem`, [x]
      `.cem` schema, [x] examples, [ ] example content-type/schema references,
      [ ] `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `json-schema/v1` (`application/schema+json`): [x] `package.cem`, [x]
      `.cem` schema, [x] examples, [ ] example content-type/schema references,
      [ ] `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `cem-dom-projection/v1` (`application/vnd.cem.dom+cem-bin`,
      `application/vnd.cem.dom+json`): [x] `package.cem`, [x] `.cem` schema,
      [x] examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `cem-ast-projection/v1` (`application/vnd.cem.ast+cem-bin`,
      `application/vnd.cem.ast+json`): [x] `package.cem`, [x] `.cem` schema,
      [x] examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `cem-events-projection/v1` (`application/vnd.cem.events+cem-bin`,
      `application/vnd.cem.events+json`): [x] `package.cem`, [x] `.cem`
      schema, [x] examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `yaml/v1` (`application/yaml`, YAML aliases): [x] `package.cem`, [x]
      `.cem` schema, [x] examples, [ ] example content-type/schema references,
      [ ] `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `csv/v1` (`text/csv`): [x] `package.cem`, [x] `.cem` schema, [x]
      examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `markdown/v1` (`text/markdown`): [x] `package.cem`, [x] `.cem`
      schema, [x] examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `xml/v1` (`application/xml`, XML aliases): [x] `package.cem`, [x]
      `.cem` schema, [x] examples, [ ] example content-type/schema references,
      [ ] `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `relax-ng/v1` (`application/relax-ng+xml`,
      `application/relax-ng-compact-syntax`): [x] `package.cem`, [x] `.cem`
      schema, [x] examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `xhtml/v1` (`application/xhtml+xml`): [x] `package.cem`, [x] `.cem`
      schema, [x] examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `svg/v1` (`image/svg+xml`): [x] `package.cem`, [x] `.cem` schema,
      [x] examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `mathml/v1` (`application/mathml+xml`, MathML aliases): [x]
      `package.cem`, [x] `.cem` schema, [x] examples, [ ] example
      content-type/schema references, [ ] `compact`/`pretty`/`tabular` CEMT
      formatters, [ ] `terminal`/`html`/`md` CEMT colorizers.
- [ ] `xslt/v1` (`application/xslt+xml`, XSLT aliases): [x] `package.cem`,
      [x] `.cem` schema, [x] examples, [ ] example content-type/schema
      references, [ ] `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `html/v1` (`text/html`): [x] `package.cem`, [x] `.cem` schema, [x]
      examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.
- [ ] `css/v1` (`text/css`): [x] `package.cem`, [x] `.cem` schema, [x]
      examples, [ ] example content-type/schema references, [ ]
      `compact`/`pretty`/`tabular` CEMT formatters, [ ]
      `terminal`/`html`/`md` CEMT colorizers.

- [ ] expand example coverage from representative constraint-kind coverage to finer diagnostic coverage, starting with schema-package source read/invalid cases and
  artifact source/parse/function-missing cases.

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
