# Module maps and portable configurations, 2026-10-25

- [x] Add ordinary module-map v1/v2/v3 source/deployment fixtures with explicit
  identities, exact formatting/artifacts and original syntax repairs.
- [x] Verify actual mapped/relative dependencies, declared resource inventories,
  native asset manifests, independent hashes and read-grant refusals.
- [x] Add ordinary CLI command/run-config and Studio JSON/CEM source fixtures,
  supported codecs, original version repairs and explicit HTML output.
- [x] Verify source validation separately from native command grammar, graph/run
  preparation, execution and host permissions; keep unsupported operations visible.
- [x] Verify native CLI and shared worker configurations, independent roots,
  source/configuration edits and refusal before publication.
- [x] Verify deployed shared controls, native project import/export, independent
  copies and offline reloads while preserving drafts and catalog originals.

## Ordinary project contracts

| Project | Authored source resources | Configurations | Demonstrated behavior |
| --- | ---: | ---: | --- |
| [Module-map deployments](../../examples/projects/module-maps/README.md) | 31 | 17 | Native v1/v2/v3 paired deployment, relative paths, declared assets/edges and actual byte identities. |
| [Portable configurations](../../examples/projects/portable-config/README.md) | 17 | 11 | Literal command envelopes, actual native plans, Studio manifest codecs and portable source archives. |

The projects contain **48 authored source resources and 28 configurations**.
They are normal catalog projects under Schema authoring and Queries and
transformations. Existing shared editor, command, artifact-tree, structured data and
sandboxed preview controls are reused. No sample-specific UI JavaScript,
JavaScript resolver/parser or serialized AST execution boundary is added.

## Module maps, native identities and authority

Standalone module-map source validation currently checks JSON syntax, including
when an explicit v1/v2/v3 schema is selected. It does not enforce deployment
version, key or resource-kind semantics. Formatting has exact native fixtures;
Inspect exposes the generic imported CEM structure and original source ranges.
Generic JSON-family Parse remains omitted because of its source-identity and
embedded tokenizer-diagnostic gap.

Actual native graph preparation enforces paired deployment contracts. V1 copies
an exact bare JavaScript import; v2 adds an explicit CSS resource; v3 copies typed
JSON/XHTML assets and rewrites one declared bare module edge to its relative
deployed URL. Only declared resources are copied. There is no automatic directory
discovery, import traversal, npm export selection or JavaScript execution.

Source asset paths resolve relative to the map; target URLs resolve beside the
explicit HTML entry. Retargeting the v3 JSON artifact changes its output path,
HTML import map and native rewritten module edge. Editing its JSON bytes changes
the source/output hashes independently. Unchanged builds in independent roots
retain the same native aggregate hash.

All nine module deployment outputs are opaque byte artifacts: no output spans
or `.map` sidecars are invented. Their actual native `moduleAssetManifest`
contains specifier, source map document URI, source URI, target, destination,
content type, source/deployed byte lengths and SHA-256. `sourceMapUri` names the
deployment document, not character-level provenance. Manifest hash contract
versions 1/2 are independent of map schema versions 1/2/3; v3's rewritten module
retains both original and deployed hashes.

The CLI verifier appends host-only `--report-json` arguments to a temporary
report. Portable commands contain no such destination; shared worker results
retain the same native manifest. Host-relative/FQ URI differences are normalized
against the actual owned root, never accepted by filename guessing.

The separate input-root-scope `--module-map` option is an unversioned schema-alias
mechanism. Neither it nor a paired deployment map grants reads. Native tests
prove custom-URI deployment refuses without a resolver grant (exit 6), succeeds
with the explicit bounded read map, and does not publish on refusal. Studio
receives only declared owned resources; missing worker dependencies also refuse.

The three syntax failures repair to the exact valid buffers. Separate syntactically
valid bad maps are accepted by source syntax validation but refused during native
planning with exit 2 and their real deployment diagnostic codes. There is no
completed command result or partial publication on this path. Static HTML preview
removes all scripts, including import maps, and keeps its existing sandbox/CSP.
JavaScript fixtures export data only; they implement no Studio/component behavior.

## Portable controls and real source archives

Native command source validation checks the versioned envelope and literal argv
strings. Actual argv grammar is checked by the host's native command codec.
An unknown command can be source-valid but cannot execute. The shared command
inspector exposes decoded argv and actual normalized input resources.

Run configurations use the real `validate --config` loader/normalizer, not a
standalone registered source adapter. Redirecting the saved run config to the
alternate XML changes its actual validated URI without changing the independent
HTML build. Duplicate inputs refuse preparation with the original code and exit
2 before a removed input could be read. That policy fixture is not advertised as
an executable invalid configuration or fabricated engine result.

CLI command generic Parse/Inspect/format and run-config generic source operations
remain unsupported. Studio JSON/CEM validation, conversion and formatting work;
CEM structural Inspect is shown separately from unavailable generic typed Parse.
Command/project forward-version failures retain their native codes, owned URIs
and null coordinates; exact repairs restore the valid sources.

The native project blueprint contains seven real resources, their actual hashes,
relative reference paths and two configurations pointing to actual authored
command resources. JSON/CEM conversion preserves that public project model.
It is a control document, never an internal parser AST. Native CLI provides no
archive bundler; source archives belong to the existing shared host provider.

## Verification

The new authored native suite passes **six tests**, covering all 28 configurations,
exact formats/artifacts, all five original source repairs, module manifest fields
and hashes, exact copied-file inventories, repeated/independent keys, changed
targets/edges, missing grants, no-publication planning failures and portable
manifest/resource/argv integrity. It checks separate command envelope/grammar
and run-config preparation semantics. Rustfmt passes.

The Feature Tour worker cohort passes **50 tests** and covers all source
configurations and all four
graphs, strict copied-byte equality, native manifest versions/hashes/owned URIs,
missing dependencies, preparation refusals, independent roots and graph/data/map
changes. The portable control case checks the public native semantic preview,
redirected run inputs, original structured refusal code/exit, envelope versus
argv grammar and decoded project resource hashes.
All five new worker cases also pass against the final regenerated archives.

The native catalog verifier passes all 32 schema-package examples and the
inventory records **172 workflow entries**. Output verification passes **218
declared assets / 331 files**. Two clean builds match with aggregate SHA-256
`e35888451112dcb59aeac7c5935bb7d49c442bce816d008c9b2fbb5ceb46b480`.
The determinism target now includes generated Feature Tour sources in its cache
inputs. Changing an example archive or its catalog index can no longer reuse a
report based only on unchanged verification summaries.
Focused ESLint and JavaScript syntax checks pass for the authored worker and
acceptance changes.

Deployed `verify:module-configs` acceptance passes **44 checks**: 22 per project,
using separate isolated browser contexts and collecting errors from both pages.
All native source operations and repairs, three module deployments, real asset
identities, semantic refusal codes, active last-good previews, saved map/data
edits and compact structured JSON rendering pass. Native manifest imports,
independent namespaces, authored command/run-config edits, deterministic source
archives, duplicate-import refusal and guarded replacement pass. Both projects
rebuild offline after reloading, retain saved controls and preserve the catalog
originals. The final deployed file inventory and digest match the fresh clean-build
report exactly.

## Preserved policies and next work

Acceptance uses isolated storage and temporary projects. User projects and drafts
are not cleared. Catalog originals remain read-only. Generated screenshots,
Vitest attachments, reports and binaries stay outside Git.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. Existing source-size,
UTF-8, ownership, theme, preview and resolver bounds are unchanged.

Continue with native DOM/AST/event projection projects, catalog-wide README
completion and the remaining full catalog acceptance checklist.
