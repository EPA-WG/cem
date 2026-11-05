# Studio strict schema-definition audit — 2026-11-05

- [x] Attribute the five existing Studio project schema-definition errors.
- [x] Correct the authored definitions without changing the portable codec.
- [x] Add native compilation, stable-ID and resource/reference-path regressions;
  preserve native CEM/JSON rejection and deterministic projection contracts.

## Findings and correction

A failing-first native compilation test reproduces exactly five
`cem.schema_definition.invalid_datatype_param` errors in the package schema:

| Attribute | Facet | Cause | Correction |
| --- | --- | --- | --- |
| `id` | `pattern` | A regex facet requires a string type; `schema:identifier` also rejects numeric-leading stable IDs accepted by Studio. | Use `schema:string` with the unchanged `^[a-z0-9][a-z0-9-]{0,63}$` regex. |
| `path` | `pathForbiddenPrefixes` token `/` | This facet requires a valid scoped path prefix. | Remove the redundant invalid prefix list. |
| `path` | `pathForbiddenPrefixes` token `../` | Parent traversal is not a valid scoped prefix. | Same. |
| `reference-path` | `pathForbiddenPrefixes` token `/` | Same invalid scoped-prefix declaration. | Same. |
| `reference-path` | `pathForbiddenPrefixes` token `../` | Same invalid scoped-prefix declaration. | Same. |

The native `cemml:path` type already rejects absolute paths, parent/dot segments,
empty segments, trailing separators and backslashes. Both attributes retain that
type and their `.cem-studio` forbidden-directory facet. No validator exception,
ignored diagnostic, widened facet grammar or portable schema-version migration
is introduced. The package README explains the authoring contract.

The native codec and JSON Schema artifact are unchanged. The typed codec remains
the authority for the complete portable project contract; generic structural
schema validation alone is not claimed equivalent to every native semantic
constraint. Existing resource-reference identity, rename behavior, hierarchy,
unknown-field, duplicate-ID, entry-artifact and projection checks remain intact.

## Native regressions

The test suite now requires diagnostic-free schema compilation and readiness.
Stable-ID cases exercise numeric-leading IDs, the 64-character boundary, empty,
leading-hyphen, uppercase, underscore, non-ASCII, whitespace and overlong values
through the authored schema and both native input projections. The unchanged
schema fails on valid `0` as well as the five definition errors. An empty CEM ID
retains `cem.studio_project.field_required`; an empty JSON ID retains
`cem.studio_project.id_invalid`. Other malformed IDs retain the latter code.

Both `path` and `reference-path` are tested against absolute paths, parent/dot
segments, doubled/trailing separators, backslashes and reserved directories at
root or nested locations. Assertions check the native schema type/directory
facet diagnostic plus exact native codec error code and field path in CEM and
JSON. The existing deterministic round trips and stable-reference tests also
remain in the gate.

## Verification

- Failing-first native schema compilation: the original five definition errors.
- First expanded native gate: **16/18**, with failures in schema compilation and
  the valid numeric-leading ID; path containment checks already pass.
- After correcting the three declarations: **18/18** pass with default features
  and **18/18** with `--no-default-features`.
- `cem_ml_schema_package_studio_project_v1:verify` passes, rebuilding the native
  CLI, validating the package manifest, running all 18 Studio native cases and
  the complete built-in package folder-contract audit.
- Fresh CLI probes use the schema's primary content type
  `application/vnd.cem.schema+cem` and schema `https://cem.dev/ns/schema/1`.
  The original source reports **five errors / one warning**; the corrected
  source reports **zero errors / the same one warning**. Both strict invocations
  exit 1 because strict mode also fails warnings. Neither is counted as a fully
  green strict CLI result.
- Focused rustfmt and `git diff --check` pass.

The remaining `cem.a11y.aria_incompatible` warning says namespace metadata
`role="schema"` is not an ARIA role. It appears identically on both original and
corrected source. Its source map points to the schema's namespace declaration;
removing that metadata or weakening ARIA checks is outside this correction.
The active checklist tracks a separate native-rule audit for that warning.

Commands from the repository root, Linux x64 / Rust 1.96.0 / Node 24.16.0:

```sh
yarn nx run cem_ml:test --test=studio_project --excludeTaskDependencies --skip-nx-cache
cargo test --locked -p cem-ml --no-default-features --target-dir dist/target/cem_ml_stripped --test studio_project
CARGO_TARGET_DIR=/home/suns/cem/dist/target/cem_ml yarn nx run cem_ml_schema_package_studio_project_v1:verify --skip-nx-cache
./dist/target/cem_ml_cli/debug/cem-ml --quiet validate --fail-level strict --format json --content-type application/vnd.cem.schema+cem --schema https://cem.dev/ns/schema/1 --report-json /tmp/cem-studio-schema-strict-typed packages/cem_ml/schema-packages/studio-project/v1/schema/studio-project.cem
rustfmt --edition 2021 --check packages/cem_ml/tests/studio_project.rs
```

`NX_DAEMON=false` is used for Nx. The temporary target-directory override reuses
prepared native test outputs; the CLI build retains its configured output path.

This changes package authoring, tests and documentation only. Production Rust
validation code, browser UI, themes/tokens, stored projects and drafts, schema
identity/version, JSON interchange format and budgets are unchanged. No public
runtime deployment or full Studio release-graph acceptance is claimed. The frozen
presentation archive is unchanged.

Follow-up: the remaining namespace-metadata ARIA warning is resolved by the
[native ARIA ownership correction](todo-completed-schema-metadata-aria-2026-11-05.md).
The before/after strict results above remain the historical schema-facet evidence.
