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

## CLI Commands (Once Parser-Enabled)

The CLI surface is wired today but the production engine returns
`EngineError::NotImplemented` until the parser-enabled milestone in
`cem-ml-cli-plan.md` Phase 11. The intended flow:

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

Until the engine boundary is wired to the real Rust pipeline, the
authoritative round-trip path is the library helper above. The feature
tests in `packages/cem_ml_cli/src/dispatch.rs` exercise the same boundary
via the feature-gated `FakeEngine`.

## Re-Running the Snapshot

```bash
CEM_ML_UPDATE_SNAPSHOTS=1 cargo test -p cem-ml --test transform_snapshots
```

The default `cargo test -p cem-ml` run compares the rendered output
byte-for-byte against `packages/cem_ml/tests/__snapshots__/*.html` and
fails if any fixture's HTML changes.
