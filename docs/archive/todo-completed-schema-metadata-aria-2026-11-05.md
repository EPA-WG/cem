# Schema metadata ARIA ownership — 2026-11-05

- [x] Distinguish metadata roles from DOM ARIA semantics in the native rule.
- [x] Add failing-first native cases for metadata, embedded DOM, namespace aliases,
  DOM namespace coverage and original capture ownership.
- [x] Verify rebuilt Node/browser WASM command-service regressions and package output.

## Cause and correction

`AriaCompatibilityRule` formerly inspected every element by local name and every
`role` value as ARIA. Studio's schema contains a namespace declaration with
`role="schema"`; that is vocabulary metadata, not an accessibility role.

The rule now reads the original source owner's captured expanded namespace name.
Metadata in a resolved non-DOM namespace keeps its own role/attribute semantics.
HTML, SVG and MathML namespace nodes retain the existing ARIA role and attribute
checks, including when embedded inside schema documents. Importer-expanded URI
names receive the same treatment. No document-wide schema exemption or lexical
`schema`/`html` prefix allowlist is introduced.

The captured document must be the exact AST allocation being validated. A capture
from another owner, missing/unbound names, and legacy lexical names do not grant
an exemption. Those paths retain the prior conservative checks. The rule reads
existing namespace metadata without reparsing, changing source ASTs, adding
state or performing resource reads. Role tables, ARIA requirements, diagnostic
code/severity and source-map reporting remain unchanged.

The native regressions cover schema and schema-package metadata plus a separate
data vocabulary; an actual DOM node named `namespace`; prefixes named `html`
and `schema` bound to opposite vocabularies; embedded invalid role/attribute
cases; valid checkbox roles; HTML/SVG/MathML; and mismatched capture ownership.
The unchanged rule fails three of the five cases. All five pass after correction.

Two command-service regressions load Studio's actual schema file in Node and
browser WASM bindings and require an empty diagnostic list. The same bindings
must still report the exact ARIA warning, source URI and source frames for an
invalid HTML `role="schema"`. Both fail against the original packaged runtime
with the same metadata warning before rebuilding.

## Verification

- Studio schema strict CLI validation now exits **0**, with zero info, warnings,
  errors, fatal diagnostics or hard violations. Its metadata remains intact.
- The HTML negative control exits **1** under the same strict policy, with exactly
  one `cem.a11y.aria_incompatible` warning for `role="schema"`.
- Studio package verification passes after rebuilding the native CLI: **18/18**
  portable project cases and the built-in package folder-contract audit pass.
- All five new native ARIA cases pass with default features and again with
  `--no-default-features`. Existing plain HTML role/attribute checks pass in the
  broader validation cohort.
- The rebuilt packaged runtime passes **30/30** runtime tests, including both new
  metadata/HTML controls and the unchanged Login diagnostic inventory; the
  complete native XSLT WASM contract check also passes. Browser bindings are
  initialized from bytes in the Node test harness, not a Chromium UI test.
- Package verification passes browser/Node ABI and generated types, **658**
  integrity records and **32** schema packages. Both optimized WASM modules are
  24,861,009 bytes with SHA-256
  `48f58e89e89f4b9b2fc4832f332150e960e8ef29c2448eb61874ed0892d39d33`.
  The existing Binaryen `-Oz` pass and size/interface checks are unchanged.
- The broader native validation cohort is **342/343**, with one Markdown events
  projection failure at `markdown/parser_contract/tests.rs:313`: the fixture
  expects a legacy JSON `byteRange` field while the projection contains typed
  `sourceMap` frames. A focused run from a clean, detached **b86b4033** checkout
  reproduces the same assertion failure without this ARIA change. This result
  remains a failure, not a green full-cohort claim. The active checklist tracks
  a separate source-range/projection audit.
- The changed Rust sections match rustfmt output; `git diff --check` passes.
  Frozen presentation SHA-256 remains
  `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Commands from the repository root, Linux x64 / Rust 1.96.0 / Node 24.16.0,
with `NX_DAEMON=false`:

```sh
yarn nx run cem_ml:test --lib --args='aria_compatibility_' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem_ml:test --lib --args='validation::' --excludeTaskDependencies --skip-nx-cache
cargo test --locked -p cem-ml --no-default-features --target-dir dist/target/cem_ml_stripped --lib aria_compatibility_
CARGO_TARGET_DIR=/home/suns/cem/dist/target/cem_ml yarn nx run cem_ml_schema_package_studio_project_v1:verify --skip-nx-cache
./dist/target/cem_ml_cli/debug/cem-ml --quiet validate --fail-level strict --format json --content-type application/vnd.cem.schema+cem --schema https://cem.dev/ns/schema/1 --report-json /tmp/cem-schema-aria-strict packages/cem_ml/schema-packages/studio-project/v1/schema/studio-project.cem
yarn nx run @epa-wg/cem-ml:build --skip-nx-cache
yarn nx run @epa-wg/cem-ml:test --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-ml:verify --excludeTaskDependencies --skip-nx-cache
```

The Markdown baseline was run in a temporary detached worktree at
`b86b40331aec702693db29de8617e66b9e63cebc`, using:

```sh
cargo test --locked -p cem-ml --target-dir /home/suns/cem/dist/target/cem_ml --lib validation::markdown::parser_contract::tests::admitted_markdown_originals_drive_native_parse_validation_and_color_pipeline -- --exact
```

The clean temporary worktree was removed after collecting the failure.

The native change is shared validation logic, with no Studio/component JavaScript
behavior or CSS/token changes. Portable project parsing, storage, source schema
metadata, role tables, severity policy and budgets remain unchanged. Packaged
runtime outputs are rebuilt locally; no public deployment or full Studio release
graph is claimed. Frozen presentation bytes remain unchanged.
