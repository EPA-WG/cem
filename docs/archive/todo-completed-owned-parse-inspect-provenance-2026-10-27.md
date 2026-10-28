# Owned native Parse/Inspect provenance — 2026-10-27

## Implementation

- [x] Reproduce missing owned publication on both previously built public WASM hosts before rebuilding them: both new fixtures refuse their first CEM presentation because source-map references are absent.
- [x] Exercise CEM/XML/JSON/XPath/Markdown native AST/event Parse and AST Inspect, Unicode/CRLF coordinates, direct CEM conversion, original/output byte witnesses, bounded fanout, atomic ownership and stale publication.
- [x] Retain typed writer output spans through `PrimaryBytes` without reconstructing native trees from JSON or parsing a presentation to obtain links.
- [x] Project artifact-owned references through the existing Studio data model and declarative result/selection controls, preserving source identity, original ranges and revision refusal.

## Contract

`PrimaryProvenance` is a native-only witness created by the writer's producer.
It binds the admitted original URI, original byte length/BLAKE3 identity and
final writer-byte BLAKE3 identity to typed `OutputSpan` values. Native input
owners supply `SourceBoundary` metadata before presentation formatting. Native
AST/event rows retain actual original ranges; serialized `source-id`,
`byte-offset` and AST attributes are never read back as ownership grants.

Common publication matches the witness to the admitted request resource and
exact output bytes. Only explicit matching original-source boundaries with
valid UTF-8 source/output ranges are admitted. Foreign URIs, anonymous numeric
IDs, formatter-module origins, changed source/writer bytes, overflow, out-of-bounds
ranges, split scalars and binary buffers supply no text-navigation endpoint.
The raw typed writer spans remain unchanged in the native witness.

Published references coalesce repeated occurrences of each original native
range. Each reference includes a representative `outputRange`; it is not a
complete output-span stream or a character-linear mapping through escaping,
formatting or coloring. Actual whole-input boundary metadata remains distinct
from finer native rows. Generated content with no named native boundary gains
no fabricated endpoint. Existing negotiated reference/byte limits and truncated
counts remain in force; primary links for fanout outputs precede extra rows.

References become artifact-owned only after atomic publication supplies real
handles. Stale publication clears maps and artifacts; failed publication confers
no artifact ownership. Explicit JSON inspection and JSON/binary codec exports
remain separate data contracts, with no implicit AST owner or navigation grant.
Direct CEM conversion and native source formatters retain writer witnesses too;
anonymous formatter origins remain non-navigable pending producer URI adapters.

Studio projects the common references into the selected artifact's provenance
using its existing workbench service. The existing declarative Results selector
opens the original resource/range; saved revision changes require a rerun.
No Studio-specific UI JavaScript, new parser, credential read or recovery grant
is introduced. Entered Login password/OTP refusal, disabled automatic recovery,
4 MiB checkpoint/group limits, 32768-node witnesses and timeouts are unchanged.

The ordinary `native-projections` project is version 1.0.2. Its acceptance checks
follow native CEM/XML presentation links through real editor selection and
repeat after an independently saved clone is rebuilt offline. JSON/binary
transfers retain exact-byte and read-only admission checks. Catalog guides no
longer claim that native presentation provenance is unavailable.

## Existing fixture correction

The broader native command cohort exposed an obsolete named-JSON Inspect
fixture expecting the original XML `note` element at the top level. The already
built CLI also returns AST vocabulary rows. The assertion now finds that native
envelope, the original element-name row and the inert literal text attribute,
retaining its empty `valueNodes` assertion. No production presentation contract
was changed to accommodate the test. Authored diagnostic-script disposal no-ops
now explain the closed-page race, and an unused acceptance-test import is removed.

## Verification

- Native command/transport cohort: **68/68**; projection validators/codecs: **33/33**; adversarial byte/range witness fixture: **1/1**.
- Native CLI library: **550/550**.
- Public package runtimes: **28/28**, including exact original CEM/XML `Café` ranges, Unicode/CRLF, foreign/data-field refusal, explicit JSON separation, malformed-source repair and stale publication on both hosts.
- Package verification: browser/Node ABI, **657** integrity records and all **32** schema packages pass. All **12** CEM-QL WASM scripts pass.
- Studio source-selection/revision guards: **32/32** final focused checks pass; the native browser projection/export/read-only fixture passes. The declarative selector retains all **100** bounded rows and supports selection of the final row. A final check against the refreshed deployment also follows actual XML/CEM ranges into the editor and repeats a saved-clone event parse offline.
- Full native catalog: **268** executed, **232** successful and **36** intentional diagnostics; **zero** known or companion diagnostic limits. Catalog identity: `be704225ed433363aa9ad92a008235ceab53e151885fa9f7ae7115d70f0ec2a1`.
- Native-projections saved/offline browser acceptance passes: real CEM/XML source selection, exact binary transfers, source/shape repair, saved Unicode/range invalidation, independent copies, offline rebuilt exports and unchanged originals.
- Complete live Login recovery: **2/2**, worker and fallback, after the footprint correction below. Each preserves both credential refusals and completes three recoveries/12 edits under the existing node, 4 MiB checkpoint/group and timeout guards.
- Demo publication: **898** files, including all **333** Studio files. Deployment hashes, paired runtimes, native rendering/highlighting, independent saved Studio HTML/CSS preview/offline rebuild, all **10** unchanged stylesheet generators and **120** canonical playground pages pass.
- Binaryen 132 `-Oz`: CEM-QL **23,808,409** bytes, SHA-256 `16d24b88229fd65e4662a9d98537755c7e623f6577efcb1942a212dab202ab05`, **28 imports/169 exports**; CEM-ML browser and Node **24,481,080** bytes, SHA-256 `2946b9bfd79d51228e7a7d59d2ed696baec3a617ea67e13a7fb515f0e31056f5`, **28 imports/170 exports**. All four deployed WASM files stay below 25 MiB.
- Authored Studio source/scripts lint and catalog acceptance/audit unit checks pass. Generated screenshots and Vitest attachments remain ignored.

## Recovery footprint correction

The first isolated recovery checks correctly refused a **33,442-node** Login
workspace at the unchanged **32,768-node** witness limit. New native provenance
rows made retained formatting-only whitespace significant. Reducing only a
subset left 32,901 and then 32,782 nodes, still correctly refused. The final
results, repeated command-field and workspace declarations avoid this inert
whitespace; their controls, layouts and every bounded source row remain intact.
The results data projection also transports only the frame fields its declaration
renders, while the service retains complete provenance/ownership records.

Both complete recovery modes then pass. No witness exclusion, larger budget,
timeout extension, credential clearing by recovery, reduced row cap or new
Studio UI JavaScript capability was introduced. The existing shared list, table,
tabs, editor and workspace components still own their behavior.
