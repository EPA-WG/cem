# Migration and deprecation report

This report is the release-facing index for active compatibility work. Detailed
engine rules live in `content-type-switch.md`; the custom-element bridge detail
lives in `custom-element-bridge-template-policy.md`; generated token omissions
and deprecated rows live in the `@epa-wg/cem-theme` token report.

## Publication baseline

The 2026-08-22 Phase 9 audit found these public npm versions:

| Package | Registry state | Source state |
| --- | --- | --- |
| `@epa-wg/cem-theme` | `0.0.14` | fixed web family |
| `@epa-wg/cem-components` | `0.0.14` | fixed web family |
| `@epa-wg/cem-elements` | not yet published | fixed web family |
| `@epa-wg/custom-element` | `0.0.39` | fixed web family |
| `@epa-wg/cem-ml` | not yet published | fixed CEM-ML family |
| `@epa-wg/cem-ml-cli` | not yet published | fixed CEM-ML family |
| `@epa-wg/cem-studio` | not yet published | fixed CEM-ML family |

The immutable `cem-ml-v0.1.0-rc.2` GitHub prerelease proves the three-unit
CEM-ML pipeline, but it is rehearsal evidence rather than a final npm/Studio
publication claim. Phase 9 remains open until the checked-in public evidence
replaces this baseline with final registry and release coordinates.

## Active migrations

### Package exports

Replace deep `dist/` imports with declared exports. Supported examples include:

- `@epa-wg/cem-theme/styles.css`;
- `@epa-wg/cem-theme/tokens/cem.tokens.json`;
- `@epa-wg/cem-theme/tokens/cem.tokens.catalog.json`;
- `@epa-wg/cem-components/catalog/cem.components.catalog.json`;
- `@epa-wg/cem-ml/wasm`, `@epa-wg/cem-ml/runtime.json`, and
  `@epa-wg/cem-ml/integrity.json`;
- `@epa-wg/cem-ml-cli/node` or `@epa-wg/cem-ml-cli/browser`; and
- the documented `@epa-wg/cem-studio` subpaths.

The token intermediate/resolved JSON files and any undeclared generated path are
debug-only. Consumers must not depend on them.

### Custom-element templates

New declarations use canonical `type="text/cem-ml"`. The explicit
`custom-element-v0` selector and `cem-ml; version=0.0` form remain functional
bridge inputs, are deprecated since `0.1.0`, and are scanned by FF-5. Their
registry currently sets the earliest removal to major 2. The browser-native
`XSLTProcessor` engine is already forbidden; supported legacy HTML/XSLT is
converted into CEM-ML/CEM-QL through the bounded compatibility adapter.

Migration procedure:

1. run the existing compatibility inventory and converter on the legacy sample;
2. replace the bridge selector with canonical CEM-ML declaration syntax;
3. move unsupported Tier 3 XSLT logic into CEM-ML/CEM-QL;
4. compare the migrated fixture through the package/browser parity gates; and
5. remove the allowlisted bridge use only after its inventory reaches zero.

### Deprecated layout tokens

`--cem-layout-inline-tight`, `--cem-layout-inline`, and
`--cem-layout-inline-loose` remain deprecated manifest rows. Use the canonical
space/stack/layout tokens described by the dimension specification. The export
pipeline reports these rows as expected deprecated omissions rather than
silently inventing values.

### Native compatibility copies

The Swift Package and Android library are the installable contracts. Root
`CEMTokens.swift`, Android `values`/`values-night`, and `compose` directories are
compatibility copy paths for existing adopters. Consumers should migrate to the
package/module layouts; removal of a copy path requires a breaking theme release
and a published migration window.

## Breaking-change checklist

Every proposed breaking change must update all applicable items before release:

- authoritative specification or schema descriptor and version;
- public package export map and clean-consumer fixture;
- deprecation registry/report, replacement, and earliest removal version;
- migration guide with before/after usage;
- token, component, native, CLI, Studio, docs/example, and source-map tests;
- changelog entry and family version decision; and
- Phase 9 publication evidence for the exact released bytes.

No active deprecation authorizes removal by itself. The Phase 9 verifier checks
that every registered deprecation has policy and migration ownership, while FF-5
continues to enforce the source inventory and removal window.
