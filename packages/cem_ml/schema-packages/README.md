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

## Bootstrap Relationship

`cem-ml/v1` defines the generic CEM-ML syntax and document model. It owns the
base `application/cem` content type, directive syntax, namespace binding,
elements, attributes, text nodes, content scopes, and handoff boundaries.

`schema/v1` defines the schema definition language expressed in CEM-ML. Schema
documents validate as CEM-ML documents first, then as instances of the schema
definition language. This package owns
`application/vnd.cem.schema+cem`.

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

Schema dependencies should be resolved by schema URL and content type, not by
filesystem path. Filesystem layout is a distribution detail for local packages.

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
   already formatted and colored tree.

7. Add examples that cover the smallest valid instance, the common production
   shape, and at least one invalid contract. Link those examples from the
   package README and keep expected diagnostics explicit.

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

The built-in CEM-ML package shows the pattern:

- [`cem-ml/v1/formatters/formatter-coloring-pipeline.cemt`](cem-ml/v1/formatters/formatter-coloring-pipeline.cemt)
- [`cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt`](cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt)
- [`cem-ml/v1/examples/formatter-coloring-pipeline.package-artifacts.fixture.cem`](cem-ml/v1/examples/formatter-coloring-pipeline.package-artifacts.fixture.cem)

## Direct References

Reusable schema relationships are declared inside schema documents with
`uses/use` entries:

```cem
{uses |
    {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
}
```

Downstream declarations refer to imported definitions with qualified names, for
example `schema:media-type` or `schema:uri`. This keeps package-specific schemas
small while preserving strict validation boundaries for their own instances.

## Output Transformations To Review

Input validation and output serialization are separate registry concerns. The
current validation work proves that source bytes can be loaded, associated with
schema URL plus content type, and checked against schema-owned rules. The next
design step is output export from CEM AST to each supported schema package's
syntax and destination content type.

The primary target is schema-owned serialization:

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

### Option A: Direct Schema-Owned CEMT Serializers

Each schema package declares one or more CEMT templates that serialize CEM AST
directly to the package's owned content types.

Example package metadata shape:

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

Pros:

- Strong schema ownership: HTML, XML, JSON, YAML, CSV, Markdown, CSS, and other
  packages own their output syntax rules next to validation rules.
- CEMT remains the primary conversion mechanism, with Rust fallback only for
  bootstrap, performance, or binary/escaping-heavy emitters.
- The registry can select serializers with the same explicit identity rules used
  for other converters.

Cons:

- Many serializers repeat traversal, escaping, indentation, and source-map
  mechanics unless shared CEMT helper modules are introduced.
- Some syntaxes need writer-level guarantees that are awkward in pure templates:
  JSON escaping, YAML scalar style, CSV quoting, XML namespace repair, HTML
  foreign-content insertion modes, and binary chunk framing.
- Bootstrap languages such as CEMT itself need a Rust fallback until the CEMT
  runtime can safely serialize its own templates.

### Option B: Shared AST Writer With Schema Syntax Profiles

The runtime provides a generic AST writer. Each schema package declares a syntax
profile: node mapping, token spelling, escaping policy, namespace policy,
whitespace policy, and content-type bindings. CEMT templates can still call the
writer, but do not hand-roll the full serializer.

Pros:

- Less duplication across XML-family packages, CEM-family packages, and
  structured data syntaxes.
- Centralizes byte-stable escaping, source-map generation, and streaming output
  contracts.
- Easier to enforce deterministic formatting and canonical output.

Cons:

- Requires a schema-level syntax-profile vocabulary before all packages can
  participate.
- Risks turning the writer into a second schema language if package-specific
  behavior is too expressive.
- CEMT becomes orchestration around the writer rather than the whole serializer.

### Option C: Layered AST To Target Model To Syntax

CEM AST is first transformed into a target semantic model, then a serializer for
that target model writes the destination syntax.

Examples:

```text
CEM AST -> HTML DOM model -> text/html
CEM AST -> XML DOM model -> application/xml
CEM AST -> JSON value model -> application/json
```

Pros:

- Best fit for formats with existing semantic models, normalization rules, or
  multiple serializations.
- Allows validation and conversion steps to operate on the target model before
  bytes are emitted.
- Reuses DOM/projection work and supports future normalized DOM outputs.

Cons:

- Adds a second transform stage even when a direct serializer would be enough.
- Can blur output serialization with content-type-to-content-type conversion.
- Requires target semantic models for formats that may only need syntax output.

### Option D: CEMT-First Producers With Shared Writer Primitives

Schema packages own CEMT output producer entries, and CEMT calls content-type
specific encoders, formatters, color output helpers, writer primitives, and
transformation helpers for byte-stable syntax tasks: escaping, namespace
declaration, attribute ordering, scalar style, CSV quoting, binary chunk
framing, source-map spans, terminal ANSI/SGR color output, HTML color output,
and canonical formatting. Native output producers are registered as paired
fallback or fast-path implementations for performance and clarity, and must be
cross-checked against the CEMT producer.

Pros:

- Preserves schema ownership and CEMT-first evolution.
- Keeps content-type-specific encoding, formatting, and color output inside the
  CEMT stack instead of as opaque host post-processing.
- Avoids duplicating the most error-prone syntax mechanics in templates.
- Lets packages move from native fallback to CEMT incrementally as writer
  primitives become available, while native remains available for performance.
- Makes native output producers executable oracles for CEMT parity.
- Keeps output serialization separate from content-type-to-content-type
  conversion while using the same registry edge model.

Cons:

- Requires a stable set of writer primitive APIs and CEMT bindings.
- Needs clear tests to prove CEMT output and native output remain equivalent
  under declared byte-exact, token-equivalent, parse-equivalent, or
  diagnostic-equivalent parity mode.
- Still requires per-schema output producer assets, encoder/formatter profiles,
  and examples.

Recommended review direction: Option D. CEMT is the primary output producer,
including transformation, syntax/context encoding, formatting, terminal/HTML
color output, source-map span creation, final artifact identity,
content-type-specific encoders, formatters, colorizers, writer primitives, and
small transformation helpers. Encoding here means target syntax/context work,
such as escaping, quoting, scalar style selection, namespace repair, and binary
chunk framing. It is separate from byte character encoding such as UTF-8 and
transport content encoding such as gzip.

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
proposal remains as an implementation backlog and worked-example source in
[`../docs/cemt-encoding-proposal.tmp.md`](../docs/cemt-encoding-proposal.tmp.md).

# list of embedded schema

* cem-ast-projection
* cem-dom-projection
* cem-events-projection
* cem-ml
* cem-native-template
* cem-ql
* cem-transform
* css
* csv
* html
* json
* json-schema
* markdown
* mathml
* relax-ng
* schema
* schema-package
* svg
* xhtml
* xml
* xslt
* yaml
