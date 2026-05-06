Invite team members

cem-dom-ac.md
File
think what functionality cem-dom cli can provide. create a prompt for documenting cli.
ideas for cli:

* parse and validate against schema, fail level: parse, validate, strict
* convert syntax from one schema to another, internal AST as one of out-of-the-box formats/schema
* generate sample from schema
* replace one schema in sub-document with another
*

Implementation status:

* Tier A implemented: `parse`, `validate`, `check`, `fixture validate`, `help`, `version`, fail levels, JSON/Markdown
  reports, deterministic aggregate reports, and reserved Tier B/C command failures.
* Parser-backed Tier B slice implemented: `parse --format ast|events` and `inspect --show
  summary|ast|diagnostics|source-offsets|tree`.
* Parser-backed conversion slice implemented: `convert --from-format html|xml --to-format dom-json|ast|events`, with
  `--out`, `--format` output aliases, and optional `--preserve-source-offsets`.
* Parser/validator-backed trace slice implemented: `trace --format json|text`, with deterministic input, parse, and
  validate events plus `--out` support.
* Parser/validator-backed Tier C slice implemented: `bench` with iterations, JSON reports, cold-cache reads, recorded
  profile intent, and per-input budget checks.
* Deferred: real schema loading/version compatibility, transforms, advanced conversion beyond parser output formats,
  schema/advanced-inspect/advanced-trace/plugin behavior, transform benchmarking, real CPU/memory profiling, source
  maps, scope metadata, and roundtrip fixture rendering.

I’ll turn the acceptance criteria into a CLI surface first, then package that as a reusable documentation-writing
prompt.

Thought for 1m 51s
Here’s a stronger CLI surface for cem-dom, based on the AC file’s framing of @epa-wg/cem-dom as a parser + schema +
interpreter stack for CEM artifacts, with parsing, validation, schema emission, transformations, reports, scoped
content, plugins, and observability as first-class concerns.

# Proposed cem-dom CLI functionality

## Core commands

    cem-dom parse <input>

Parse HTML/XML/CEM syntax into a normalized CEM DOM representation.

Useful options:

```bash
--content-type text/html|application/xml|...
--schema <schema-uri-or-file>
--format events|ast|dom-json|html|xml
--fail-level parse|validate|strict
--base-uri <uri>
--out <file>
--report-json <file>
--report-md <file>
```

Behavior:

`parse` fail level: fail only on fatal parse errors.

`validate` fail level: also fail on hard schema violations.

`strict` fail level: warnings become failures, including unknown elements/attributes that would otherwise be tolerated
in compatible minor schema versions.

Emits structured diagnostics with URI, line, column, byte offset, code, severity, and message.

---

```bash
cem-dom validate <input...>
```

Validate documents against CEM schemas.

Useful options:

```bash
--schema <schema-uri-or-file>
--fail-level parse|validate|strict
--report-json <file-or-dir>
--report-md <file-or-dir>
--format text|json|markdown
--zero-hard-violations
--include accessibility,security,references
```

Checks should include:

* unknown elements and attributes
* invalid element or state combinations
* broken id, for, and aria-* references
* missing accessible names
* unsafe inline content
* schema version compatibility
* major-version mismatch failures

---

```bash
cem-dom transform <input>
```

Apply a CEM transform, XSLT-equivalent transform, or built-in transformation pipeline.

Example:

```bash
cem-dom transform examples/semantic/login.html \
  --schema schemas/cem.cem \
  --transform transforms/light-dom.cemx \
  --out dist/login.html \
  --report-json dist/login.report.json
```

Useful options:

```bash
--transform <uri-or-file>
--to light-dom|custom-element|html|xml|dom-json
--content-type <type>
--source-map
--fail-level parse|validate|strict
```

---

```bash
cem-dom convert <input>
```

Convert syntax or document representation from one schema/content model to another.

Example:

```bash
cem-dom convert page.html \
  --from-format html \
  --to-format ast \
  --out page.ast.json
cem-dom convert button.cem \
  --from-schema schemas/cem-v1.cem \
  --to-schema schemas/cem-v2.cem \
  --out button.v2.cem
```

Supported conversion ideas:

* CEM-native syntax → XML
* XML → CEM-native syntax
* HTML semantic fixture → normalized CEM AST
* CEM AST → light-DOM custom-element markup
* RELAX-NG / XSD mirror → CEM-native schema, if supported later
* internal AST as an official interchange format

Implemented parser-backed slice:

```bash
--from-format html|xml
--to-format dom-json|ast|events
--format dom-json|json|ast|events # output alias when --to-format is omitted
--preserve-source-offsets
--out <file>
```

Useful options:

```bash
--from-schema <schema>
--to-schema <schema>
--from-format cem|html|xml|ast|events
--to-format cem|html|xml|ast|events|dom-json
--preserve-comments
--preserve-source-offsets
--source-map
```

---

```bash
cem-dom schema emit <schema>
```

Emit derived schema artifacts.

Example:

```bash
cem-dom schema emit schemas/cem.cem \
  --to relax-ng,typescript,json-schema \
  --out-dir dist/schema
```

Possible targets:

```bash
--to cem
--to relax-ng
--to xsd
--to typescript
--to rust
--to json-schema
--to ast-schema
```

This maps directly to the AC requirement that CEM-native syntax is the source of truth while release artifacts include
XML-schema mirrors and TypeScript type headers.

---
```bash
cem-dom schema sample <schema>
```
Generate valid sample documents from a schema.

Example:
```bash
cem-dom schema sample schemas/component.cem \
--root cem-component \
--count 5 \
--out examples/generated
```
Useful options:
```bash
--root <element>
--count <n>
--mode minimal|typical|maximal|edge|invalid
--seed <number>
--content-type text/html|application/xml
--include-comments
```
Modes:

* `minimal`: smallest valid document.
* `typical`: representative real-world sample.
* `maximal`: includes optional branches.
* `edge`: boundary cases.
* `invalid`: intentionally invalid samples for validator tests.

---
```bash
cem-dom schema replace <input>
```
Replace one schema-governed sub-document with another schema or upgraded schema version.

Example:
```bash
cem-dom schema replace page.html \
--select 'scope[schemaUri="https://cem.dev/schema/v1"]' \
--with-schema schemas/cem-v2.cem \
--rewrite-namespaces \
--out page.v2.html
```
Selection mechanisms:
```bash
--select <css-or-cem-selector>
--xpath <xpath>
--scope-uri <schema-uri>
--content-type <content-type>
--namespace-uri <namespace-uri>
```
Replacement behavior:

* update namespace/schema declarations
* validate the selected sub-document before replacement
* validate the final whole document after replacement
* preserve source offsets where possible
* emit a source map for rewritten regions
* fail if the replacement crosses an owned scope boundary without explicit permission

---

Developer and CI commands
```bash
cem-dom check <input...>
```
Convenience command for CI: parse + validate + report.
```bash
cem-dom check examples/semantic/*.html \
--schema schemas/cem.cem \
--zero-hard-violations \
--report-json reports \
--report-md reports
```

---

```bash
cem-dom fixture validate
```
Validate known semantic fixtures.

```bash
cem-dom fixture validate examples/semantic/*.html
```
This should mirror the acceptance-plan command shape like `cem-dom:validate-fixtures`.

---

```bash
cem-dom fixture roundtrip
```
Run parse → validate → transform → render/snapshot.

```bash
cem-dom fixture roundtrip examples/semantic/*.html \
--transform transforms/light-dom.cemx \
--snapshot-dir snapshots
```
---

```bash
cem-dom bench <input...>
```
Benchmark parse, validate, and transform.

```bash
cem-dom bench examples/semantic/*.html \
--report-json bench.report.json
``` 
Useful options:
```bash
--iterations <n>
--cold-cache
--profile memory|cpu
--budget-ms 150
```
This maps to the AC requirement for performance budgets around parsing, validation, and transformation.

---
```bash
cem-dom inspect <input>
```
Inspect document structure, scopes, schema bindings, plugins, diagnostics, or AST.

Examples:
```bash
cem-dom inspect page.html --show scopes
cem-dom inspect page.html --show ast
cem-dom inspect page.html --show schema-bindings
cem-dom inspect page.html --show source-offsets
```
Useful options:
```bash
--show scopes|ast|events|schema-bindings|plugins|diagnostics|source-map
--format text|json|tree
```
---
```bash
cem-dom trace <input>
```
Emit deterministic parser/interpreter/transform trace data.

```bash
cem-dom trace page.html \
--transform transforms/light-dom.cemx \
--out trace.json
```
Useful for debugging:

* parse event order (implemented)
* validation events (implemented)
* transformation events
* scope boundaries
* plugin execution order
* thread-pool scheduling, later Tier B/C
* source-map stitching

Implemented parser/validator slice:

```bash
cem-dom trace page.html --format json
cem-dom trace page.html --format text --out trace.txt
```

---
Plugin-facing commands
```bash
cem-dom plugin list
cem-dom plugin inspect <plugin>
cem-dom plugin run <plugin> <input>
```
Examples:

```bash
cem-dom plugin list --config cem-dom.config.mjs

cem-dom plugin run ./plugins/security-checker.mjs page.html \
--content-type text/html \
--report-json security.report.json
```
Plugin docs should distinguish:

* observe plugins: emit diagnostics but do not mutate
* mutate plugins: transform content and must emit source maps
* inherited plugins from parent scopes
* scope-local plugins
* plugin failure behavior


This aligns with the plugin descriptor and transformation-chain model in the AC.

---

Suggested top-level CLI shape
    cem-dom
        parse Parse input into events, AST, DOM JSON, HTML, or XML
        validate Validate documents against CEM schemas
        check Parse + validate convenience command for CI
        transform Apply CEM/XSLT-equivalent transforms
        convert Convert syntax, schema version, or output representation
        inspect Inspect AST, scopes, schema bindings, diagnostics, plugins
        trace Emit parser/interpreter/transform trace
        bench Benchmark parser/validator/transform pipeline

        schema emit Emit RELAX-NG/XSD/TypeScript/Rust/JSON schema artifacts
        schema sample Generate valid or intentionally invalid sample documents
        schema replace Replace or upgrade schema-bound sub-documents

        fixture validate Validate semantic fixtures
        fixture roundtrip Parse → validate → transform → render/snapshot fixtures

        plugin list List configured plugins
        plugin inspect Show plugin descriptor and compatibility
        plugin run Run a plugin against input for debugging

## Prompt for documenting the cem-dom CLI
```md
You are documenting the CLI for `@epa-wg/cem-dom`.

Context:
`@epa-wg/cem-dom` is the document layer for CEM. It is a parser + schema + interpreter + transform stack for CEM
artifacts, including semantic fixtures, component templates, design docs, transforms, and schema-governed sub-documents.

The CLI should expose functionality for:

- parsing HTML, XML, and CEM-native syntax into a shared DOM / AST model
- validating documents against CEM schemas
- emitting machine-readable JSON reports and human-readable Markdown reports
- supporting fail levels: `parse`, `validate`, and `strict`
- converting syntax from one schema or representation to another
- using the internal AST as an official output/interchange format
- generating sample documents from schemas
- replacing or upgrading one schema-governed sub-document inside a larger document
- applying XSLT-equivalent or CEM-native transforms
- transforming semantic fixtures into light-DOM custom-element markup
- emitting schema mirrors such as RELAX-NG or XSD
- emitting TypeScript type headers, and optionally Rust type headers
- inspecting scopes, schema bindings, diagnostics, source offsets, plugins, and AST
- supporting scoped content-type boundaries such as HTML containing CSS, SVG, or other embedded content
- supporting plugins that can observe or mutate transformation output
- emitting source maps for mutating transforms
- providing CI-friendly fixture validation, roundtrip checks, reports, and benchmarks
- exposing observability through structured parse, validate, and transform events

Write a complete `docs/cli.md` page.

Audience:

- contributors implementing `@epa-wg/cem-dom`
- CEM package maintainers
- CI authors
- advanced users who need to validate, transform, or migrate CEM documents

Documentation goals:

1. Make the CLI feel coherent and product-ready.
2. Explain the mental model: input → parse → validate → interpret/transform → output/report.
3. Define command groups and naming conventions.
4. Specify command syntax, options, examples, outputs, reports, and exit codes.
5. Clearly explain `--fail-level parse|validate|strict`.
6. Show how the CLI fits local development, CI, schema authoring, fixture validation, and migration workflows.
7. Mark Tier A commands as required for MVP and Tier B/C commands as future or experimental where appropriate.
8. Avoid over-promising features that are not in Tier A unless they are explicitly labeled as future or experimental.

Required document structure:

# `cem-dom` CLI

## Overview

Explain what the CLI is for and how it relates to the parser, validator, schema, interpreter, transform, plugin, and
reporting layers.

## Installation

Show npm / workspace usage examples:

- `npx @epa-wg/cem-dom`
- `yarn cem-dom`
- `yarn nx run cem-dom:<target>` when relevant

## Command summary

Provide a compact table with:

- command
- purpose
- Tier A/B/C status
- typical output
- CI suitability

Commands to document:

- `cem-dom parse`
- `cem-dom validate`
- `cem-dom check`
- `cem-dom transform`
- `cem-dom convert`
- `cem-dom inspect`
- `cem-dom trace`
- `cem-dom bench`
- `cem-dom schema emit`
- `cem-dom schema sample`
- `cem-dom schema replace`
- `cem-dom fixture validate`
- `cem-dom fixture roundtrip`
- `cem-dom plugin list`
- `cem-dom plugin inspect`
- `cem-dom plugin run`

## Global options

Document:

- `--schema <uri-or-file>`
- `--content-type <type>`
- `--base-uri <uri>`
- `--fail-level parse|validate|strict`
- `--report-json <file-or-dir>`
- `--report-md <file-or-dir>`
- `--format <format>`
- `--out <file-or-dir>`
- `--source-map`
- `--config <file>`
- `--quiet`
- `--verbose`
- `--debug`
- `--no-color`

## Fail levels

Define:

- `parse`: exit non-zero only on fatal parse failure
- `validate`: exit non-zero on parse failure or hard validation violation
- `strict`: exit non-zero on parse failure, hard validation violation, or warning

Explain how this interacts with schema semver:

- compatible minor schema drift can produce warnings
- major schema mismatch fails validation
- strict mode treats warnings as failures

## Diagnostics and reports

Document diagnostic shape:

```ts
type CemDomDiagnostic = {
  uri: string;
  line: number;
  column: number;
  byteOffset?: number;
  code: string;
  severity: "info" | "warning" | "error" | "fatal";
  message: string;
  scope?: {
    schemaUri?: string;
    contentType?: string;
    namespaceUri?: string;
  };
};
```

Document:

* terminal output
* *.report.json
* *.report.md
* source maps
* exit codes

## Command reference

For each command, include:

* purpose
* status: Tier A/B/C or experimental
* syntax
* options
* examples
* output examples
* failure behavior
* related commands

`cem-dom parse`

Cover parsing to:

* events
* AST
* DOM JSON
* HTML
* XML

Examples:
```bash
cem-dom parse examples/semantic/login.html --format ast
cem-dom parse page.html --schema schemas/cem.cem --fail-level validate
```

`cem-dom validate`

Cover validation against schema, reports, accessibility/security/reference checks.

Examples:

```bash
cem-dom validate examples/semantic/*.html \
  --schema schemas/cem.cem \
  --report-json reports \
  --report-md reports
```

`cem-dom check`

Document as CI-friendly parse + validate.

Example:
```bash
cem-dom check examples/semantic/*.html --zero-hard-violations
cem-dom transform
```
Cover transform loading from file, URI, stream, or DOM-compatible input.

Examples:
```bash
cem-dom transform examples/semantic/login.html \
  --transform transforms/light-dom.cemx \
  --out dist/login.html
```

`cem-dom convert`

Cover schema-to-schema and format-to-format conversion.

Examples:
```bash
cem-dom convert button.cem \
  --from-schema schemas/cem-v1.cem \
  --to-schema schemas/cem-v2.cem \
  --out button.v2.cem
cem-dom convert page.html \
  --from-format html \
  --to-format ast \
  --out page.ast.json
```

`cem-dom schema emit`

Cover emitting schema mirrors and type headers.

Examples:
```bash
cem-dom schema emit schemas/cem.cem \
  --to relax-ng,typescript \
  --out-dir dist/schema
```

`cem-dom schema sample`

Cover minimal, typical, maximal, edge, and invalid sample generation.

Examples:
```bash
cem-dom schema sample schemas/component.cem \
  --root cem-component \
  --mode typical \
  --count 3 \
  --out examples/generated
```

`cem-dom schema replace`

Cover replacing a schema-governed sub-document.

Examples:
```bash
cem-dom schema replace page.html \
  --scope-uri https://cem.dev/schema/v1 \
  --with-schema schemas/cem-v2.cem \
  --rewrite-namespaces \
  --out page.v2.html
```

Also document selection by:

* scope URI
* namespace URI
* content type
* XPath
* CEM selector

`cem-dom inspect`

Cover inspecting:

* AST
* scopes
* schema bindings
* diagnostics
* source offsets
* plugins
* source maps

`cem-dom trace`

Cover deterministic trace output for debugging parser, validator, interpreter, transform, plugin, and scheduling behavior.

`cem-dom bench`

Cover parse/validate/transform performance budgets.

## Fixture commands
Document:
```bash
cem-dom fixture validate examples/semantic/*.html
cem-dom fixture roundtrip examples/semantic/*.html
```
Explain how these map to release verification.

## Plugin commands
Document:
```bash
cem-dom plugin list
cem-dom plugin inspect <plugin>
cem-dom plugin run <plugin> <input>
```
Explain observe vs mutate plugins, source-map requirements, scope inheritance, and plugin failure behavior.

# Exit codes
Define proposed exit codes:

* 0: success
* 1: parse/validation/strict failure
* 2: CLI usage error
* 3: schema resolution error
* 4: transform failure
* 5: plugin failure
* 6: I/O error
* 7: internal error

# Examples
Include end-to-end examples:

## Validate semantic fixtures in CI
```bash
cem-dom check examples/semantic/*.html \
  --schema schemas/cem.cem \
  --fail-level validate \
  --report-json reports \
  --report-md reports
```
## Convert a document to internal AST
```bash
cem-dom parse page.html --format ast --out page.ast.json
```
## Transform to light-DOM custom-element markup
```bash
cem-dom transform examples/semantic/login.html \
  --transform transforms/light-dom.cemx \
  --to light-dom \
  --out dist/login.html
```
## Generate samples from a schema
```bash
cem-dom schema sample schemas/component.cem \
  --root cem-component \
  --mode maximal \
  --out examples/generated/component.max.html
```
## Upgrade a schema-bound sub-document
```bash
cem-dom schema replace page.html \
  --scope-uri https://cem.dev/schema/v1 \
  --with-schema https://cem.dev/schema/v2 \
  --rewrite-namespaces \
  --source-map \
  --out page.v2.html
```
# Configuration file

Document a possible cem-dom.config.mjs:
```js
export default {
  schemas: {
    "https://cem.dev/schema/v1": "./schemas/cem-v1.cem",
    "https://cem.dev/schema/v2": "./schemas/cem-v2.cem"
  },
  reports: {
    json: true,
    markdown: true
  },
  failLevel: "validate",
  plugins: [
    {
      name: "security-checker",
      inputContentType: "text/html",
      mode: "observe",
      module: "./plugins/security-checker.mjs"
    }
  ]
};
```

# MVP boundary

Clearly identify Tier A CLI features:

* parse
* validate
* check
* transform
* schema emit for TypeScript and one XML schema mirror
* fixture validate
* fixture roundtrip
* JSON and Markdown reports
* fail levels

Mark as later or experimental:

* nested content-type scope switching
* plugin runtime management
* Rust type-header emission
* full NVDL dispatch
* full XSLT 4.0
* trace scheduling
* thread-pool debugging
* advanced source-map stitching

# Writing style

Use precise, implementation-friendly language.
Prefer concrete examples over abstract prose.
Do not invent final implementation details where an AC is still open.
Where behavior depends on an open question, add an “Open decision” callout.


## One useful framing sentence for the CLI docs

> `cem-dom` is the command-line interface for turning CEM documents into trustworthy artifacts: it parses input, validates it against schemas, transforms it into target DOM representations, emits stable reports and type artifacts, and gives maintainers CI-friendly tools for migration, debugging, and release verification.

