# Source-operation diagnostics and original owners — 2026-10-27

- [x] Reproduce retained XML source errors, missing selected-view diagnostics and malformed-inspection error-string loss with failing native fixtures.
- [x] Publish source diagnostics separately from the Inspect presentation body, with backward-compatible decoding for older responses.
- [x] Use retained native imports for XML-family/data source projections and native stylesheet views, preserving literal data, source identities, ranges and original diagnostics.
- [x] Keep Markdown source tooling before document-to-HTML lowering; present native events and original ranges without inspecting generated HTML.
- [x] Return structured diagnostics and no structural artifact on refused inspection, while retaining the diagnostic view.
- [x] Export the selected operation's diagnostics through CLI reports and common command services, without a second Inspect invocation.
- [x] Retire only the proven obsolete Supplier XML and component/MathML/SVG/XSLT diagnostic exceptions; retain the separate portable control-source limitation.

## Cause and correction

Parse sent some successfully loaded XML-family sources through generic CEM
namespace and binding rules. Inspect could present the retained imported tree
while its diagnostic view used that different generic processing path. Its
public response omitted diagnostics entirely, forcing catalog checks to run a
second diagnostic-view command. Structural import refusals became internal-error
strings and lost their original structured diagnostic fields.

`InspectResponse.diagnostics` now carries the selected operation's native
diagnostics independently of its body and presentation syntax. Older serialized
responses decode with an empty diagnostic list. CLI Inspect supports the common
report options; common command products retain the same diagnostics. Neither
host extracts diagnostics from exported AST fields or presentation text.

Admitted retained XML, HTML/XHTML, SVG, MathML, CSV and structured-data owners
feed the shared source projection directly. Native CSS, XSLT and RELAX NG
source views use their lifecycle results, including their native event ranges.
CSS AST remains lexical and CSS tree remains semantic. Generic custom-schema
passthrough remains available when no native owner was admitted.

Markdown inspection previously described generated HTML offsets as source
positions. Structural source Parse and Inspect now load the original Markdown
owner before HTML lowering. Its native events supply source rows and original
ranges, including Unicode/CRLF. This is a native event presentation, not a claim
of a complete Markdown language AST. Ordinary document conversion, Markdown
tables and rendered previews retain the existing HTML-lowering pipeline.

Markdown site **1.1.1** keeps every resource/configuration ID and path. Its
`inspect-article` label and expected output now describe native source events,
including heading text at original byte **171**. Preview and outline queries
retain their generated HTML behavior. Newly cloned projects receive the updated
metadata; existing saved projects and drafts retain their own settings.

Refused structural views return a null internal body, no presentation bytes and
the original structured diagnostics. CLI/common publication does not publish a
`null` placeholder artifact or turn refusal into a write error. Such a structural
request fails; inspecting diagnostics still succeeds and exposes source errors.
Inspect remains an inspection operation rather than an implicit validation
command. Original severity, URI, source maps, zero values and absent coordinate
fields remain intact.

## Verification

- Full native CLI library: **549/549 pass**, including selected-view report publication, all six views, original malformed XML positions, failure status and no partial output.
- Native source-owner regressions cover all six views across XML, XHTML, SVG, MathML and XSLT, source warnings with absent/zero coordinates, refused imports and Markdown original event offsets. Structured-data, mixed-quote and XML retained-owner tests pass after updating refusal checks to require structured diagnostics and no partial artifact.
- Common command execution: **16/16 pass**, including native diagnostics on the selected result and no `null` publication on refusal.
- Legacy Inspect response decoding and explicit wire-format selection: **1/1 pass**. The warning/source-map fixture preserves exact original frames plus absent and zero coordinates across every view.
- Optional-schema admission: **1/1 pass**; missing source assets preserve native admission errors without CEM fallback.
- Direct authored catalog checks: **6/6 pass**. Supplier XML and component/MathML/SVG/XSLT Inspect report zero diagnostics from their original operation. Portable Studio CEM inspection retains its exact **89** pinned original diagnostics as a separate known limitation.
- Catalog acceptance/audit units: **48/48 pass**. A companion-view result can no longer satisfy selected-operation evidence.
- Node and browser WASM package: **20/20 pass**, including original SVG accessibility warnings/maps across all views, native diagnostic publication and refused structural requests without artifacts. All **12 CEM-QL WASM scripts pass**.
- Package verification: **657 integrity records**, browser/Node ABI and all **32 schema packages** pass.
- Studio source browser acceptance: **5 selected cases pass**, covering the new original-owner diagnostic/repair path, ordinary XML/XHTML, Markdown inspection/preview/metadata repair, JSON/YAML saved sources and Login query sources.
- Complete live Login recovery: **2 cases pass**, worker and fallback, with entered password/one-time-code refusal until cleared, source edits and fresh-generation handoff. Original per-case deadlines and runtime budgets remain unchanged; the selected invocation takes 186.01 seconds.
- Assembled demo verification passes: all owned file hashes and 333 Studio assets match, independent saved/offline Studio HTML/CSS preview works, all ten theme generators retain their CSS, and 120 canonical playgrounds plus the live tree gallery pass. Fixture-owned copies preserve users' projects and drafts.
- Deployed Studio worker catalog: **254 configurations checked online and offline**. Each phase records 217 passing executions, 33 intentional diagnostics, the one pinned portable-control source limit and three explicit binary-source policy refusals. Supplier XML now passes with no diagnostics. Exact offline outputs are verified; originals and copies remain unchanged. No companion-view evidence is used.
- Native catalog and all **32 schema-package examples pass**: **254 configurations executed**, with 220 passing, 33 intentional diagnostics and the one pinned portable-control source limit. Every Inspect receipt uses its original operation's report; no companion diagnostic limits remain.

The initial WASM fixture incorrectly required an unnamed SVG to have no warnings;
the corrected regression requires its native accessibility warning and exact
maps in every view. Browser acceptance also caught the old generated-XHTML
expectation in the Markdown sample and a missing explicit resource-scheme option
in the new fixture. Those source/test expectations are corrected above. One
catalog invocation overlapped the sample rebuild and observed a temporarily
missing static file; only the completed-build rerun supplies final evidence.
No runtime limits, test deadlines or diagnostic assertions are relaxed.

The full demo build and inventory publication pass: **898 files**, including
**333 Studio assets**, with all four WASM copies below 25 MiB. Pinned Binaryen
**132.0.0**, `-Oz`, preserves all capabilities and unchanged ABI counts:

| Runtime | Optimized bytes | SHA-256 | Exports/imports |
| --- | ---: | --- | --- |
| CEM-QL | 23,744,795 | `08b6b8b5a8912beadb8e6e1d4238a0e49905f311eb3780e68c7081613120b03a` | 169 / 28 |
| CEM-ML | 24,394,392 | `031f060c2a835b7e8ce3f5902edd52eb109a5ff40e1b9e0563786246a69922a9` | 170 / 28 |

## Remaining scope and privacy

Portable Studio project CEM inspection still uses the generic source projection;
its typed control-source adapter is separate work. The remaining diagnostic
fixture is now explicitly evidence from that selected operation's report. Its
original positions, warnings and hard errors remain pinned, including absent or
zero coordinates. This task does not claim clean control-source conformance.

No production application JavaScript, UI behavior or component styling is added.
Reusable functionality remains in the shared Rust/runtime paths; Studio keeps
its declarative shared controls. Passwords and one-time codes continue to block
recovery until explicitly cleared. Automatic recovery remains disabled. Existing
4 MiB checkpoint/group limits, 32,768-node witness guard and original deadlines
remain unchanged. Generated binaries, reports, screenshots and Vitest
attachments are excluded from the source commit.
