# CEM-ML Schema Packages

[cem-ml-schema-content-registry-design.md](../../../docs/cem-ml-schema-content-registry-design.md)

Schema packages are versioned schema modules registered by schema URL and
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
   scheme/host/port/authority/path-prefix/path-extension/path-basename/query/query-parameter-name/value/forbidden-parameter/required-parameter/fragment/forbidden-fragment constraints, and media-type essence, type, subtype,
   parameter-name allow-lists, parameter-value checks, and required-parameter
   checks, without moving package meaning into Rust.
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

These principles are the base contract for each package described below.

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
metadata is the source for schema URI, content type, namespace, and source-file
registration.

`cem-native-template/v1` defines the CEM-native template module language used
by template adapters. It owns `application/vnd.cem.template+cem` and also
claims current generic CEM source content types as aliases that require an
explicit schema when ambiguous.

`cem-transform/v1` defines CEMT (`.cemt`) converter-template resources. It
owns `application/vnd.cem.transform+cem` and reuses the CEM-native template
schema as its base language.

`cem-ql/v1` defines CEM-QL query source module and compiled query artifact
resource identities. It owns `application/vnd.cem.query+cem-ql`, claims
`text/cem-ql` as an authoring alias, and claims compiled artifact/cache aliases
for query binaries. CEM-QL source is not CEM-ML syntax; its parser lives in the
`cem-ql` crate.

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

Schema dependencies should be resolved by schema URL and content type, not by
filesystem path. Filesystem layout is a distribution detail for local packages.

## Package Folder Contract

Every built-in schema package is a self-describing versioned folder under
`packages/cem_ml/schema-packages/{package-id}/vN/`. The folder is the local
distribution unit for the schema, validation examples, output formatters, and
output colorizers:

```text
schema-packages/{package-id}/v1/
  package.cem
  schema/{package-id}.cem
  examples/
    {case}.{content-extension}
    {case}.example.cem
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

Schemas are always authored in `.cem` format under `schema/`. The schema source
declares the schema URI, version, owned content types, namespaces, content model,
constraints, diagnostics, and explicit `{uses}` dependencies.

Examples are not only naked source files. Each example set includes the source
file in the matching content type and a CEM-format example reference, either as
package manifest metadata or as a package-relative `.example.cem` sidecar. That
reference must name the source path, content type, schema URL, expected
pass/fail result, and expected diagnostic codes for invalid cases. When an
example is loaded, the loader resolves the declared content type and schema URL
and validates the source bytes against that schema; it must not rely on filename
extension inference alone.

Formatter assets live under `formatters/` and are CEMT (`.cemt`) transforms so
they participate in the normal output pipeline and preserve source-map ranges.
Every package should expose at least these formatter profiles:

- `compact`: the default profile, minimizing optional whitespace while keeping
  deterministic byte output.
- `pretty`: a readable profile aligned with common Prettier-style defaults for
  indentation, wrapping, and stable ordering.
- `tabular`: vertically aligned where useful, with scope closers kept on the
  same line when they fit.

Colorizer assets live under `colorizers/` and are also CEMT transforms over the
formatted CEM tree. Every package should expose at least these colorizer
profiles:

- `terminal`: terminal-safe output, including no-color and capability-aware ANSI
  variants.
- `html`: escaped HTML output with class or style metadata suitable for rendered
  previews.
- `md`: Markdown-safe colored output, using fenced or inline forms that preserve
  the underlying source ranges.

## Creating A Custom Schema Package

Use this checklist when adding a project-local or future external schema
package. Built-in packages use the same contract, but are embedded into the
runtime catalog by Rust code after validation.

1. Create a versioned package folder:

```text
schema-packages/{package-id}/v1/
  package.cem
  schema/{package-id}.cem
  examples/
    {case}.{content-extension}
    {case}.example.cem
  converters/
  formatters/
  colorizers/
```

2. Author `schema/{package-id}.cem` as a schema-definition document. It should
   declare the schema URI, version, owned namespaces, content model,
   constraints, diagnostics, and any `{uses}` dependencies on other schemas.
   Schema dependencies must be referenced by schema URI, not by filesystem path.

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
   shape, and at least one invalid contract. Pair every source example with a
   CEM-format example reference that names the content type and schema URL, link
   those examples from the package README, and keep expected diagnostics
   explicit.

8. Validate the manifest directly:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.schema-package+cem \
  --schema https://cem.dev/ns/schema-package/1 \
  schema-packages/{package-id}/v1/package.cem
```

9. Validate the package folder against source consistency rules. Built-in
   packages are covered by the CLI integration test; local packages should run
   the same validator before they are added to a runtime catalog.

Custom packages are not automatically trusted by the built-in runtime. A host
must explicitly load or embed the package descriptor before its content types,
namespaces, converters, or schema rules participate in registry resolution.

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
`@stringSuffixes`, `@itemCount`, `@minItems`, `@maxItems`, `@pathPrefixes`,
`@pathExtensions`, `@pathBasenames`, `@uriSchemes`, `@uriHosts`, `@uriPorts`,
`@uriRequiresAuthority`, `@uriPathPrefixes`, `@uriPathExtensions`,
`@uriPathBasenames`,
`@uriQueries`, `@uriQueryParameters`, `@uriQueryParameterValues`,
`@uriQueryForbiddenParameters`, `@uriQueryRequiredParameters`,
`@uriFragments`, `@uriForbiddenFragments`, `@mediaTypes`, and
`@mediaTypeTypes`/`@mediaTypeSubtypes`/`@mediaTypeSuffixes`/`@mediaTypeParameters`/
`@mediaTypeParameterValues`/`@mediaTypeForbiddenParameters`/
`@mediaTypeRequiredParameters` then narrow those compatible primitives
declaratively in the schema document.

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

Native producers exist for performance, bootstrap, binary framing, and clarity,
but are paired with CEMT implementations and cross-checked. Each supported
schema package should eventually declare:

- output source identity: CEM AST projection content type and schema URL;
- destination identity: owned content type and schema URL;
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
schema URL, and encoding category, and the result carries output identity so it
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
