# ADR: `cem-ml` Parser And Schema Stack

**Status:** Accepted for planning. No parser code has been added.

**Date:** 2026-05-07

**Related plan:** [`docs/cem-ml-cli-plan.md`](./cem-ml-cli-plan.md), Phase 1.

## Context

The Rust `cem-ml` CLI should support the feature set summarized in
[`cem-ml-cli-contract.md`](./cem-ml-cli-contract.md) while avoiding premature parser
implementation. Phase 1 exists to assess Java XML precedents, Rust library options,
schema mirror choices, diagnostics, source locations, security defaults, and WASM
constraints before parser-backed command work starts.

Surface update: canonical authoring input is now curly-brace CEM-ML as defined in
[`cem-ml-syntax.md`](./cem-ml-syntax.md). The XML/HTML parser inventory remains
relevant as secondary parity input support, schema mirror generation, and conformance
oracle work; it is not the canonical source-syntax decision.

The required diagnostic shape remains:

```txt
{ uri, line, column, byteOffset, code, severity, message }
```

The parser implementation must eventually feed a CEM-owned event and report model. No
third-party DOM or parser API is allowed to become the public `cem-ml` data surface.

## Java XML Stack Inventory

| Area                         | Pattern to preserve                                                                                                                              | Notes for `cem-ml`                                                                                                                                                |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| JAXP DOM                     | `DocumentBuilderFactory` creates namespace-aware, validating or non-validating DOM builders.                                                     | Useful as a boundary model only. DOM materialization should not drive the first Rust parser design because CEM needs a streaming-capable event model.             |
| SAX                          | SAX parser factories configure features on `XMLReader`; `Locator` and `SAXParseException` provide system/public ids and line/column diagnostics. | Preserve the event callback shape and diagnostic normalization, but store byte offsets in the CEM layer because SAX line/column values are approximate.           |
| StAX                         | `XMLInputFactory` exposes pull parsing and DTD/entity resolver properties. `Location` can expose line, column, and byte-or-character offset.     | This is the closest Java precedent for a Rust pull-event engine. Treat offset semantics carefully because StAX offset meaning depends on byte vs character input. |
| Xerces-style parser behavior | Feature flags control namespaces, validation, disallowing doctypes, and external DTD loading.                                                    | Keep parser features explicit and testable. Disable external entity and DTD resolution unless an explicit resolver is configured.                                 |
| Saxon-style XPath/XSLT       | Saxon s9api uses a `Processor` as the shared configuration root, then compiler/executable stages for XPath, XSLT, and schema processing.         | Preserve the staged boundary pattern for future transform work. Do not put XPath/XSLT inside the parser crate boundary yet.                                       |
| Jing/Trang RELAX NG          | Jing validates RELAX NG XML and compact syntax and is built around SAX2. Trang converts schema syntaxes.                                         | Use Jing/Trang as Java oracle tooling for schema mirror tests, not as the runtime implementation for `cem-ml`.                                                    |
| Validator.nu HTML parser     | Java HTML5 parser supports SAX, DOM, and XOM. True streaming SAX is available, but some HTML recovery is not streamable.                         | HTML input may need a parser-specific recovery model. Preserve deterministic diagnostics, and mark unsupported streaming recovery cases explicitly.               |
| XML Catalogs                 | OASIS XML Catalogs map external identifiers and URI references. JAXP catalog support spans SAX, DOM, StAX, validation, and transformation.       | Implement a CEM resolver policy before schema-backed parsing. Catalog-backed resolution should be allowlisted and offline by default.                             |

## Rust Ecosystem Inventory

| Candidate                      | Fit                                                                                                                                                     | Risk                                                                                                                                             |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| `quick-xml`                    | Strong XML event-reader candidate. It is StAX-like, streams events, exposes `buffer_position()` and `error_position()`, and has optional async support. | The low-level reader does not manage namespaces by itself. Use `NsReader` or a CEM namespace layer, and keep line/column derivation in CEM code. |
| `html5ever`                    | Strong HTML5 parser candidate. It supports incremental input through `parse_document` and delegates tree construction/errors through `TreeSink`.        | Default DOM examples are not production DOMs. Source byte offsets are not the primary API, so CEM must prototype location capture.               |
| `markup5ever_rcdom`            | Useful for tests and examples around `html5ever`.                                                                                                       | Its own docs say it is not production quality and not fuzzed for arbitrary input. Do not use as CEM runtime DOM.                                 |
| `roxmltree`                    | Good read-only XML tree for simple inspection and tests.                                                                                                | It materializes a full tree and is not a streaming parser boundary.                                                                              |
| `xot`                          | Modern XML tree candidate with mutable XML documents and span information.                                                                              | More promising for an internal tree than as the first parser boundary. Evaluate after the event model is stable.                                 |
| `libxml` crate and libxml2     | Offers XML/HTML parsing, schema validation, DOM, and XPath via libxml2.                                                                                 | Native C dependency and poor WASM fit. Keep as conformance oracle or escape hatch, not the default engine.                                       |
| `xsd_parser`                   | Good XSD-to-Rust code generation and schema introspection candidate.                                                                                    | Its own roadmap lists schema-based validation as planned. Use for XSD adapter experiments, not runtime validation.                               |
| `fastxml`                      | Claims pure-Rust DOM, XPath, and streaming XSD validation.                                                                                              | Early crate surface and domain-specific claims need independent verification before adopting. Track as research only.                            |
| `sxd-document` and `sxd-xpath` | Mature-ish pure-Rust XPath 1.0 path for simple XML trees.                                                                                               | XPath 1.0 only, limited relationship to future CEM transform needs.                                                                              |
| `xrust`                        | Ambitious pure-Rust XPath/XSLT direction with WASM-oriented external resource closures.                                                                 | Docs warn the library has not been extensively tested. Do not depend on it for planned CLI feature work yet.                                     |
| Pure Rust RELAX NG crates      | No clearly mature, primary RELAX NG validator emerged from the inventory.                                                                               | This is the largest schema runtime gap. Prefer generated CEM-specific validation or Java/libxml oracle checks until a Rust option is proven.     |

## Decision

1. **Parser engine recommendation:** build a CEM-owned parser engine boundary over Rust event readers, not over a
   third-party DOM.
    - CEM-native path: implement the canonical curly-brace tokenizer/parser first and lower it into the shared CEM
      event model.
    - XML parity path: prototype with `quick-xml`, using namespace-aware reading or a CEM namespace layer.
    - HTML parity path: prototype with `html5ever`, using a custom CEM sink rather than `markup5ever_rcdom`.
    - Public command output stays in `cem_ml` data shapes, not in third-party crate structs.
2. **Schema mirror recommendation:** use RELAX NG as the primary XML schema mirror for CEM semantic documents.
    - Keep CEM-native schema syntax as the canonical source of truth.
    - Emit RELAX NG XML and compact syntax mirrors for validation/tooling.
    - Treat XSD as a downstream adapter only when a consumer requires it.
3. **Runtime validation recommendation:** do not require a general RELAX NG runtime validator in `cem-ml` until a Rust
   implementation is proven.
    - Use generated CEM-specific validators for Rust behavior where practical.
    - Use Jing/Trang and, if needed, libxml2 as external conformance oracles in tests and release tooling.
4. **XPath/XSLT recommendation:** defer XPath/XSLT engine selection.
    - Follow Saxon's staged `Processor`/compiler/executable pattern for future architecture.
    - Do not add `sxd-xpath`, `xrust`, Saxon, or libxslt bindings for Phase 2-8 CLI feature work.

## Source Location Strategy

1. `byteOffset` is the primary stable source coordinate in `cem-ml`.
2. `line` and `column` are derived by `cem-ml` from a per-input UTF-8 line index, using `byteOffset`.
3. Parser-provided line/column values may be captured for debugging, but normalized CLI/report diagnostics use the CEM
   line-index result.
4. For XML, prototype byte offsets with `quick-xml` `buffer_position()` and `error_position()`.
5. For HTML, prototype byte offsets separately. If `html5ever` cannot expose enough source-span data through a custom
   sink/tokenizer path, parser-backed HTML offset support remains blocked.
6. Diagnostics emitted by schema, validation, transform, or plugin layers must attach back to the nearest known CEM
   source span and preserve `uri`.

## Security Defaults

1. Treat all input as untrusted.
2. Disable external DTDs, external entities, and network fetches by default.
3. Allow schema, DTD, stylesheet, and entity resolution only through an explicit resolver policy.
4. Prefer offline XML Catalog style mappings for known CEM schemas.
5. Set bounded parser limits for entity expansion, attribute count, nesting depth, source bytes, and diagnostics count
   before accepting untrusted files.
6. Parse HTML inertly. Scripts are never executed; unsafe script, event-handler, `javascript:` URL, and `srcdoc`
   patterns remain validation diagnostics.
7. If Java oracle tools are used, run them as explicit tooling steps with fixed classpaths, no plugin loading, no
   network access, and resource limits.
8. WASM builds must not depend on Java processes, libxml2, filesystem catalog lookup, or host network access.

## Requirement Comparison

| Requirement                       | Decision fit                                                                                                                 |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Deterministic diagnostics         | CEM-owned diagnostic model plus line-index derivation avoids parser-specific line/column drift.                              |
| Namespace and schema URI behavior | Namespace handling belongs in the CEM event layer; schema URI resolution belongs behind an explicit catalog/resolver policy. |
| Secure defaults                   | Java precedents support disabling DTD/entities and external access. Rust implementation must make the same policy explicit.  |
| Stable event model                | `quick-xml` and `html5ever` can feed a CEM event model. The CEM event model remains the only stable boundary.                |
| Canonical source syntax           | Curly CEM-ML is implemented as the primary source tokenizer; XML/HTML inputs are parity adapters into the same event model.  |
| Schema mirror generation          | RELAX NG mirrors fit CEM's mixed, extensible document model better than XSD as the primary mirror.                           |
| WASM feasibility                  | Pure Rust XML/HTML parser paths remain feasible. Java and libxml2 paths are oracle or native-only escape hatches.            |

## Unresolved Gaps

1. Prove exact source-span capture for `html5ever` HTML parsing.
2. Prove namespace event normalization for `quick-xml` against the five semantic fixtures and XML namespace edge cases.
3. Decide whether the internal materialized tree should be custom, `xot`, or another structure after the event model is
   stable.
4. Finish the CEM-native schema grammar and generator path into RELAX NG XML and compact syntax.
5. Decide how generated CEM-specific validation maps to RELAX NG conformance tests.
6. Decide how XML Catalog semantics should be represented in Rust without introducing network or filesystem surprises.
7. Reassess Rust XPath/XSLT candidates only when transform work enters scope.

## Follow-Up Plan

1. Phase 2 can define crate boundaries and modules without parser dependencies.
2. Phase 3 can define diagnostic, fail-level, report, and command-output data shapes.
3. Phase 4-8 can use a fake engine for CLI feature tests.
4. Parser implementation must start with the canonical CEM-ML tokenizer prototype, then measure source-span fidelity for
   the `quick-xml` and `html5ever` parity paths.
5. Schema implementation must start with CEM-native-to-RELAX-NG mirror generation and Java-oracle conformance tests.

## References

- [Oracle JAXP Security Guide](https://docs.oracle.com/en/java/javase/26/security/java-api-xml-processing-jaxp-security-guide.html)
- [Oracle `DocumentBuilderFactory`](https://docs.oracle.com/en/java/javase/26/docs/api/java.xml/javax/xml/parsers/DocumentBuilderFactory.html)
- [Oracle SAX `Locator`](https://docs.oracle.com/en/java/javase/11/docs/api/java.xml/org/xml/sax/Locator.html)
- [Oracle `SAXParseException`](https://docs.oracle.com/en/java/javase/26/docs/api/java.xml/org/xml/sax/SAXParseException.html)
- [Oracle StAX `XMLInputFactory`](https://docs.oracle.com/en/java/javase/17/docs/api/java.xml/javax/xml/stream/XMLInputFactory.html)
- [Oracle StAX `Location`](https://docs.oracle.com/en/java/javase/19/docs/api/java.xml/javax/xml/stream/Location.html)
- [Apache Xerces2 Java features](https://xerces.apache.org/xerces2-j/features.html)
- [Saxon s9api package docs](https://www.saxonica.com/html/documentation12/javadoc/net/sf/saxon/s9api/package-summary.html)
- [Saxon XPath with s9api](https://www.saxonica.com/html/documentation12/xpath-api/s9api-xpath.html)
- [RELAX NG home page](https://relaxng.org/)
- [Jing RELAX NG validator](https://relaxng.org/jclark/jing.html)
- [Validator.nu HTML Parser](https://about.validator.nu/htmlparser/)
- [OASIS XML Catalogs v1.1](https://www.oasis-open.org/standard/xmlcatalogs/)
- [Oracle `CatalogFeatures`](https://docs.oracle.com/en/java/javase/25/docs/api/java.xml/javax/xml/catalog/CatalogFeatures.html)
- [`quick-xml` docs](https://docs.rs/quick-xml/latest/quick_xml/)
- [`quick_xml::Reader` docs](https://docs.rs/quick-xml/latest/quick_xml/reader/struct.Reader.html)
- [`html5ever::parse_document` docs](https://docs.rs/html5ever/latest/html5ever/driver/fn.parse_document.html)
- [`html5ever::TreeSink` docs](https://docs.rs/html5ever/latest/html5ever/interface/trait.TreeSink.html)
- [`markup5ever_rcdom` docs](https://docs.rs/markup5ever_rcdom/latest/markup5ever_rcdom/)
- [`roxmltree` docs](https://docs.rs/roxmltree/)
- [`xot` docs](https://docs.rs/xot/latest/xot/index.html)
- [`libxml` crate docs](https://docs.rs/libxml/latest/libxml/)
- [`xsd_parser` docs](https://docs.rs/xsd-parser)
- [`sxd-xpath` docs](https://docs.rs/sxd-xpath/latest/x86_64-apple-darwin/sxd_xpath/)
- [`xrust` docs](https://docs.rs/xrust/latest/xrust/)
