# Native conversion fixture intent — 2026-11-05

- [x] Repair the cross-surface `.busy` regression baseline while preserving the
  established native attribute-value semantics and exact authored input.
- [x] Strengthen the fixture to require the owning expression, original body
  source span and braced event value; preserve quoted CEM/XML/HTML literal intent
  through canonical round trips.
- [x] Reconcile the conversion rules with the native-slot contract and verify
  default and `--no-default-features` native integration cohorts.

## Cause and correction

The original five-case `cross_surface_conversion_rules` suite reproduced
**4 passes / 1 failure** in each native feature profile. Its CEM fixture contains
`@disabled={.busy}`. The failing assertion searched for a literal attribute
compatibility `value` equal to `{.busy}`.

The established [native attribute contract](../cem-ml-syntax.md#native-attribute-expression-slots)
distinguishes an unquoted native expression from a quoted string. The parser
already retains this attribute with an empty literal compatibility value and one
owning `value_nodes` edge to a native `$` expression with body `.busy`. That
behavior was introduced in `ce890ba9` and remains covered by native attribute and
expression-lexing tests. This was a stale literal-value assertion, not a lost
expression or a parser repair.

The [cross-surface fixture](../../packages/cem_ml/tests/cross_surface_conversion_rules.rs)
now locates the exact disabled attribute on its owning button and requires:

- No literal compatibility value, exactly one owning native expression, an exact
  `$` node with one text body `.busy`, and no promotion into element content.
- An `ExpressionEmbedding` frame with the original source ID and a byte range
  selecting exactly `.busy` from the authored input.
- The adjacent disabled-name/expression-value normalized events, retaining the
  exact `{.busy}` lexeme and original braced source range.
- The existing mixed `Hello {.name}` value remaining literal on the exact
  `aria-label` attribute.

An added quoted-CEM case preserves `{.busy}` as a literal with no native value
nodes. Existing unmarked XML/HTML fixtures require the same literal boundary.
All three canonical conversion paths now reparse and check the appropriate typed
intent as well as the existing byte stability and tokenizer/content-type source
frames. Comments, rich JSON, namespace rebinding, anonymous scopes and standalone
content-expression checks remain in place. No example bytes were rewritten or
quoted to make the original CEM assertion pass.

The [conversion rules](../../packages/cem_ml/docs/cross-surface-conversion.md#8-attribute-cem-ql-spans)
now distinguish native slots, quoted literals and explicit template-aware AVT
handling, with a link to XML's element-local `expression-attributes` import
contract. The parser, runtime, formatter, codec and native ownership rules are
unchanged. No JavaScript workaround, application/UI change or WASM publication
is involved.

## Verification and separate baseline

Linux x64, with `CARGO_BUILD_JOBS=2` for the related integration cohorts:

| Integration suite | Default | No default features |
| ----------------- | ------- | ------------------- |
| `cross_surface_conversion_rules` | 6/6 pass | 6/6 pass |
| `attribute_expression_lexing` | 4/4 pass | 4/4 pass |
| `cross_surface_projection` | 4/4 pass | 4/4 pass |
| `fixture_pair` | 5/5 pass | 5/5 pass |
| `namespace_rebinding_fixtures` | 3/3 pass | 3/3 pass |
| `native_attribute_values` | 4/5 pass | 4/5 pass |
| `reference_syntax_adoption` | 8/8 pass | 8/8 pass |
| `reverse_conversion` | 2/2 pass | 2/2 pass |

Each complete eight-suite invocation is **36/37**, not a green complete native
release. The sole adjacent failure is the unchanged
`version_three_literal_attributes_decode_with_empty_native_slots` fixture, which
rewrites a current **version-5** binary header to **version 3**, recomputes the
hash and receives `UnexpectedEof` from decode. Its body still contains the
current version-5 source-metadata section. The fixture and
production codec were not modified here. An active follow-up requires genuine
version-3 compatibility evidence while preserving strict decoder refusals; this
change does not claim to repair legacy binary compatibility.

The initial broader default invocation stopped at that failure. Complete default
and no-default-feature invocations used `--no-fail-fast` to run all eight suites;
none was filtered or skipped. Existing tree-sitter C compiler unused-label
warnings remain. The changed Rust fixture passes `rustfmt --check`, and the
frozen presentation archive retains its exact size and SHA-256.

```sh
cargo test -p cem-ml --test cross_surface_conversion_rules
cargo test -p cem-ml --no-default-features --test cross_surface_conversion_rules
cargo test -p cem-ml --no-fail-fast --test cross_surface_conversion_rules --test native_attribute_values --test attribute_expression_lexing --test reference_syntax_adoption --test reverse_conversion --test cross_surface_projection --test fixture_pair --test namespace_rebinding_fixtures
cargo test -p cem-ml --no-default-features --no-fail-fast --test cross_surface_conversion_rules --test native_attribute_values --test attribute_expression_lexing --test reference_syntax_adoption --test reverse_conversion --test cross_surface_projection --test fixture_pair --test namespace_rebinding_fixtures
rustfmt --edition 2021 --check packages/cem_ml/tests/cross_surface_conversion_rules.rs
```

The first two commands reproduced the original baseline before the repair; the
default focused run and both complete cohorts verified the corrected fixture.
The separate stripped-WASM size gate and historical runtime deadlines remain
open; native test feature profiles are not WASM size verification.
