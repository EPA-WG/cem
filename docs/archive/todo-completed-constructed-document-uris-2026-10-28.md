# Constructed document and provenance URIs — 2026-10-28

- [x] Add failing native import/parity fixtures and public WASM/Studio command
  regressions for constructed document, base and provenance URI behavior.
- [x] Correct the shared native accessor while preserving source ownership,
  diagnostics, original ranges and query argument guards.
- [x] Rebuild and verify native/WASM/Studio/deployed consumers, then run isolated
  Login credential-policy recovery with existing budgets and deadlines.

## Contract and repair

String import already supplied explicit document metadata with no document URI,
independently of its provenance address and optional base URI. The shared native
XPath node accessor ignored that metadata and returned the source URI instead.
CEM-QL and XPath both consume this accessor. The existing CEM-QL assertion for
`data:document_uri(data:parse("<r/>", "xml"))` failed before the repair, returning
`data:parse/source`. A new native import test reproduced the same mismatch;
the public CEM-QL WASM regression also fails against the previous distribution.

The accessor now reads the retained tree's optional document URI and preserves
its document-kind guard. This changes no parser, import profile, native owner,
source identity, source-map frame, base URI or query argument contract.

| Input | Document URI | Base URI and provenance |
| --- | --- | --- |
| CEM-QL string import | Absent | Optional supplied base; `data:parse/source` provenance |
| XPath string import | Absent | Static expression base; original parsed-source provenance |
| Loaded native document | Retained document URI | Existing imported metadata and original source ranges |
| Non-document node | Absent | Existing inherited base and native source ownership |

The regression matrix covers XML, JSON-to-XML, CSV and YAML with Unicode input,
absent/explicit bases, native owner identity, source boundaries, bounded original
UTF-8 ranges, source lines, malformed-input diagnostic ownership, CEM-QL/XPath
parity and loaded-document behavior. The original failing assertions remain.
Public WASM tests exercise standalone CEM-QL and both common runtime loaders;
Studio coverage uses its normal native query command service.

The shared native engine owns the repair. No application/component JavaScript
parser or UI behavior was introduced. Entered Login passwords/one-time codes
must still block recovery until explicitly cleared; automatic recovery remains
disabled. The 4 MiB checkpoint budget, 32,768-unit witness bound and recovery
deadlines remain unchanged.

## Verification

- Native import/accessor/string-parsing cohort: 14/14 pass.
- Reduced-feature native import, provenance, retained-tree and XPath cohort:
  33/33 pass, including the strengthened parsed-source identity assertions.
- Native CEM-QL import/parity, source-provenance and namespace-view cohort:
  9/9 pass. The previous constructed-string URI assertion is unchanged.
- Direct native CLI CEM-QL and XPath probes preserve loaded-file URI identity,
  explicit bases, empty constructed document URIs and source lines.
- Studio's authored browser test passes ESLint; JavaScript syntax and
  `git diff --check` pass. Existing generated tree-sitter C warnings remain.

- The complete `yarn copy:demo` build succeeds. The deployment contains 898
  files; both CEM-QL copies are 23,968,047 bytes and both CEM-ML copies are
  24,645,558 bytes, all below 25 MiB. Native/WASM capabilities are preserved.
- All 13 CEM-QL public WASM scripts pass, including the new URI regression in
  standalone CEM-QL, common Node and common browser WASM. This includes parsing,
  native ownership/transport, caches, memory budgets and checkpoints.
- Public common runtime Node/browser tests: 28/28 pass. Distribution verification
  passes for both ABIs, 658 integrity records and 32 schema packages.

- Studio query commands, cross-query original-source provenance and diagnostic
  ownership browser cohort: 3/3 pass.
- Deployed acceptance passes: all 898 inventory hashes, all 333 Studio files,
  shared gallery initialization/isolated memories, native rendering/highlighting,
  Studio URL selection and independent saved copies, HTML/CSS preview, offline
  rebuild and worker scope, all ten unchanged theme generators, 120 canonical
  playground pages and the live tree gallery.

- Final isolated complete Login recovery: 2/2 worker/fallback cases pass, with
  three recoveries, twelve edits and two credential refusals per runtime.
  Entered passwords/one-time codes block recovery until explicitly cleared;
  automatic recovery remains disabled. The original checkpoint/witness budgets
  and all allocation/resume deadlines are unchanged.

Generated screenshots and Vitest attachments remain excluded from Git.
