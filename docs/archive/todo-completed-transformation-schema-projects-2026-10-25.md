# Transformation and schema-authoring projects, 2026-10-25

- [x] Add ordinary CEM and XSLT transformation projects with explicit source and
  template identities, multi-step graphs, native artifacts and original repairs.
- [x] Add ordinary CEM schema/package, JSON Schema and dual-syntax RELAX NG
  authoring projects with explicit supported operations and diagnostic repairs.
- [x] Keep schema-source validation separate from native CEM instance validation
  and generic JSON/XML syntax checks. Preserve reference and resolver boundaries.
- [x] Verify all authored configurations, exact native format/artifact fixtures,
  provenance, independent edits and refusal before failed-graph publication.
- [x] Verify shared worker execution, graph parameter/data/template changes,
  refreshed owned schemas, missing dependency refusal and independent copies.
- [x] Add failing shared source-admission cases before enabling owned XSLT, JSON
  Schema and RELAX NG text, retaining byte/size/source-kind and binary guards.
- [x] Verify deployed shared controls, actual previews, saved independent copies,
  offline reloads and unchanged catalog originals without clearing user storage.

## Ordinary project contracts

| Project | Authored resources | Configurations | Demonstrated behavior |
| --- | ---: | ---: | --- |
| [CEM transformation](../../examples/projects/cem-transform/README.md) | 10 | 7 | XML orders, declared parameters, native report and HTML stages, JSON table artifact. |
| [XSLT transformation](../../examples/projects/xslt-transform/README.md) | 11 | 8 | The same XML orders, direct XSLT HTML and a native XSLT report followed by a CEM HTML stage. |
| [CEM schema](../../examples/projects/cem-schema/README.md) | 7 | 7 | Native schema/package source authoring and separate valid/invalid note instances. |
| [JSON Schema](../../examples/projects/json-schema/README.md) | 5 | 4 | Draft 2020-12 source validation/formatting, local declarations and unsupported-dialect repair. |
| [RELAX NG](../../examples/projects/relax-ng/README.md) | 9 | 10 | XML/compact typed Parse, source validation/formatting and start/delimiter/reference-policy repairs. |

These are five ordinary catalog projects, containing **42 authored source
resources and 36 configurations**. They use the existing editor, command, graph,
table and media preview controls. No example-specific UI JavaScript, parser, codec or serialized
AST execution boundary is introduced.

Studio's host resource policy now admits the canonical XSLT, JSON Schema and both
RELAX NG text MIME types, including normalized case and parameters. The existing
shared editor and declarative controls implement editing and interaction.
Compiled `+cem-bin` selectors remain read-only even when their bytes are valid
UTF-8. Editing permission does not grant an executable parser or resolver access.

Both transformation graphs explicitly select `generated/index.html`. Their
native HTML fixtures are identical, including the native writer's layout spaces:
165 UTF-8 bytes. Direct XSLT HTML has its own 150-byte fixture. Narrow Git
whitespace attributes preserve the graph HTML and JSON Schema formatter output
fixtures, and the original malformed XML failure bytes.

The CEM graph also publishes `generated/report.json` as an application data
artifact. Its constructed generic-data values export without output spans or a
sidecar map; this absence is asserted. CLI serialization is indented and worker
serialization compact, so cross-host comparisons check the complete JSON value.
Repeated output within a host remains byte-stable. HTML output has native spans
identifying its actual page template; arbitrary input character lineage is not
invented for computed values.

The XSLT report is an ordinary native report/item tree, consumed by CEM for the
final HTML. Its graph advertises only HTML. The current XPath/XSLT input adapter
requires imported native nodes; a constructed CEM-tree result cannot be silently
treated as a new retained XML owner. Encoding XML produces text, not that owner.
The examples use the supported stage order and keep native handoff/reimport as
an explicit follow-up.

XSLT execution is covered in the complete profile. A probe of the existing
optional-body WASM deployment, built October 24 at 21:36 PDT, still reports
`externalBodies=true` with XSLT external and refuses contextless bundle compilation
with `cem.schema_asset.missing`, preserving the original source URI. That artifact
was not rebuilt for this work; optional executable graphs remain unsupported
until the original parser-contract handoff is implemented.

## Schema source and instance boundaries

The CEM package owns a local schema source and explicitly claims its namespace.
The schema declares note elements/attributes, a required-id field contract and
its diagnostic. Validating the schema/package source is distinct from loading
the package to validate an instance. Saved schema-message edits change the
instance diagnostic while preserving its native code, severity and field-contract
detail. Missing schema or package inputs refuse validation.

JSON Schema source validation admits the supported Draft 2020-12 dialect; local
`$defs`/`$ref` declarations do not imply reference traversal or instance assertions.
Generic JSON syntax validation deliberately accepts a domain-invalid object.
JSON-family Parse/Inspect can embed CEM tokenizer errors despite exiting zero,
including this schema's structural inspection. Those operations are omitted and
the existing native diagnostic/identity follow-up is extended.

RELAX NG XML and compact Parse expose the typed source identity and original
ranges; compact validation remains lexical. Formatting preserves source lexemes.
The projects demonstrate local grammar declarations and hard rejection of
external references, without an instance-validator claim. Generic XML syntax
validation deliberately accepts an unrelated element. Typed source Parse is
distinguished from generic imported-CEM structural Inspect.

## Verification

The new authored native suite passes **five tests**, covering all 36 configurations,
exact source/artifact bytes, original mapped owners, all eight diagnostic repairs,
independent data/template/minimum edits and failed source/template builds without
partial publication. Schema cases cover changed owned contracts, separate
instances, missing schema/package refusal, source-only JSON Schema/RELAX NG
semantics and rejected external references in both RELAX NG syntaxes. Rustfmt and
focused authored JavaScript lint pass.

The shared source-admission cohort passes **29 browser cases**. Fifteen cases
failed before the policy correction: eight text-editing cases and seven compiled
binary selectors. Final cases verify native commands, save/reopen, exact stored
bytes, revisions, UTF-8/size/source-kind bounds and download-only behavior. All
**10 MathML compatibility cases** also pass.

Studio typechecking and focused lint for all changed authored JavaScript pass.
Typechecking uses the exact native-generated CLI schema input, removed again
after verification. The existing standalone preparation and generated-WASM lint
scope follow-ups remain separate.

The Feature Tour worker suite passes **45 tests**, including every new command,
all graphs, declared minimum changes, data and template changes, native output
spans, original diagnostic coordinates and repairs, missing dependencies and
independent project identities. It checks that JSON/XML syntax-only configurations
do not apply the authored application schemas.

All five regenerated archives pass native Studio-project validation. Source Parse
and Inspect configurations use the public `inspection` kind, with authored argv
selecting the actual operation. The corrected final archives also pass the full
45-test worker cohort.

Native Feature Tour verification passes all 32 schema-package examples and the
authored project configurations. The checked workflow inventory has 144 entries.
Output verification passes 218 declared assets and 329 files; deterministic builds
match SHA-256
`63f32058e219863aab91144ae0b439383990192a908c8223e7f8b99e675e894e`.

The final deployed `verify:transformations-schemas` gate passes all **75 evidence
flags**: 17 CEM transformation, 16 XSLT transformation, 14 CEM schema, 13 JSON
Schema and 15 RELAX NG. Actual sandboxed HTML and compact JSON table previews,
source/graph/schema edits, draft navigation guards, failed-build retention,
independent copies and offline native reruns pass. All original catalog sources
remain read-only and unchanged; no unexpected page errors are reported.

## Preserved policies and next work

Acceptance uses isolated browser storage and temporary project copies. User
projects and drafts are not cleared. Catalog originals remain read-only; generated
screenshots, Vitest attachments, reports and binaries remain outside Git.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. Theme tokens,
declaration authority, resolver grants and runtime admission limits are unchanged.

Continue with module maps/portable configurations, native DOM/AST/event projection
projects, then close the remaining catalog-wide README/acceptance checklist.
