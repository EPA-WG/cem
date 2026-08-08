# CEM-ML Generic Schema Package

Status: bootstrap schema, examples, formatter/colorizer CEMT assets, README
previews, and package-local verification frame

[cem-ml-syntax.md](../../../../../docs/cem-ml-syntax.md)

This package is the first schema-package source for the generic CEM-ML document
model. It owns CEM-ML syntax and content-type identity, not domain semantics.
Domain vocabularies such as CEM core annotations, HTML, SVG, templates, query,
transform, and schema-package manifests are separate packages layered on top of
this generic model.

## Source Identity

Owned schema URI:

```text
https://cem.dev/ns/cem-ml/1
```

Schema source:

```text
schema/cem-ml-generic.cem
```

This filename is the documented v1 bootstrap exception to the default
`schema/cem-ml.cem` shape. It preserves the generic CEM-ML schema identity
embedded by the runtime catalog while the package still follows the rest of the
versioned folder contract.

Primary content type:

```text
application/cem
```

Current aliases mirror the Rust runtime's existing accepted CEM source content
types:

- `text/cem-ml`
- `text/cem`
- `application/cem+xml`

The semantic CEM annotation vocabulary remains in
`packages/cem_ml/schema/cem-core.md` under `https://cem.dev/ns/core/1`.

## Syntax Facts And Diagnostics

The generic CEM-ML schema declares persisted document directives, namespace
directives, default namespace directives, schema hints, lexical node forms,
attributes, text, comments, CDATA/raw text, expression nodes, and typed content
handoff scopes. Parser output preserves byte ranges, line/column coordinates,
namespace context, and handoff boundaries for downstream schema validation,
projection, formatting, and source-map reporting.

The package schema declares the diagnostics used by its examples and current
runtime validation reports:

- `cem.doc.version_missing`
- `cem.doc.semver_invalid`
- `cem.doc.format_unknown`
- `cem.doc.version_unsupported`
- `cem.doc.prerelease_unmatched`
- `cem.ast.unbalanced_close`
- `cem.ast.unclosed_scope`
- `cem.ast.unresolved_reference`
- `cem.syntax.unclosed_scope`
- `cem.syntax.invalid_name`
- `cem.namespace.unbound_prefix`
- `cem.schema.unbalanced_close`
- `cem.schema.unclosed_scope`
- `cem.schema.unresolved_namespace`
- `cem.schema.unresolved_namespace_allowed`
- `cem.schema.unresolved_namespace_ignored`
- `cem.schema.unknown_annotation`
- `cem.schema.unknown_annotation_value`
- `cem.schema.disallowed_state`
- `cem.schema.state_not_allowed_for_role`
- `cem.schema.scoping.exclusive_src_select`
- `cem.schema.scoping.missing_source`
- `cem.ns.invalid_ns_directive`
- `cem.schema.unsupported_constraint`
- `cem.handoff.xslt_dispatched`
- `cem.xslt.version_invalid`
- `cem.handoff.child_parser_deferred`
- `cem.handoff.unsupported_content_type`
- `cem.content_type.unsupported_handoff`

Current formatter boundary: canonical CEM-ML formatter traversal,
block-child whitespace construction, content-boundary fragments, and
close-scope indentation are schema-owned CEMT helpers under
`formatters/cem-format-tree-helpers.cemt`. Native Rust remains the writer host
boundary only: it consumes formatted and optionally colored `cem-tree` nodes,
serializes structural target syntax, and renders formatter-owned fragments
without inventing package layout.

## Folder Contract

`package.cem` is the manifest-owned index for this folder. It declares the
schema URI and source file, the primary and alias content types, namespace
claims, Rust-backed projection converter metadata, formatter/colorizer CEMT
artifacts, helper artifacts, and every validation example under `examples/`.

Example metadata is intentionally manifest-owned. This package does not require
checked-in `.example.cem` sidecars because `package.cem` already records the
example path, content type, schema URI, expected pass/fail result, and expected
diagnostic codes.

## Converter Edges

The generic CEM-ML package owns the source side of the bootstrap projection
converters. They are declared as Rust hooks for current runtime availability,
while endpoint schema/content-type ownership is validated in the final registry
gate with the projection packages loaded.

| Converter | From | To | Implementation |
| --- | --- | --- | --- |
| `cem-ml-to-dom-projection-rust` | `application/cem`, `https://cem.dev/ns/cem-ml/1` | `application/vnd.cem.dom+cem-bin`, `https://cem.dev/ns/projection/dom/1` | `CemMlDomProjectionConverter` |
| `cem-ml-to-ast-projection-rust` | `application/cem`, `https://cem.dev/ns/cem-ml/1` | `application/vnd.cem.ast+cem-bin`, `https://cem.dev/ns/projection/ast/1` | `CemMlAstProjectionConverter` |
| `cem-ml-to-events-projection-rust` | `application/cem`, `https://cem.dev/ns/cem-ml/1` | `application/vnd.cem.events+cem-bin`, `https://cem.dev/ns/projection/events/1` | `CemMlEventsProjectionConverter` |

## Formatter And Colorizer Assets

Schema-local output transformations live beside the schema package:

- [`formatters/cem-format-tree.cemt`](formatters/cem-format-tree.cemt)
  declares `cem.format-tree` for `application/cem` /
  `https://cem.dev/ns/cem-ml/1`.
- [`formatters/cem-format-tree-helpers.cemt`](formatters/cem-format-tree-helpers.cemt)
  declares private canonical formatter helpers used by `cem.format-tree`,
  including `cem.format-tree.apply-stage`,
  `cem.format-tree.build-nodes`, and `cem.format-tree.build-envelope`.
- [`formatters/formatter-coloring-pipeline.cemt`](formatters/formatter-coloring-pipeline.cemt)
  declares `acme.showcase.format-tree` as a package-qualified formatter that
  extends `cem.format-tree`.
- [`formatters/cem-tree-helpers.cemt`](formatters/cem-tree-helpers.cemt)
  declares private `cemml.cem-tree.*` formatter helpers used by schema-local
  formatter entrypoints.
- [`colorizers/cem-color-tree.cemt`](colorizers/cem-color-tree.cemt)
  declares `cem.color-tree` for formatted CEM trees before the writer phase.
- [`colorizers/cem-color-tree-helpers.cemt`](colorizers/cem-color-tree-helpers.cemt)
  declares private canonical colorizer helpers used by `cem.color-tree`.
- [`colorizers/formatter-coloring-pipeline.cemt`](colorizers/formatter-coloring-pipeline.cemt)
  declares `acme.showcase.color-tree` as a package-qualified colorizer that
  extends `cem.color-tree`.
- [`colorizers/cem-tree-helpers.cemt`](colorizers/cem-tree-helpers.cemt)
  declares private `cemml.cem-tree.*` colorizer helpers used by schema-local
  colorizer entrypoints.

The package manifest declares entrypoint files as `formatter` and `colorizer`
artifacts and shared helper files as `formatter-helper` and `colorizer-helper`
artifacts so runtime CEMT stages are tied to schema-owned assets instead of
inline Rust template strings. The artifact entries also declare the target CEM
tree identity (`application/cem`, `https://cem.dev/ns/cem-ml/1`, `cem-tree`),
the supplied CEMT function name, the CEMT function profile when present, and
the formatter/color profiles that select the asset during output pipeline
execution.

The formatter profiles use one CEMT body with profile-aware layout:

- `compact` keeps minimal deterministic spacing and is the interchange-safe
  default;
- `pretty` expands non-text child groups into block layout;
- `tabular` keeps attributes inline while they fit within `wrapColumn`, then
  lays wrapped attributes out on vertically aligned parent + 1 indent lines.

Default output line endings are LF (`\n`, Linux style). Readable indentation
defaults to four spaces per depth level. `lineEnding` and `indent` are generic
formatter options shared across packages; CEM-ML does not define
package-specific line-ending or indentation options. Readable formatters that
emit literal tab characters use generic `tabSize` for the visual tab-stop
assumption, which defaults to `8`.

The colorizer profiles map to writer-boundary behavior:

- `terminal` records terminal output plus auto capability metadata and renders
  ANSI/SGR-colored CEM text;
- `html` maps to the class-based HTML mode and materializes HTML writer
  attributes;
- `md` records Markdown output metadata and renders Markdown-safe inline HTML
  color spans;
- `none`, `classes`, `inline-style`, and `css-custom-properties` are manifest
  selectors for current package-local output pipelines.

Formatters produce formatted CEM tree output, colorizers enrich that tree, and
only the writer emits terminal, HTML, Markdown, or source bytes. Token arrays,
ANSI codes, and HTML spans are writer-boundary implementation details.

Package-qualified formatter and colorizer artifacts are selected by explicit
CEMT function name first, then by stage profile fallback. This keeps the
canonical `cem.format-tree` and `cem.color-tree` pipeline stable while allowing
showcase or schema-specific formatter/colorizer bodies to opt in through the
same manifest-declared asset path.

`cem.format-tree.build-nodes` performs canonical node traversal in CEMT with
`typeOf`, `match`, `map`, `length`, numeric depth helpers, and helper calls. It
uses schema-owned helpers for inter-node whitespace, block-child indentation,
content-boundary fragments, attribute spacing, and close-scope indentation.
The Rust writer host contract starts after this stage: it consumes
formatter-owned `whitespace`, `raw`, and `format-token` fragments exactly as
provided, emits CEM delimiters and escaped scalar text, materializes requested
terminal/HTML/Markdown color output, and must not repair missing formatter
layout by synthesizing package-specific spacing.

`cem.color-tree.apply-stage` performs canonical coloring in CEMT over the
already formatted `cem-tree`. It reads the selected `$colorProfile`, recursively
annotates `nodes`, `formatNodes`, and `colorNodes`, materializes writer
attribute nodes and text wrappers for HTML profiles, and leaves the writer as
the final serialization phase for the colored tree.

## Safety Notes

CEM-ML source can embed raw handoff content such as HTML, CSS, JavaScript, XML,
JSON, templates, or future vendor media types. Validation and formatting must
treat handoff payloads as data unless a downstream package explicitly parses or
executes that content. Unsupported handoffs fail closed with
`cem.handoff.unsupported_content_type`; supported but deferred child parsers
record `cem.handoff.child_parser_deferred` so callers can choose whether to
continue.

Formatters and colorizers must preserve source ranges and avoid executing
embedded content. HTML and Markdown writers must escape source text and
generated attributes through the writer boundary. Resolver-sensitive work must
use the shared resolver-policy layer; passive validation, formatting,
colorizing, and preview generation should not perform policy-sensitive resource
reads.

## Formatter And Preview SDLC

When a command example, fixture, formatter, colorizer, converter, CLI report
shape, or visible presentation output changes, regenerate the README examples
with the package `samples2readme` target. Valid UTF-8 CEM sources remain exact
fenced source; an SVG preview is allowed only for an unfenceable fallback.

The package `verify` target checks generated README source and any referenced
fallback preview artifacts for drift.

## Verification

Focused package gates:

```bash
cargo test -p cem-ml cem_ml_package_examples_are_manifest_indexed
```

```bash
cargo test -p cem-ml-cli schema_owned_cem_ml_examples_validate_through_cli
```

```bash
cargo test -p cem-ml cem_tree_output_templates_are_schema_package_assets
```

```bash
cargo test -p cem-ml conversion_output_pipeline_applies_literal_baseline_formatter_profiles
```

```bash
cargo test -p cem-ml convert_target_cem_pretty_aligns_block_closing_braces_with_opening_indent
```

```bash
cargo test -p cem-ml conversion_output_pipeline_applies_literal_baseline_colorizer_profiles
```

```bash
yarn nx run cem_ml_schema_package_cem_ml_v1:verify
```

## Release Behavior

This package is versioned as `1.0.0`. The primary `application/cem` content
type and `https://cem.dev/ns/cem-ml/1` schema URI are bootstrap compatibility
anchors. Alias content types are accepted for current runtime compatibility,
but persisted examples use the primary content type unless an alias behavior is
being tested explicitly.

Projection converter edges are ready Rust bootstrap hooks. Formatter and
colorizer assets are package-owned CEMT resources. Rust host primitives remain
only at the writer boundary and for the legacy intrinsic fallback used when a
caller bypasses package CEMT output assets.

Tracked but not complete:

- additional alias content-type examples if alias-specific parser or lifecycle
  behavior changes.

<!-- package-review-waiver: additional alias content-type examples if alias-specific parser or lifecycle behavior changes. -->

Alias-specific examples are waived while aliases share the primary
`application/cem` parser and lifecycle behavior. They become required when an
alias adds distinct parser, lifecycle, validation, formatter, or diagnostic
behavior.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic</summary>

- Source: [`examples/basic.cem`](./examples/basic.cem)
- Content type: `application/cem`
- Schema: `https://cem.dev/ns/cem-ml/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-ml/v1/examples/basic.cem,contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1

{article @id="welcome" @attr1="abc"  @attr2="long attr value"  @attr3="abc"  @attr4="abc"  @attr5="abc"
    @attr6="abc"  @attr7="abc" |
    {h1 | Welcome}
    {p | This is a minimal CEM-ML document.}
}
```

<details>
<summary>nested-handoff</summary>

- Source: [`examples/nested-handoff.cem`](./examples/nested-handoff.cem)
- Content type: `application/cem`
- Schema: `https://cem.dev/ns/cem-ml/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-ml/v1/examples/nested-handoff.cem,contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{main @id="profile" |
    {section @class="summary" |
        {h1 | Ada Lovelace}
        {@type="text/html" |
            <p><strong>Known for:</strong> analytical engine notes.</p>
        }
    }
}
```

<details>
<summary>embedded-handoffs</summary>

- Source: [`examples/embedded-handoffs.cem`](./examples/embedded-handoffs.cem)
- Content type: `application/cem`
- Schema: `https://cem.dev/ns/cem-ml/1`
- Expected result: `pass`
- Expected diagnostics: `cem.handoff.child_parser_deferred`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-ml/v1/examples/embedded-handoffs.cem,contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

````cem
@doc cem-ml 1
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{main @id="embedded-handoffs" |
    {section @class="style-script-payloads" |
        {h1 | Embedded handoff coverage}
        {@type="text/css; charset=utf-8" |
            ```.card { color: var(--accent); }```
        }
        {@type="application/javascript" |
            ```export default { title: "Atoms/Button", args: { label: "Save" } };```
        }
    }
    {section @class="xml-json-payloads" |
        {@type="application/xml" |
            ```<resource><body><![CDATA[{token-name}]]></body></resource>```
        }
        {@type="application/json" |
            ```{"query":"{ viewer { id name } }","variables":{"id":"42"}}```
        }
    }
}
````

<details>
<summary>formatter-coloring-pipeline-package-artifacts</summary>

- Source: [`examples/formatter-coloring-pipeline.package-artifacts.fixture.cem`](./examples/formatter-coloring-pipeline.package-artifacts.fixture.cem)
- Content type: `application/cem`
- Schema: `https://cem.dev/ns/cem-ml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema.unresolved_namespace`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-ml/v1/examples/formatter-coloring-pipeline.package-artifacts.fixture.cem,contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

````cem
@doc cem-ml 1
@ns showcase = "https://cem.dev/ns/showcase/1"
@default showcase

{cemt-output-pipeline-fixture
    @source="schema-packages/cem-ml/v1/package.cem"
    @formatter="acme.showcase.format-tree"
    @colorizer="acme.showcase.color-tree"
    @color-profile="classes" |
    {stage
        @name="source-ast"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @category="cem-fragment" |
```cem
{article |
    {text | Ready }
    {strong |
        {text | now}
    }
    {text | .}
}
```
    }

    {stage
        @name="formatted-cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @category="cem-tree" |
```cem
{cem-tree @content-type="application/cem" @schema="https://cem.dev/ns/cem-ml/1" @category="cem-tree" @mode="fragment" @canonical=true @formatter-profile="acme.showcase.format-tree" |
    {format-nodes |
        {format-marker @name="cem.format-tree" @formatter-role="formatter.boundary" @formatter-profile="acme.showcase.format-tree"}
        {format-decision @name="showcase" @value="formatted tree before writer" @formatter-role="formatter.showcase"}
    }
    {nodes |
        {article |
            {format-layout @kind="format-decision" @name="layout" @value="inline" @formatter-role="formatter.layout"}
            {text | Ready }
            {strong |
                {format-layout @kind="format-decision" @name="layout" @value="inline-emphasis" @formatter-role="formatter.inline-emphasis"}
                {text | now}
            }
            {text | .}
        }
    }
}
```
    }

    {stage
        @name="colored-cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @category="cem-tree" |
```cem
{cem-tree @content-type="application/cem" @schema="https://cem.dev/ns/cem-ml/1" @category="cem-tree" @mode="fragment" @canonical=true @formatter-profile="acme.showcase.format-tree" @colored=true @color-profile="classes" |
    {format-nodes |
        {format-marker @name="cem.format-tree" @formatter-role="formatter.boundary" @formatter-profile="acme.showcase.format-tree"}
        {format-decision @name="showcase" @value="formatted tree before writer" @formatter-role="formatter.showcase"}
    }
    {color-nodes |
        {color-marker @name="cem.color-tree" @color-profile="classes" @colorizer-role="colorizer.boundary"}
        {color-decision @name="showcase" @value="colored tree before writer" @colorizer-role="colorizer.showcase"}
        {color-decision @name="queued-edit" @value="queued edit replay before writer" @color-profile="classes" @colorizer-role="colorizer.queued-edit"}
    }
    {writer-boundaries |
        {writer-boundary @stage="after-color" @value="writer consumes colored CEM tree"}
    }
    {nodes |
        {article @color-role="syntax.name" |
            {format-layout @kind="format-decision" @name="layout" @value="inline" @formatter-role="formatter.layout"}
            {style @color-role="syntax.name" @color-profile="classes"}
            {writer-attribute @name="class" @value="cem-color cem-color-syntax-name" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
            {span @color-role="syntax.string" |
                {style @color-role="syntax.string" @color-profile="classes"}
                {writer-attribute @name="class" @value="cem-color cem-color-syntax-string" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
                {color-wrapper @name="span" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.text-wrapper"}
                {color-decision @name="wrapped-role" @value="syntax.string" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.wrapped-role"}
                {text | Ready }
            }
            {strong @color-role="syntax.keyword" |
                {format-layout @kind="format-decision" @name="layout" @value="inline-emphasis" @formatter-role="formatter.inline-emphasis"}
                {style @color-role="syntax.keyword" @color-profile="classes"}
                {writer-attribute @name="class" @value="cem-color cem-color-syntax-keyword" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
                {span @color-role="syntax.keyword" |
                    {style @color-role="syntax.keyword" @color-profile="classes"}
                    {writer-attribute @name="class" @value="cem-color cem-color-syntax-keyword" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
                    {color-wrapper @name="span" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.text-wrapper"}
                    {color-decision @name="wrapped-role" @value="syntax.keyword" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.wrapped-role"}
                    {text | now}
                }
            }
            {span @color-role="syntax.string" |
                {style @color-role="syntax.string" @color-profile="classes"}
                {writer-attribute @name="class" @value="cem-color cem-color-syntax-string" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
                {color-wrapper @name="span" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.text-wrapper"}
                {color-decision @name="wrapped-role" @value="syntax.string" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.wrapped-role"}
                {text | .}
            }
        }
    }
}
```
    }
}
````

<details>
<summary>invalid-unclosed-scope</summary>

- Source: [`examples/invalid-unclosed-scope.cem`](./examples/invalid-unclosed-scope.cem)
- Content type: `application/cem`
- Schema: `https://cem.dev/ns/cem-ml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.ast.unclosed_scope`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-ml/v1/examples/invalid-unclosed-scope.cem,contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1

{article @id="broken" |
    {h1 | Missing article close}
```

<details>
<summary>invalid-unsupported-handoffs</summary>

- Source: [`examples/invalid-unsupported-handoffs.cem`](./examples/invalid-unsupported-handoffs.cem)
- Content type: `application/cem`
- Schema: `https://cem.dev/ns/cem-ml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.handoff.unsupported_content_type`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-ml/v1/examples/invalid-unsupported-handoffs.cem,contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

````cem
@doc cem-ml 1
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{main @id="unsupported-handoffs" |
    {@type="application/vnd.storybook.csf+json" |
        ```{"default":{"title":"Atoms/Button"},"stories":{"Primary":{"args":{"label":"Save"}}}}```
    }
    {@type="application/vnd.example.future+json" |
        ```{"future":true}```
    }
}
````
