# Version-specific module-map lifecycle fixtures — 2026-10-28

## Completed work

- [x] Replace the shared legacy success buffer with each version's canonical
  schema-package `examples/source.module-map.json` fixture and assert retained
  native JSON AST identity and original bytes.
- [x] Add selected/embedded version, missing/unsupported schema, MIME-only v1,
  generic JSON isolation and version-specific grammar refusal fixtures with
  native diagnostic locations and source maps.
- [x] Verify default/no-default-feature native lifecycle/module-map paths, CLI,
  existing public WASM/Studio consumers and isolated credential-policy recovery.

The original `builtins_load_versioned_module_maps_through_json_syntax_adapter`
expected empty diagnostics for one declaration-free string-import buffer across
v1/v2/v3. Its unchanged v1 failure was reproduced before editing. The current
native contract requires an embedded `$schema` matching the selected identity;
v3 also requires typed import entries. The fixture now reuses the package-owned
examples, exercising v1 string imports, v2 deployment resources, and v3 typed
imports/declared module edges/XHTML resources without reading deployment assets.

Five lifecycle tests cover 28 fixture cases: eight admitted syntax/data loads
and twenty semantic refusals. All six selected/embedded version mismatches,
missing and unsupported declarations, v1/v2 typed-import rejection, v1 resource
rejection, v2 XHTML rejection, v3 string-import rejection and missing v3 edge
targets retain their actual native diagnostics. Refusal checks assert severity,
field path, lifecycle adapter identity, URI, byte offset, line/column and bounded
original source-map frames. Semantic refusal retains the JSON syntax AST for
inspection. The MIME-only contract still selects v1. Generic JSON stays inert,
including a v3-named document with a version-invalid string-import entry.

All Rust edits are inside `#[cfg(test)] mod tests`; the production lifecycle
prefix is byte-for-byte identical to the committed source. No parser, validator,
import/export, command execution, native/WASM ABI, component or Studio behavior
changed. No assertion, semantic check, credential refusal, timeout, witness limit
or checkpoint budget was weakened. Earlier historical fixture-failure evidence
remains in its archives; this item closes that specific baseline failure.

## Verification

- Original isolated fixture: one test failed before repair, at v1.
- Native module-map cohort: 33/33 pass with default features.
- Complete lifecycle cohort: 103/103 pass.
- Native module-map cohort with `--no-default-features`: 33/33 pass.
- Native CLI deployment v1/v2/v3 checks: 3/3 pass, including exact declared
  JavaScript copying, explicit text/binary resources and declared-edge rewrites.
- Existing public Node/browser-worker WASM module-map/AST contract: 2/2 pass.
- Studio authored module-map configuration/repair and artifact/independent-root
  browser checks: 2/2 pass.
- Public WASM distribution verification checks the browser/Node ABI, all 658
  integrity records and 32 schema packages. Test-only Rust changes require no
  regenerated WASM or deployment assets; the frozen public production outputs
  are unchanged.
- Focused Rust additions were formatted separately without reformatting the
  large lifecycle module; `git diff --check` passes. Existing generated
  tree-sitter C compiler warnings remain unrelated.

- Final isolated complete Login recovery: 2/2 worker/fallback cases pass,
  with three recoveries and twelve edits per runtime and two credential
  refusals per runtime. Entered passwords/one-time codes block recovery until
  explicitly cleared; automatic recovery remains disabled. The 4 MiB group
  budget, 32,768-unit witness limit and all deadlines are unchanged.

Generated screenshots and Vitest attachments remain excluded from Git. The
formatter artifact-reader freshness trace and constructed XML document/provenance
URI separation remain separate TODO items.
