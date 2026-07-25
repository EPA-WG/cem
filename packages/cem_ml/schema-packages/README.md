# CEM-ML Schema Packages

Status: current implemented schema-package overview. Reference-normalization
target design lives in
[`../../../docs/cem-ml-reference-normalization-design.md`](../../../docs/cem-ml-reference-normalization-design.md).
This overview describes shipped package folders and current CLI behavior unless
it explicitly says "target design".

[cem-ml-schema-content-registry-design.md](../../../docs/cem-ml-schema-content-registry-design.md)

Schema packages are versioned schema modules registered by schema URI and
content type. The first packages form the bootstrap chain for the CEM stack:

```text
CEM-ML syntax
  -> schema definition language
    -> schema package manifest schema
      -> package.cem manifest instances
```

## Shared Package Principles

Every folder in `schema-packages/` follows the same ownership model, regardless
of whether the package describes CEM-ML, a query language, a projection, or an
external syntax such as JSON, HTML, or XML:

1. **Schemas are declarative CEM-ML documents.** A package schema is authored as
   `.cem` markup against the schema definition language. It declares nodes,
   attributes, content models, field contracts, constraints, diagnostics, and
   dependencies as data. Package-specific field semantics must not be hidden in
   Rust validators.
2. **The built-in vocabulary stays primitive.** The bootstrap model provides
   only general structural and scalar concepts such as document, node, element,
   attribute, text, string, boolean, integer, number, URI, media type, path,
   list, semantic version, identifier/name-list, qualified-name/type reference,
   and explicit symbol/wildcard reference forms. Their validation role follows
   the same pattern-oriented semantics as RELAX NG: primitives and composition
   describe an accepted document shape without embedding a package's domain
   model in the validator. Higher-level meaning belongs in the package's `.cem`
   schema. Schema-owned datatype parameters can refine compatible primitives,
   including string prefix/suffix checks, list item-count bounds, URI
   scheme/forbidden-scheme/host/forbidden-host/port/forbidden-port/authority/
   path-prefix/path-extension/path-basename/query/forbidden-query/
   query-parameter-name/value/forbidden-parameter/required-parameter/
   fragment/forbidden-fragment constraints, and media-type essence/
   forbidden-essence, type, subtype, parameter-name allow-lists,
   parameter-value checks, and required-parameter checks, without moving package
   meaning into Rust.
3. **Node references combine schema declaration with CEM-QL resolution.** A
   CEM-ML schema declares which fields or content positions carry a reference,
   including its type and constraints. CEM-QL supplies the query and resolution
   semantics used to select or follow another node. The reference contract
   remains part of the CEM-ML-authored schema; it is not a package-specific Rust
   pointer convention.
4. **Transformation chains are CEMT-owned.** The ordered load, parse, normalize,
   validate, project, and convert steps for a package are declared and composed
   by CEMT (`.cemt`) transformation resources. `package.cem` registers the
   transformation identities and endpoints. Native code may implement primitive
   host operations or a declared bootstrap/performance fallback, but it must not
   be the undeclared owner of package-specific chain ordering or semantics.
5. **Formatting and coloring are CEMT stages.** Formatter and colorizer
   selection, ordering, profiles, and composition are declared through CEMT
   transformation resources registered by `package.cem`. Formatting produces a
   formatted CEM tree, coloring enriches that tree, and only then does the
   writer emit target-native bytes. The writer does not recreate either chain
   in Rust.
6. **Command examples carry SVG previews.** Package README command-line
   examples that demonstrate visible output should be followed immediately by a
   package-relative SVG preview of the resulting terminal report, formatted
   bytes, rendered document, or other user-visible artifact. Store these
   previews under `examples/previews/` with names tied to the fixture and
   command/profile, for example `basic-table-pretty-terminal.svg`. When a
   command example, fixture, formatter, colorizer, converter, CLI report shape,
   or presentation output changes, AI-assisted edits must update the affected
   SVG preview in the same change or explicitly state that the preview remains
   unchanged because the visible output did not change.
7. **Resolver policy is separate from resource adapters.** CEM-ML resolver
   policy decides whether a request is denied, passed through, or explicitly
   substituted before a local read or `ResourceResolver` dispatch happens.
   `ResourceResolver` implementations stay byte/resource adapters: they read,
   write, or list the effective URI selected by policy and do not own trust,
   substitution, or diagnostic semantics. Any package that resolves external
   resources during compile, render, preflight, or runtime must preserve the
   requested URI, normalized URI, effective/resolved URI, substituted URI when
   present, resolver-policy stamp, source range, and resource content hash in
   diagnostics and cache/dependency identity. Passive validation, formatting,
   colorizing, and preview generation must not perform policy-sensitive
   resource reads unless that package explicitly documents a resolving mode.
8. **Embedded expression schemas are language-owned.** Parent packages declare
   expression slots, expected bindings, result type/nullability, evaluation
   phase, source ranges, and safety policy; they do not own a private
   expression grammar. The shared CEM expression contract is the CEM-QL
   expression schema owned by `cem-ql/v1`. Template, transform, schema
   behavior, and component packages consume that contract by declaring slots
   that delegate to CEM-QL parse, type, diagnostic, and evaluator semantics.
   Standalone expression execution must be exposed through the shared CEM-QL
   API and the CEM-ML `transform` CLI path so an expression can run against
   data without being wrapped in a query module or template module.

These principles are the base contract for each package described below.

## Format Support Definition Of Done

When adding or expanding support for a source or output format, the package is
not complete until the full lifecycle is represented in the package folder and
tests:

- formatter line endings: default output is LF (`\n`, Linux style) unless a
  package explicitly documents a different default. `lineEnding` is a generic
  formatter option, not a package-specific option; package-specific options use
  a namespace only when they express package-specific semantics. If an external
  standard requires or strongly expects another record separator, package docs
  must warn about the default conflict and document the strict interchange
  option, for example `lineEnding=crlf`;
- formatter page geometry: readable formatter profiles default to a four-space
  `indent`, an eight-column `tabSize`, and a 100-character `wrapColumn` target
  when the formatter performs wrapping. This README is the active implementation
  contract; [`indent-vs-tab-size.md`](../../../docs/indent-vs-tab-size.md) is
  the linked decision and rationale archive, not an independent source of
  acceptance criteria;
- formatter indentation: readable formatter profiles default to a four-space
  indent unit. `indent` is a generic formatter option whose value is the exact
  whitespace string to repeat per depth level; spaces and tabs in this value
  must be preserved rather than trimmed. Packages that emit indented output must
  read the generic option before applying package-specific layout rules;
- formatter tab stops: readable formatter profiles that emit literal tab
  characters default to an eight-column tab-stop assumption. `tabSize` is a
  generic formatter option whose positive integer value must be carried through
  formatter metadata and any preview renderer that expands tabs;
- formatter wrapping: readable formatter profiles that wrap output default to
  a 100-character soft target. `wrapColumn` is a generic formatter option whose
  positive integer value must guide wrapping without overriding target-format
  correctness or user-provided hard layout requirements;
- standards and registry mapping: cite the primary specification, registered
  content types, content-type parameters, fragment identifiers, and known
  interoperability notes;
- source identity: declare primary and alias content types in `package.cem`,
  normalize parameters generically, and keep package-specific policy in the
  schema or CEMT assets;
- parser facts: expose deterministic syntax facts, source ranges, encoding
  state, dialect state, and recoverable/fatal parser facts as data rather than
  package-specific Rust diagnostics;
- schema-owned diagnostics: bind those facts to declared constraints,
  severities, diagnostic codes, and structured details in `.cem` schema source.
  Resource parse-fact diagnostics use constraint-owned `@fact-kind` bindings so
  native parsers emit neutral facts and schemas decide which facts become
  diagnostics;
- examples and manifests: cover the smallest valid fixture, representative
  production shape, edge cases, invalid contract cases, and security-relevant
  cases through manifest-declared examples;
- formatter profiles: provide `compact`, `pretty`, and `tabular` behavior when
  meaningful, with explicit import-safe versus review/presentation boundaries;
- colorizer profiles: provide `terminal`, `html`, and `md` output when useful,
  preserving source-map ranges and writer-boundary metadata;
- command demos: include README command examples with adjacent SVG previews for
  stable visible output and keep previews refreshed with the commands and
  source assets;
- safety notes: document active-content, formula-injection, external-resource,
  entity-expansion, script execution, privacy, integrity, or spoofing concerns
  that apply to the format;
- verification: add focused Rust tests, CLI integration tests, package-local Nx
  `verify` inputs/outputs, and drift checks for generated artifacts;
- release behavior: state compatibility defaults, lossy options, unsupported
  dialects/features, and migration/versioning expectations.

## Package Review Protocol

Reviews of any schema package must audit the package against the shared
principles and format-support definition of done above. A review is incomplete
if it only checks that examples validate, only checks README prose, or only
checks a CLI demo. Review findings should explicitly cover these layers:

1. **Engine and CLI parity.** Package-owned behavior must be reachable through
   the core engine API and the CLI. A CLI wrapper can adapt flags and streams,
   but it must not be the only place where a source format enters its parser,
   formatter/colorizer, writer, diagnostics, or conversion metadata path. Direct
   engine tests are required when the package adds convert, parse, validate, or
   output support.
2. **Schema-owned facts and diagnostics.** Native code may extract byte-accurate
   parser facts, token streams, source ranges, encoding reports, or performance
   fallbacks. The package review must identify which facts are exposed as data
   and where the schema owns diagnostic codes, severities, policies, and
   structured details. Any remaining Rust-owned diagnostic policy must be
   documented as current boundary work with a target migration path.
3. **Resolver policy boundaries.** Imports, external resources, includes,
   schema dependencies, and package/module-map references must resolve through
   CEM-ML resolver policy. Packages may declare requested references, but they
   must not invent implicit fallback behavior. Denied references and unresolved
   references must have distinct diagnostics, and any explicit policy-owned
   substitution must preserve requested identity, substituted identity, source
   range, and artifact/cache stamp inputs.
4. **Package folder completeness.** `package.cem`, `schema/*.cem`, examples,
   formatter/colorizer artifacts, README sections, scripts, previews, and
   package-local `project.json` verify targets must agree. Every checked-in
   example must be manifest-declared with content type, schema, expected result,
   and expected diagnostics when applicable.
5. **Output pipeline shape.** Formatter/colorizer assets should declare
   `@produces="cem-tree"` and produce/consume formatted or colored CEM trees.
   Token arrays, HTML spans, ANSI codes, and other byte-oriented structures are
   writer-boundary implementation details unless the package explicitly owns a
   lower-level binary or token format.
6. **Profile semantics.** `compact`, `pretty`, `tabular`, `terminal`, `html`,
   and `md` profiles must either have distinct documented behavior or be
   explicitly documented and tested as intentional aliases until real behavior is
   implemented. Generic formatter options such as `lineEnding`, `indent`,
   `tabSize`, and `wrapColumn` must be reviewed across all profiles.
7. **README AC coverage.** The README must include standards/registry mapping,
   source identity, parser facts, formatter/colorizer profiles, command demos
   with adjacent SVG previews, safety/security notes, verification gates,
   release behavior, and tracked incomplete work.
8. **Verification and drift gates.** Package-local `verify` must fail when
   manifest validation, schema-owned example validation, formatter/colorizer
   output, HTML/terminal presentation, README previews, or generated artifacts
   drift. The target inputs must include package files and shared Rust/CLI code
   that can change package behavior.

When a review finds gaps, immediately convert the findings into executable
todo checkitems in `docs/todo.md` before implementation continues. Keep those
items specific enough that each can be closed by a code/doc change plus a named
verification command.

## Bootstrap Relationship

`cem-ml/v1` defines the generic CEM-ML syntax and document model. It owns the
base `application/cem` content type, directive syntax, namespace binding,
elements, attributes, text nodes, content scopes, and handoff boundaries.

`schema/v1` defines the schema definition language expressed in CEM-ML. Schema
documents validate as CEM-ML documents first, then as instances of the schema
definition language. This package owns
`application/vnd.cem.schema+cem`, including schema-owned field contracts for
child occurrence counts, child choices, child-set cardinality, and relative
child ordering, boundary placement, and exact/required/forbidden child sequences.

`schema-package/v1` defines the package manifest schema for
`schema-packages/{schema-name}/{version}/package.cem`. The schema package
schema is itself authored with the schema definition language, and manifest
instances validate against it. This package owns
`application/vnd.cem.schema-package+cem`.

Built-in runtime schema descriptors are loaded from embedded `package.cem`
manifests plus each schema document's explicit `{uses}` declarations. The
Rust registry keeps public constants for stable identities, but package
metadata is the source for schema URI, complete version identity, content type,
namespace, source-file registration, and descriptor provenance.

`cem-native-template/v1` defines the CEM-native template module language used
by template adapters. It owns `application/vnd.cem.template+cem` and also
claims current generic CEM source content types as aliases that require an
explicit schema when ambiguous. Template expression slots delegate to the
shared CEM-QL expression schema; the template package owns slot context and
phase policy, not expression grammar.

`cem-transform/v1` defines CEMT (`.cemt`) converter-template resources. It
owns `application/vnd.cem.transform+cem` and reuses the CEM-native template
schema as its base language.

`cem-ql/v1` defines CEM-QL query source module, shared expression schema, and
compiled query artifact resource identities. It owns
`application/vnd.cem.query+cem-ql`, claims `text/cem-ql` as an authoring alias,
and claims compiled artifact/cache aliases for query binaries. CEM-QL source is
not CEM-ML syntax; its parser lives in the `cem-ql` crate. Standalone
expression execution belongs to the same package/API surface, not a
template-package-specific feature. The Rust API exists, and the CEM-ML
`transform` command can run an inline `--template-expression` or a `*.cem-ql`
expression transformation file.

`json/v1` defines generic JSON text resource identity. It owns
`application/json` and claims `text/json` as an alias. JSON source is not
CEM-ML syntax, and this package intentionally does not claim JSON Schema or
CEM-specific projection/vendor `+json` content types.

`yaml/v1` defines generic YAML resource identity. It owns `application/yaml`
and claims the common compatibility aliases `application/x-yaml`, `text/yaml`,
and `text/x-yaml`. YAML source is not CEM-ML syntax; parser/adaptor support is
separate from the schema package. Vendor or domain-specific `+yaml` content
types should use their own packages.

`csv/v1` defines generic comma-separated value resource identity. It owns
`text/csv`, models header disposition, row and field order, quoted fields, and
source-map hooks. CSV source is not CEM-ML syntax; parser/adaptor support is
separate from the schema package.

`markdown/v1` defines generic Markdown resource identity. It owns
`text/markdown`, models variant and charset metadata, block and inline
structure, links, references, embedded HTML policy, and source-map hooks.
Markdown source is not CEM-ML syntax; parser/adaptor support is separate from
the schema package.

`xml/v1` defines generic XML resource identity. It owns `application/xml`,
claims the RFC 7303 XML aliases, and models XML declaration, charset,
namespace-aware element and attribute structure, processing instructions,
comments, CDATA, DTD/entity hooks, and source-map hooks. XML source is not
CEM-ML syntax; parser/adaptor support is separate from the schema package.
Domain media types ending in `+xml` use their own packages and may depend on
this package.

`relax-ng/v1` defines RELAX NG schema resource identity. It owns
`application/relax-ng+xml`, claims
`application/relax-ng-compact-syntax` as the compact-syntax alias, depends on
`xml/v1`, and models validation-schema resources, grammar start patterns,
pattern definitions, include/external-reference policy, and source-map hooks.

`xhtml/v1` defines XHTML resource identity. It owns
`application/xhtml+xml`, depends on `xml/v1`, claims the XHTML document
namespace, and models XML-backed HTML document structure, head/body ordering,
metadata, flow and phrasing content, foreign-content hooks, and source-map
hooks. `text/html` remains a separate HTML serialization identity owned by
`html/v1`.

`svg/v1` defines SVG resource identity. It owns `image/svg+xml`, depends on
`xml/v1`, claims the SVG document namespace, and models XML-backed vector
graphics structure, viewport, paint, geometry, text, definitions, filters,
animation, script/style policy, accessibility hooks, external-resource policy,
foreign-content hooks, and source-map hooks.

`mathml/v1` defines MathML resource identity. It owns `application/mathml+xml`
and the registered presentation/content MathML media type aliases, depends on
`xml/v1`, claims the MathML document namespace, and models XML-backed
mathematical structure, presentation/content profiles, semantics, annotations,
accessibility hooks, external-annotation policy, and source-map hooks.

`xslt/v1` defines XSLT stylesheet resource identity and the bounded legacy
custom-element XSLT compatibility identity. It owns `application/xslt+xml`,
claims `text/xsl` and current custom-element XSLT markers as aliases, depends on
`xml/v1`, claims the XSLT document namespace, and records version-pinned,
capability-gated transform execution without reintroducing browser-native XSLT
execution.

`html/v1` defines HTML resource identity. It owns `text/html`, claims the HTML
DOM namespace, and models HTML-parser recovery, normalized DOM structure,
doctype/quirks metadata, template inert fragments, script and external-resource
policy, custom-element hooks, SVG/MathML foreign-content dispatch, and
source-map hooks. HTML is not XML; XHTML remains the separate XML-backed
`application/xhtml+xml` package.

`css/v1` defines CSS stylesheet and scoped style content identity. It owns
`text/css`, models stylesheets, style blocks, style attributes, rules,
selectors, declarations, component values, custom properties, cascade scope
metadata, host document integration, external `@import` and `url()` policy, and
source-map hooks. CSS source is not CEM-ML syntax; parser/adaptor support is
separate from the schema package.

`json-schema/v1` defines JSON Schema document identity. It owns
`application/schema+json`, depends on `json/v1`, and models JSON Schema
dialect, vocabulary, reference, and validation-resource metadata separately
from generic JSON values.

`cem-dom-projection/v1`, `cem-ast-projection/v1`, and
`cem-events-projection/v1` define the semantic CEM DOM, AST, and event-stream
projection layers. Each projection package owns a primary CEM binary content
type (`application/vnd.cem.*+cem-bin`) and a `+json` debug/interchange alias.
The JSON aliases are views over the semantic projection schemas, not canonical
runtime transport formats.

## Validation Model

The relationship is layered validation, not broad inheritance:

- A schema document must parse as CEM-ML and validate against the schema
  definition language.
- `schema-package.cem` is a schema document, so it validates against
  `https://cem.dev/ns/schema/1`.
- `package.cem` is a manifest instance, so it validates against
  `https://cem.dev/ns/schema-package/1`.
- Converter declarations, including Rust hooks, CEMT templates, fallback
  symbols, and planner cost, are schema-owned `package.cem` metadata consumed by
  the built-in conversion registry.
- A package manifest does not inherit arbitrary schema-definition elements such
  as `element`, `attribute`, or `constraint` unless the package manifest schema
  explicitly permits them.
- Field contracts are schema-owned for all schemas. Required, optional, and
  forbidden fields, accepted children, value constraints, dependent fields,
  mutually exclusive groups, conditional rules, open-content policy, and
  diagnostic details must be declared in `.cem` schema source and evaluated
  generically by the runtime, not encoded as package-specific Rust field lists.
- Function-backed diagnostics are also schema-owned: behavior declarations use
  CEM-QL `@select`/`@match` expressions to choose failures and inline CEM-ML
  behavior functions to produce messages and structured details outside CEMT.
  Defaulted typed behavior parameters bind into those functions. Qualified
  function references resolve through schema `{uses}` aliases to reusable
  CEM-ML behavior functions that opt in with `@visibility="package"` or
  `@visibility="public"`. Diagnostic-scoped `{arguments}` bind non-default
  parameter overrides for function behaviors. Field-contract declarations can
  bind `@diagnostic` plus `@behavior` for dependency and choice/case engine
  primitives while keeping broad diagnostic family codes stable. Constraint
  declarations can do the same for operational engine primitives such as
  resource readability, parser/validation, and reference resolution. Engine
  behavior argument binding and the full engine primitive library are still
  being built out.

Schema dependencies should be resolved by schema URI and content type, not by
filesystem path. Filesystem layout is a distribution detail for local packages.

## Package Folder Contract

Every built-in schema package is a self-describing versioned folder under
`packages/cem_ml/schema-packages/{package-id}/vN/`. The folder is the local
distribution unit for the schema, validation examples, output formatters, and
output colorizers:

```text
schema-packages/{package-id}/v1/
  package.cem
  project.json
  schema/{package-id}.cem
  examples/
    {case}.{content-extension}
    previews/
      {case}-{command-or-profile}.svg
  formatters/
    compact.cemt
    pretty.cemt
    tabular.cemt
  colorizers/
    terminal.cemt
    html.cemt
    md.cemt
```

`package.cem` is the package index. It must make each package part discoverable:
the `{schema}` declaration points at the `.cem` schema source, `content-type`
entries declare the owned primary and alias content types, formatter/colorizer
artifacts point at `.cemt` output-stage transforms, and example declarations
identify the package-owned fixtures that should be validated.

Schemas are always authored in `.cem` format under `schema/`. The default source
path is `schema/{package-id}.cem`. The schema source declares the schema URI,
version, owned content types, namespaces, content model, constraints,
diagnostics, and explicit `{uses}` dependencies.

Two v1 bootstrap packages intentionally keep compatibility filenames instead of
the default `schema/{package-id}.cem` shape:

- `cem-ml/v1/schema/cem-ml-generic.cem` preserves the bootstrap generic CEM-ML
  schema identity embedded by the runtime catalog.
- `schema/v1/schema/cem-schema.cem` preserves the bootstrap schema-definition
  identity embedded by the runtime catalog.

Any additional nondefault schema filename must be documented here and encoded in
the schema-package structure audit before it is accepted.

Examples are not only naked source files. `package.cem` is the canonical example
reference index: every `{example}` declaration must name the source path,
content type, schema URI, expected pass/fail result, and expected diagnostic
codes for invalid cases. Package-relative `.example.cem` sidecars are optional
generated projections of the same manifest metadata; they are not required
distribution sources and must not override the manifest record. When an example
is loaded, the loader resolves the declared content type and schema URI and
validates the source bytes against that schema; it must not rely on filename
extension inference alone.

Package README command examples are demo contracts. When they show meaningful
stdout, report JSON, formatted text, or rendered output, place a matching SVG
preview directly after the fenced command block and store the asset in
`examples/previews/`. The preview should represent the command's stable
user-facing result rather than local build noise such as Cargo compilation
lines. The preview must be refreshed when relevant source fixtures, CEMT
formatters/colorizers/converters, CLI report fields, color palettes, spacing,
or presentation rules change.

Formatter assets live under `formatters/` and are CEMT (`.cemt`) transforms so
they participate in the normal output pipeline and preserve source-map ranges.
Every package should expose at least these formatter profiles:

- `compact`: the default profile, minimizing optional whitespace while keeping
  deterministic byte output.
- `pretty`: a readable profile aligned with common Prettier-style defaults for
  indentation, wrapping, and stable ordering.
- `tabular`: vertically aligned where useful, wrapping only after the
  `wrapColumn` target is reached and keeping scope closers on the same line
  when they fit.

Colorizer assets live under `colorizers/` and are also CEMT transforms over the
formatted CEM tree. Every package should expose at least these colorizer
profiles:

- `terminal`: terminal-safe output, including no-color and capability-aware ANSI
  variants.
- `html`: escaped HTML output with class or style metadata suitable for rendered
  previews.
- `md`: Markdown-safe colored output, using fenced or inline forms that preserve
  the underlying source ranges.

Every package folder is also an Nx library project named
`cem_ml_schema_package_{package-id-with-underscores}_v1`. The package-local
`project.json` owns the package inputs for caching and exposes a cached `verify`
target. That target runs `package.cem` through the release CLI at the parse
failure boundary and writes the package-local report artifact; full semantic
schema-package validation remains part of the final registry/package validation
gate. The target must track:

- `package.cem` and `README.md`;
- `schema/**/*.cem`;
- `formatters/**/*.cemt`, `colorizers/**/*.cemt`, and `converters/**/*.cemt`;
- every example fixture and SVG command preview under `examples/`.

Downstream CLI tests depend on these package targets through Nx instead of
treating schema-package files as unowned fixture inputs.

Converter endpoint checks are deliberately a final registry pass. Package-local
`verify` targets may parse `package.cem`, confirm readable package-owned
resources, and track local CEMT assets, but they must not depend on sibling
schema-package projects just because a converter references another package's
schema or content type. After all package-local targets are green, the CLI
registry gates load the full built-in catalog and validate endpoint
schema/content-type ownership, conversion graph behavior, and parity fixtures.
The current cross-package converter edge classes are `cem-ml` to the projection
packages, `html`/`xml` to the DOM projection, and the DOM projection back to
HTML/XML serializer packages.

## Creating A Custom Schema Package

Use this checklist when adding a project-local or future external schema
package. Built-in packages use the same contract, but are embedded into the
runtime catalog by Rust code after validation.

1. Create a versioned package folder:

```text
schema-packages/{package-id}/v1/
  package.cem
  project.json
  schema/{package-id}.cem
  examples/
    {case}.{content-extension}
  converters/
  formatters/
  colorizers/
```

2. Author `schema/{package-id}.cem` as a schema-definition document. Built-in
   bootstrap compatibility filenames must be documented in the exception list
   above and encoded in the structure audit. The schema source should declare
   the schema URI, version, owned namespaces, content model, constraints,
   diagnostics, and any `{uses}` dependencies on other schemas. Schema
   dependencies must be referenced by schema URI, not by filesystem path.

3. Author `package.cem` against
   `https://cem.dev/ns/schema-package/1`. The manifest must declare the package
   id/version, the schema URI and package-relative source file, owned content
   types, and namespace claims. Prefer a single `@primary=true` content type and
   mark compatibility spellings with `@alias=true`.

```cem
@doc cem-ml 1
@ns pkg = "https://cem.dev/ns/schema-package/1"
@default pkg

{package @id="note" @version="1.0.0" |
    {schema
        @uri="https://example.test/ns/note/1"
        @source="schema/note.cem"
    }

    {content-type @value="application/vnd.example.note+cem" @primary=true}
    {namespace @prefix="note" @uri="https://example.test/ns/note/1"}
}
```

4. Keep manifest metadata consistent with the schema source. Local validation
   checks that the manifest schema URI, content-type claims, and namespace
   claims match the referenced `schema/*.cem` source.

5. Add converter metadata only when the package owns a real conversion edge.
   CEMT converters must declare template path, template content type/schema,
   target entrypoint when needed, output syntax/category metadata for
   serializers, and parity fixtures when paired with a native fallback. Rust
   converters must declare `rust-symbol`; CEMT converters that name a Rust
   fallback must also declare `fallback-reason`.

6. Add output-stage CEMT assets when the package owns formatted or colored
   output. Formatter artifacts live under `formatters/`; colorizer artifacts
   live under `colorizers/`. Formatters convert source AST/projection data into
   a formatted CEM tree. Colorizers mutate that CEM tree into a colored,
   writer-ready CEM tree. The writer runs only after coloring and receives the
   already formatted and colored tree. The baseline formatter profiles are
   `compact`, `pretty`, and `tabular`; the baseline colorizer profiles are
   `terminal`, `html`, and `md`.

7. Add examples that cover the smallest valid instance, the common production
   shape, and at least one invalid contract. Declare every source example in
   `package.cem` with content type, schema URI, expected result, and explicit
   expected diagnostics for invalid cases. Link those examples from the package
   README. When the README includes command-line examples with visible output,
   add an SVG preview under `examples/previews/` immediately after each command
   block and update it whenever the command, fixture, formatter/colorizer,
   converter, CLI report shape, or presentation output changes. Generate
   `.example.cem` sidecars only when a downstream package consumer needs a
   CEM-format projection of the manifest metadata.

8. Validate the manifest directly:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.schema-package+cem \
  --schema https://cem.dev/ns/schema-package/1 \
  schema-packages/{package-id}/v1/package.cem
```

9. Validate the package folder against the package bootstrap rules. The
   validator first compares manifest/schema-source declarations without adding
   the package to a registry: schema URI declaration, content-type claims, and
   namespace claims must match the referenced `schema/*.cem` source. Only after
   those pure checks pass may the validator construct an isolated provisional
   descriptor and run registry-backed endpoint, example, artifact, and
   namespace checks against built-ins plus that provisional overlay. Built-in
   packages are covered by the CLI integration test; local packages should run
   the same validator before they are added to a runtime catalog.

Custom packages are not automatically trusted by the built-in runtime. A host
must explicitly load or embed the validated package descriptor before its
content types, namespaces, converters, or schema rules participate in global
registry resolution. A provisional validation overlay is local to the validator
run and is discarded unless required checks pass.

Target descriptor records preserve provenance for this migration: complete
schema identity, package id/version, manifest and schema source artifact
identity, raw and normalized content-type claims, namespace claims, descriptor
origin, registry layer, match rule, and source ranges when available. Current
CLI/report projection may keep shipped diagnostic codes and broad value buckets,
but structured metadata should retain the target operand bindings,
declared/normalized values, lookup provenance, and per-item or per-operand
reasons.

## Schema-Owned CEMT Output Pipeline

Formatter and colorizer files are schema-package assets, not loose runtime
overrides. A package that owns output behavior keeps those assets in the same
versioned hierarchy as its schema:

```text
schema-packages/{package-id}/v1/
  package.cem
  schema/{package-id}.cem
  formatters/{profile-or-function}.cemt
  colorizers/{profile-or-function}.cemt
```

The manifest declares each file with `artifact @kind="formatter"` or
`artifact @kind="colorizer"`, including the CEMT function name, profile, target
content type, target schema, and `target-category="cem-tree"`. Runtime
selection uses an explicit function selector first (`cemtFormatter` /
`cemtColorizer`, or the matching CLI flags), then profile fallback when that
profile is unambiguous.

The output pipeline is intentionally staged:

```text
source AST/projection
  -> CEMT formatter
  -> formatted CEM tree
  -> CEMT colorizer
  -> colored CEM tree
  -> writer
  -> target-native formatted content
```

The writer is the final phase. It does not choose layout, color roles, or
writer-owned classes; those decisions must already exist on the colored CEM
tree. When a decision is between JSON and CEM-native representation for these
stage fixtures, keep the CEM-native representation and prompt only when the
target format itself requires another syntax.

Package formatter and colorizer output functions should declare
`@produces="cem-tree"` for this staged path. Lower-level writer streams such as
token arrays remain writer-boundary implementation details, not the public
formatter/colorizer artifact contract. When a target format is naturally emitted
as ordered tokens, represent those tokens as nodes inside the formatted or
colored CEM tree with explicit writer-boundary metadata, then let the generic
writer perform the final token-to-text or token-to-byte emission.

CLI output follows the same target-identity rule. `cem-ml convert` writes
stdout or `--out` in the requested target format: HTML as HTML, CEM-ML as CEM
text, YAML as YAML, and JSON only when JSON is the requested target. Structured
JSON conversion artifacts are debug sidecars written with `--artifact-json`;
they do not replace the primary native output. The CLI validation target keeps
checked-in `convert --config` examples for HTML, CEM-native, and YAML outputs
and compares their generated bytes against expected native fixtures.

The built-in CEM-ML package shows the pattern:

- [`cem-ml/v1/formatters/cem-format-tree.cemt`](cem-ml/v1/formatters/cem-format-tree.cemt)
- [`cem-ml/v1/formatters/cem-format-tree-helpers.cemt`](cem-ml/v1/formatters/cem-format-tree-helpers.cemt)
- [`cem-ml/v1/formatters/formatter-coloring-pipeline.cemt`](cem-ml/v1/formatters/formatter-coloring-pipeline.cemt)
- [`cem-ml/v1/formatters/cem-tree-helpers.cemt`](cem-ml/v1/formatters/cem-tree-helpers.cemt)
- [`cem-ml/v1/colorizers/cem-color-tree.cemt`](cem-ml/v1/colorizers/cem-color-tree.cemt)
- [`cem-ml/v1/colorizers/cem-color-tree-helpers.cemt`](cem-ml/v1/colorizers/cem-color-tree-helpers.cemt)
- [`cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt`](cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt)
- [`cem-ml/v1/colorizers/cem-tree-helpers.cemt`](cem-ml/v1/colorizers/cem-tree-helpers.cemt)
- [`cem-ml/v1/examples/formatter-coloring-pipeline.package-artifacts.fixture.cem`](cem-ml/v1/examples/formatter-coloring-pipeline.package-artifacts.fixture.cem)

Those artifacts keep selected public formatter/colorizer functions as wrappers
over schema-owned helpers named `cem.*` and `cemml.cem-tree.*`, including the
canonical formatter builder helpers `cem.format-tree.build-nodes` and
`cem.format-tree.build-envelope`. Helpers that do not represent formatter or
colorizer output stages use internal `{function @returns=...}` declarations
with typed return contracts. New schema-specific output functions should reuse
that wrapper pattern: pass
formatter/color decisions, writer-boundary metadata, and queued edits as
CEM-native data into the helper instead of copying the whole staged pipeline
body. Helper functions live in dedicated `formatter-helper` and
`colorizer-helper` artifacts under the same schema package hierarchy, and the
runtime loads matching helpers for the selected output stage before executing
the public formatter/colorizer body.

Canonical formatter node traversal now lives in the schema-owned
`cem.format-tree.build-nodes` CEMT helper, and canonical coloring now lives in
the schema-owned `cem.color-tree.apply-stage` helper over the formatted
`cem-tree`. Native CEMT runtime operations remain only for lower-level
formatting primitives that still need host-provided writer-policy data, such as
block-child whitespace and content boundaries.

## Direct References

Reusable schema relationships are declared inside schema documents with
`uses/use` entries:

```cem
{uses |
    {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
}
```

Downstream declarations refer to imported definitions with qualified names, for
example `schema:media-type`, `schema:uri`, or `schema:path`. `path` values are
scoped resource specifiers rather than document-relative filesystem paths:
`./...` resolves against the active context root, protocol values resolve through
the matching resolver, and bare values are module-map specifiers. This keeps
package-specific schemas small while preserving strict validation boundaries for
their own instances. Attribute datatype params such as `@stringPrefixes`,
`@stringSuffixes`, `@stringForbiddenPrefixes`, `@stringForbiddenSuffixes`,
`@itemCount`, `@minItems`, `@maxItems`, `@pathPrefixes`,
`@pathForbiddenPrefixes`, `@pathDirectoryNames`,
`@pathForbiddenDirectoryNames`, `@pathExtensions`, `@pathForbiddenExtensions`,
`@pathBasenames`, `@pathForbiddenBasenames`, `@uriSchemes`,
`@uriForbiddenSchemes`, `@uriHosts`, `@uriForbiddenHosts`, `@uriPorts`,
`@uriForbiddenPorts`, `@uriRequiresAuthority`, `@uriPathPrefixes`, `@uriForbiddenPathPrefixes`,
`@uriPathExtensions`, `@uriForbiddenPathExtensions`, `@uriPathBasenames`,
`@uriForbiddenPathBasenames`,
`@uriQueries`, `@uriForbiddenQueries`, `@uriQueryParameters`, `@uriQueryParameterValues`,
`@uriQueryForbiddenParameters`, `@uriQueryRequiredParameters`,
`@uriFragments`, `@uriForbiddenFragments`, `@mediaTypes`,
`@mediaTypeForbiddenEssences`, and
`@mediaTypeTypes`/`@mediaTypeSubtypes`/`@mediaTypeSuffixes`/
`@mediaTypeForbiddenTypes`/`@mediaTypeForbiddenSubtypes`/
`@mediaTypeForbiddenSuffixes`/`@mediaTypeParameters`/
`@mediaTypeParameterValues`/`@mediaTypeForbiddenParameters`/
`@mediaTypeRequiredParameters` then narrow those compatible primitives
declaratively in the schema document.

For package path-layout field contracts, the initial generic vocabulary is
intentionally limited to prefix, directory-name allow/forbid, extension, and
basename allow/forbid facets. The parallel `schema:path` datatype params expose
only the existing prefix/forbidden-prefix, directory-name, extension, and
basename allow/forbid checks listed above. These facets run after `schema:path`
resolution in the active scope context; they do not inspect the authored
spelling as a document-relative path. Additional generic facets such as path
depth, segment count, suffix, glob or segment classes, and alias or module-map
matching remain deferred until a concrete schema-owned check needs stable
cross-protocol semantics. Package-specific Rust helpers must not introduce
those checks as hidden generic CEM path semantics.

Reference-normalization target design treats these path values as URI identity
finalization against the active base, resolver purpose, package/module-map
context, and policy. Finalization does not imply that the resource exists or is
readable; schema-owned resource checks own those assertions.

## CEMT Transformation Ownership

Input validation, content conversion, and output serialization are separate
registry concerns, but their package-specific composition has one owner: CEMT.
Each package registers CEMT transformations in `package.cem`, and `.cemt`
resources define the executable chain from loaded input through the appropriate
semantic model to the requested destination.

Schema-owned serialization follows this shape:

```text
CEM AST
  -> schema-owned output transform
    -> destination syntax
      -> destination content type
```

Content-type-to-content-type conversion remains a separate planning surface.
For example, `text/html -> application/xhtml+xml` should be modeled as a
conversion between two content identities, while `CEM AST -> text/html` is an
output serializer for the HTML schema package.

A package declares a CEMT conversion edge with metadata of this form:

```cem
{converter
    @id="ast-to-html"
    @implementation="cemt"
    @template="templates/ast-to-html.cemt"
    @template-content-type="application/vnd.cem.transform+cem"
    @streamable=true
    @lossiness="syntax-normalized" |
    {from @content-type="application/vnd.cem.ast+cem-bin" @schema="https://cem.dev/ns/projection/ast/1"}
    {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
}
```

The chain may use a target semantic model when the format requires one:

```text
CEM AST -> HTML DOM model -> text/html
CEM AST -> XML DOM model -> application/xml
CEM AST -> JSON value model -> application/json
```

CEMT is the primary output producer and chain definition. It owns
transformation, syntax/context encoding, formatting, terminal/HTML color
output, source-map span creation, and final artifact identity. CEMT may call
shared content-type-specific encoders, formatter/colorizer helpers, and writer
primitives for byte-stable operations such as escaping, namespace declaration,
attribute ordering, scalar style, CSV quoting, and binary chunk framing.
Encoding here means target syntax/context work; it is separate from character
encoding such as UTF-8 and transport encoding such as gzip.

For the current schema-package manifest surface, artifact paths and function
names are still authored lexical fields. The reference-normalization target
interprets a manifest artifact path as document/artifact identity after
resolver context is applied, a manifest function name as exact exported symbol
spelling, and a compiled CEMT declaration as a function identity record tied to
the resolved artifact/module. Profile selectors remain exact dotted symbols.
Declarative artifact validation uses that split in stages: resource-readable
and resource-parse checks own artifact availability and CEMT syntax validity,
then the read-only `schema:cemt-output-function` lookup selects exported
`encoding-function`, `format-function`, or `color-function` declaration
metadata by resolved artifact identity plus lexical function name. Contract
checks compare manifest target content type, target schema, category, profile,
and subject metadata against the selected declaration without executing the
CEMT body or writer pipeline.

Native producers exist for performance, bootstrap, binary framing, and clarity,
but are paired with CEMT implementations and cross-checked. Each supported
schema package should eventually declare:

- output source identity: CEM AST projection content type and schema URI;
- destination identity: owned content type and schema URI;
- CEMT output producer: asset path, content type, and entrypoint;
- encoder and formatter profiles: escaping, namespace, whitespace, ordering,
  line ending, scalar style, chunk framing, and canonicalization policy;
- color output profile: semantic style roles, terminal capability policy,
  HTML class/style policy, palette, and no-color/accessibility fallback;
- native paired producer: Rust symbol, readiness, fallback or fast-path reason,
  and parity mode;
- examples: source AST fixture, expected bytes, diagnostics, and round-trip or
  parse-back validation where applicable.

CEMT output templates should use standard encoding, formatting, and color output
function surfaces rather than hand-written escaping or host-side color filters.
The primary call is:

```text
encode(subject, target, options?) -> encoded-artifact
```

`subject` is unencoded typed data, `target` identifies destination content type,
schema URI, and encoding category, and the result carries output identity so it
cannot be silently double-encoded or inserted into the wrong context. Formatting
and terminal/HTML color helpers belong to the same CEMT stack.

This is a schema-owned serializer contract, not hidden content-type conversion.
For example, `CEM AST -> text/html` is output production for the HTML package,
while `text/html -> normalized HTML model -> application/xhtml+xml` is a
content-type conversion pipeline that may parse, normalize, validate, and
change semantic models. Both use registry identities, but the runtime exposes
separate planning domains for content conversion and schema output production.
The canonical CEMT output contract is maintained in
[`cem-transform/v1/README.md`](cem-transform/v1/README.md). The temporary
implementation backlog and worked examples remain in
[`../docs/cemt-encoding-proposal.tmp.md`](../docs/cemt-encoding-proposal.tmp.md).

## Embedded Schema Packages

- `cem-ast-projection`
- `cem-dom-projection`
- `cem-events-projection`
- `cem-ml`
- `cem-native-template`
- `cem-ql`
- `cem-transform`
- `css`
- `csv`
- `html`
- `json`
- `json-schema`
- `markdown`
- `mathml`
- `relax-ng`
- `schema`
- `schema-package`
- `svg`
- `xhtml`
- `xml`
- `xslt`
- `yaml`
