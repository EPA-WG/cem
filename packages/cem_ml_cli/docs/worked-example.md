# `cem-ml-cli` Worked Example: `login.cem` Round Trip

This walkthrough drives the canonical `login.cem` fixture through every
Tier A layer and shows the rendered light-DOM HTML output. It is meant as
a smoke check for new contributors and as the reference for the
`cem-ml validate` / `cem-ml parse` / `cem-ml convert` flows.

## Fixture

`examples/cem-ml/login.cem`:

```cem
@doc cem-ml 1
@ns cem = "https://cem.dev/ns/core/1"
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{main @cem:screen="login" @aria-labelledby="login-title" |
  {h1 @id="login-title" | Sign in}
  {form @cem:form="sign-in" @method=post @action="/session" |
    {label @for=email | Email}
    {input @id=email @name=email @type=email @autocomplete=email @required}

    {label @for=password | Password}
    {input @id=password @name=password @type=password @autocomplete=current-password @required}

    {button @type=submit @cem:action=primary | Sign in}
  }
}
```

## Library Round Trip

The `cem-ml` library exposes a one-call helper that drives every layer:

```rust
use cem_ml::interpreter::light_dom::render_html;

let input = std::fs::read_to_string("examples/cem-ml/login.cem")?;
let output = render_html(&input);
println!("{}", output.rendered);
```

The pipeline executed under the hood:

1. `cem_ml::source::BytesSource` yields chunked bytes.
2. `cem_ml::source::decode::Utf8Decoder` validates UTF-8 and emits decoded
   scalars with absolute byte ranges.
3. `cem_ml::tokenizer::cem::CemTokenizer` produces `SchemaToken`s with
   source-map stacks rooted in `TransformKind::CemTokenizer`.
4. `cem_ml::events::cem::CemEventNormalizer` lowers the tokens into the
   shared `NormalizedEvent` stream.
5. `cem_ml::schema::machine::CemSchemaMachine` validates the events
   against the active `CompiledSchema::cem_core()` (the schema authored
   at `packages/cem_ml/schema/cem-core.md`).
6. `cem_ml::parser::builder::CemAstBuilder` builds the `CemDocument`
   arena, populating `id_table` for reference resolution.
7. `cem_ml::validation::run` adds the Tier A semantic-rule catalog
   (reference integrity, accessible-name, state combinations,
   unsafe-content, JavaScript-URL detection).
8. `cem_ml::interpreter::light_dom::LightDomInterpreter` renders the
   light-DOM HTML output. Every emitted byte run is paired with a
   `SourceMapStack` traceable to the originating source bytes.

## Rendered Output

The snapshot captured at
`packages/cem_ml/tests/__snapshots__/login.html`:

```html
<main aria-labelledby="login-title" cem:screen="login"><h1 id="login-title">Sign in</h1><form action="/session" method="post" cem:form="sign-in"><label for="email">Email</label><input autocomplete="email" id="email" name="email" required type="email"><label for="password">Password</label><input autocomplete="current-password" id="password" name="password" required type="password"><button type="submit" cem:action="primary">Sign in</button></form></main>
```

The output is light-DOM HTML — no shadow DOM. The CEM annotations
(`cem:screen`, `cem:form`, `cem:action`) survive as attributes on the
host elements so an `@epa-wg/custom-element` consumer can attach behavior
to them.

## Source-Map Trace

Every `OutputSpan` in `TransformOutput.output_spans` walks back through
`TransformKind::InterpreterRender` → the AST node's
`TransformKind::CemAstBuilder` frame → the tokenizer's
`TransformKind::CemTokenizer` frame → the originating byte range in
`login.cem`. The integration test
`packages/cem_ml/tests/end_to_end.rs::every_output_span_traces_to_source_or_to_a_transform_frame`
exercises this for every canonical fixture.

## Graph Sidecar Sample

The transform-graph sample at
`examples/cem-ml/transform-graph/source-map-sidecar/` writes an HTML
export, an extracted CSS export, and `.map` sidecars with `outputSpans`.

```bash
dist/target/debug/cem-ml transform \
  --config examples/cem-ml/transform-graph/source-map-sidecar/graph.cem \
  --report-json /tmp/cem-ml-source-map-sidecar/report.json \
  --source-map-summary

node -e "const fs = require('node:fs'); for (const f of ['page.html.map','page.css.map']) { const m = JSON.parse(fs.readFileSync('/tmp/cem-ml-source-map-sidecar/' + f, 'utf8')); console.log(f + ': ' + m.outputSpans.length + ' output spans'); }"
```

The HTML sidecar maps copied output ranges back to the source; generated
bytes such as the inserted stylesheet link are intentionally unmapped.
The CSS sidecar maps the extracted stylesheet content after rebasing it
from the inline `<style>` block into the standalone CSS output.

## CEMT Formatter And Colorizer Run Config

The schema-package CEMT output example uses a checked run-config file plus the
CEM-native stage fixture command. The stage fixture shows the formatter output
as a formatted CEM tree and the colorizer output as a colored CEM tree. The
run-config then selects the same schema-package formatter/colorizer functions
and writes the final HTML writer output.

```bash
cem-ml fixture cemt-pipeline \
  --package-artifacts \
  --out packages/cem_ml_cli/dist/cemt-output-pipeline.package-artifacts.fixture.cem

cem-ml convert \
  --config packages/cem_ml_cli/docs/examples/cemt-output-pipeline.run.json \
  --artifact-json packages/cem_ml_cli/dist/cemt-output-pipeline.artifact.json
```

The config at
[`docs/examples/cemt-output-pipeline.run.json`](examples/cemt-output-pipeline.run.json)
maps `cem+repo://` to the repository root and selects the embedded
`cem-ml/v1` schema-package CEMT assets:

- `cemtFormatter`: `acme.showcase.format-tree`
- `cemtFormatterProfile`: `acme.showcase.format-tree`
- `cemtColorizer`: `acme.showcase.color-tree`
- `cemtColorProfile`: `classes`

The built-in CEM-ML package is already embedded in the CLI registry, so the
example does not load it again through `schemaPackages`. A project-local
schema package uses the same output `rootScope` selectors and adds its own
`schemaPackages` entry for the package manifest URI.

The generated fixture remains CEM-native:

```cem
{cem-tree @content-type="application/cem" @schema="https://cem.dev/ns/cem-ml/1" @category="cem-tree" @mode="fragment" @canonical=true @formatter-profile="acme.showcase.format-tree" |
```

The generated writer output at
`packages/cem_ml_cli/dist/cemt-output-pipeline.html` is target-native HTML.
When `--artifact-json` is supplied, the structured convert artifact is written
separately to `packages/cem_ml_cli/dist/cemt-output-pipeline.artifact.json` for
debugging; its `content` field mirrors the HTML emitted by the writer. The
`cem_ml_cli:validate-cemt-pipeline-fixture` target compares the generated HTML
bytes with
[`docs/examples/cemt-output-pipeline.expected.json`](examples/cemt-output-pipeline.expected.json).
The same validation target also runs checked-in `convert --config` examples for
CEM-native and YAML outputs and compares their generated bytes with
`docs/examples/native-output-cem.expected.json` and
`docs/examples/native-output-yaml.expected.json`.

Multi-output config follows the same native-output rule: each output writes
primary target-native bytes to its own `dest` / `destination`. A global `--out`
is rejected for multiple configured outputs, and `--artifact-json` remains a
single-output debug JSON sidecar. The checked-in error specs
`docs/examples/multi-output-global-out.error.expected.json`,
`docs/examples/multi-output-artifact-json.error.expected.json`, and
`docs/examples/multi-output-missing-destination.error.expected.json` lock those
messages into the same `cem_ml_cli:validate-cemt-pipeline-fixture` target.

```html
<article class="cem-color cem-color-syntax-name"><span class="cem-color cem-color-syntax-string">Ready </span><strong class="cem-color cem-color-syntax-keyword"><span class="cem-color cem-color-syntax-keyword">now</span></strong><span class="cem-color cem-color-syntax-string">.</span></article>
```

## CLI Commands

The CLI surface is wired to the parser-enabled Rust engine for the current
Tier A flows:

```bash
# Inspect the parsed AST as JSON.
cem-ml parse examples/cem-ml/login.cem --format dom-json

# Validate and emit a Markdown report.
cem-ml validate examples/cem-ml/login.cem --report-md packages/cem_ml_cli/dist

# Render the light-DOM HTML to stdout.
cem-ml convert examples/cem-ml/login.cem --to-format dom-json

# Run the canonical fixture-validation set (zero hard violations expected).
cem-ml fixture validate
```

The library helper above remains the smallest in-process round-trip path.
CLI feature and integration tests exercise the same parser, validation,
conversion, transform, report, and sidecar boundaries through
`packages/cem_ml_cli/src/dispatch.rs` and `packages/cem_ml_cli/tests/`.

## Re-Running the Snapshot

```bash
CEM_ML_UPDATE_SNAPSHOTS=1 cargo test -p cem-ml --test transform_snapshots
```

The default `cargo test -p cem-ml` run compares the rendered output
byte-for-byte against `packages/cem_ml/tests/__snapshots__/*.html` and
fails if any fixture's HTML changes.
