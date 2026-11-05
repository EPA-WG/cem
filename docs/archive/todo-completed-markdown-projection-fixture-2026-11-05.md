# Markdown native projection fixture — 2026-11-05

- [x] Audit the Markdown events-projection failure against its native contract.
- [x] Replace the obsolete JSON field search with complete typed projection and
  native writer-provenance assertions, retaining admission/refusal/color checks.
- [x] Verify the full native validation cohort and Markdown feature-profile gate.

## Finding

The unchanged fixture reproduces its failure at the old `byteRange` JSON
substring assertion. The earlier native validation run was 342/343, and a clean
`b86b4033` checkout independently reproduced the same failure. The current
`f5c33ce2` checkout reproduces it again before this correction.

Markdown Parse already projects retained native events through the CEM AST and
event vocabularies. Each projected node/event carries `source-id`, `byte-offset`
and `byte-length`; the published artifact also retains a hash-bound native
writer witness with original `SourceBoundary` frames. The programmatic JSON
compatibility member represents that typed vocabulary rather than the old
per-event JSON `byteRange` shape. No production provenance repair is required
for the exercised inputs.

## Fixture correction

The assertion now parses the public CEM-ML presentation artifact with the native
CEM parser. It checks the artifact content type/schema, projection root, original
Markdown schema/content type and source URI. Only blank formatter trivia is
excluded when counting published event elements; meaningful unexpected content
still fails. Both AST and Events must publish every retained native event once,
in order, with matching kind, tag, text, source URI and exact byte offset/length.
Projection attributes must remain literal values with no expression nodes or
duplicate fields. Events retain their open/close/value mapping.

For every native event, the artifact's original writer witness must admit an
exact matching source URI and byte span against both original input bytes and
published output bytes. UTF-8 boundaries are checked on the original buffer.
This verifies original native frames directly without using JSON as an AST
handoff or inferring provenance from serialized source fields.

The fixture now includes CRLF, nested strong markup, multibyte `Café 🧬` text and
literal `{#never} {$never()}` text. An independent source-byte lookup pins the
literal event's native offset and UTF-8 byte length before projection checks.
The package's authenticated original manifest/schema/formatter/colorizer
admission, missing-source refusal branch, unsafe embedded-HTML refusal for both
projections, no CEM fallback diagnostics, colored output and shared native owner
identity assertions remain intact.

## Verification

- Before correction: the unchanged focused fixture fails at the legacy JSON
  `byteRange` search (8.38 seconds of test work).
- After correction: the focused complete admission/Parse/refusal/color case
  passes (8.73 seconds), including exact native ranges in both projections.
- The complete native validation cohort passes **343/343** (70.81 seconds),
  closing the earlier 342/343 result without changing production code.
- All **16/16** Markdown cases pass with `--no-default-features` (10.92
  seconds), including owned contracts, source metadata and the complete pipeline.
- Focused rustfmt and `git diff --check` pass.
- Frozen presentation SHA-256 remains
  `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Commands from the repository root, Linux x64 / Rust 1.96.0 / Node 24.16.0,
with `NX_DAEMON=false` and prepared dependencies:

```sh
yarn nx run cem_ml:test --lib --args='validation::markdown::parser_contract::tests::admitted_markdown_originals_drive_native_parse_validation_and_color_pipeline' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem_ml:test --lib --args='validation::' --excludeTaskDependencies --skip-nx-cache
cargo test --locked -p cem-ml --no-default-features --target-dir dist/target/cem_ml_stripped --lib validation::markdown::
rustfmt --edition 2021 --check packages/cem_ml/src/validation/markdown/parser_contract/tests.rs
```

Only the cfg(test) fixture and task/evidence documentation change. Native parser,
projection, provenance and writer implementations, programmatic compatibility
results, command-service interfaces, budgets and public runtimes are unchanged.
No new WASM build, Studio build, deployment or release-graph claim is required by
this fixture correction. The preceding ARIA repair's rebuilt runtime verification
remains recorded in its separate archive.
