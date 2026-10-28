# Formatter artifact-reader freshness — 2026-10-28

## Completed contracts

- [x] Reproduce and label the complete formatter/colorizer discovery and
  execution read trace, retaining exact ordered artifact requests.
- [x] Add pipeline mutation/refusal, re-admission and URI/MIME cache identity
  fixtures for local/explicit caches and legacy/authoritative readers.
- [x] Verify native cache/output/provenance cohorts, unchanged public WASM and
  Studio formatting, and isolated credential-policy recovery.

The original environment-reader fixture failed before editing: its four-read
expectation received eight. The prior clean-baseline reproduction is preserved
in the [formatter provenance archive](todo-completed-formatter-markup-provenance-2026-10-28.md#independent-existing-failure).
The full four-artifact format/color pipeline currently reads:

| Phase | Ordered requested artifacts |
| --- | --- |
| Format binding/function discovery | Formatter helper, colorizer helper |
| Formatter execution | Formatter body, formatter helper |
| Color binding/function discovery | Formatter helper, colorizer helper |
| Colorizer execution | Colorizer body, colorizer helper |

The registry discovers both helper kinds at each binding phase. Every parsed
module-options lookup rereads its currently admitted source before considering
reuse. The cache key includes the full descriptor, actual URI, returned content
type and source-byte SHA-256. The old expectation omitted four helper reads
when parsed options were already available. Descriptor-only or once-only reuse
would miss changed or revoked source admission. The production reads are retained.
Git history identifies the cause: `ee6c073d8` (2026-10-24, owned schema
inputs) replaced descriptor-only cache reuse with source admission before every
lookup and source-qualified keys. The legacy-reader fixture retained its older
four-read assertion. The corrected fixture preserves that admission repair.

The original named test now uses a reusable test-only fixture and asserts the
complete trace, unchanged native formatter/colorizer execution descriptors,
absence of fallback functions and colored HTML output. It exercises legacy and
authoritative readers, automatic local and explicit caches, and repeated runs
against warm explicit caches.

New execution checks rename helper declarations, supply malformed source bytes,
or revoke reads at positions 4 and 8, after discovery has admitted the original
helpers. All 24 reader/cache/stage/change permutations refuse before native or
public writer output, then succeed after re-admission using the same explicit
cache where applicable. No writer elapsed marker or output spans appear after
refusal. URI/MIME changes at formatter and colorizer helper execution qualify
new cache entries bound to the actual source bytes. The expanded fixture runs
58 complete/failed pipeline invocations across its parameter matrix.

Existing asset-pack tests continue to cover the authoritative refusal versus
legacy embedded-fallback distinction, byte/URI/MIME/descriptor qualification,
cache-hit admission, missing sources and the 256-entry bound. No production
parser, source reader, registry, cache, formatter, writer, command, component or
Studio code changed. The production conversion prefix is byte-for-byte identical
to committed source; new helpers are included only inside `#[cfg(test)]`.

## Verification

- Original isolated fixture: 1 failure before repair, with the exact eight-read
  sequence recorded.
- Output-pipeline native cohort: 9/9 pass with default features and 9/9 with
  `--no-default-features`, including the repaired trace and two new matrix tests.
- Native asset-pack/cache cohort: 7/7 pass.
- Native formatter/provenance cohort: 5/5 pass, including both intended native
  command writer/source-identity cases and Markdown/CSV formatter checks. An
  initial module-qualified filter selected zero tests; the corrected
  `native_formatter_` filter executed these five cases.
- Public Node/browser-worker WASM palette/encoding and original writer-provenance
  checks: 4/4 pass. Distribution verification checks the browser/Node ABI,
  658 integrity records and 32 packages.
- Studio Login formatting, inert source-color export and original diagnostic
  ownership browser tests: 3/3 pass.
- New Rust helper code is formatted; the large conversion module was not
  reformatted. `git diff --check` passes. Existing generated tree-sitter C
  warnings remain unrelated.
- Test-only changes require no regenerated WASM or deployment assets; frozen
  production output and all shared UI/theme contracts remain unchanged.

- Final isolated complete Login recovery passes 2/2 worker/fallback cases,
  with three recoveries and twelve edits per runtime and two credential
  refusals per runtime. Entered passwords/one-time codes still block recovery
  until explicitly cleared; automatic recovery remains disabled. The 4 MiB
  group budget, 32,768-unit witness limit and all deadlines are unchanged.

Generated screenshots and Vitest attachments remain excluded from Git. The
constructed XML document/provenance URI separation and generated WASM lint scope
remain separate TODO items.
