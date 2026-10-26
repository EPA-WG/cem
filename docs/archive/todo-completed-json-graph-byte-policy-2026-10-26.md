# Shared abstract JSON graph byte policy — completed 2026-10-26

The native CLI published indented abstract JSON graph values while common
command execution published compact JSON. The shared native regression first
reproduced that difference. Application JSON graph exports now have a single
native encoding policy; Studio acceptance compares exact bytes and hashes.

## Implementation

- [x] Define the shared `cem_ml::graph_output` boundary: declared
  `application/json`, `text/json` and `+json` data exports use UTF-8, two-space
  indentation, LF, native JSON map/number/escaping semantics, and no BOM or
  appended newline. MIME case and parameters do not change that policy. An
  absent identity does not select JSON implicitly.
- [x] Give explicit `PrimaryBytes` precedence, including empty output, native
  whitespace/line endings, key order and binary bytes. Do not reparse, trim or
  reformat a native writer's bytes.
- [x] Connect the same encoder to staged CLI publication, the direct/production
  stdout writer and common command-service graph publication. Preserve native
  source maps, output labels, media types and hashes of actual published bytes.
  Authored graph configurations still require `@out`; an inadmissible missing
  destination is not used to bypass that existing contract.
- [x] Distinguish native collection controls with producer-owned runtime
  metadata derived from the typed collection body. The marker is skipped on
  the control wire and cannot be recovered from application JSON fields.
  Keep collection manifests/sidecars on their existing publication paths.
- [x] Preserve complete application JSON values, including scalar strings and
  fields named `content`, `kind`, `items`, `sourceMap` and `outputSpans`. A fixture
  that resembles a collection contains an unregistered nested destination;
  it remains data and cannot publish a child artifact or a sidecar.
- [x] Replace graph JSON value-only acceptance exceptions with exact-byte
  comparisons. The ordinary CEM-QL project also checks entry-artifact hashes
  and complete offline artifact identity. Pinned builds retain stored bytes;
  the policy applies when a new build is published.
- [x] Pin Supplier 1.1.1 to the shared byte contract. Exact catalog acceptance
  exposed a hand-ordered expected JSON file with a final newline. A new native
  CLI regression reproduced the mismatch before the fixture was changed. Its
  records are unchanged; the fixture now matches native CLI/worker lexical
  ordering and no appended newline. Source IDs and configurations stay stable,
  and the versioned catalog package, README and integrity index are rebuilt.
- [x] Correct the existing native project-cohort assertion from seven XPath
  configurations to the catalog's current 15. Keep all native operation,
  output, diagnostic, source-range and repair assertions.

## Verification

- [x] Native graph suites: 44 tests passed, including nested/scalar JSON,
  Unicode/escaping, large integers, native collection distinction, explicit
  empty/raw writer bytes, MIME handling and original source-map retention.
- [x] Native command-service suites: 63 tests passed.
- [x] CLI transformation suites: 74 tests passed, including collection
  sidecars, source maps, YAML/XML contracts and direct/staged JSON publication.
- [x] Supplier native cohort: all six tests passed, including exact JSON bytes
  while preserving existing semantic, namespace, repair and isolation checks.
- [x] Native data-query projects: all five tests passed; Login graph tests:
  both passed. The ordinary JSON export retains `9007199254740993` exactly
  through native import, transformation and file publication.
- [x] Full Studio build and 21 dependencies passed, including both WASM
  runtimes and the 59-component declarative architecture check. Studio type
  checking, catalog audit and 23 catalog acceptance contract tests passed.
- [x] Six focused browser cases passed: CEM-QL parameter-map builds with exact
  JSON bytes and the large integer; CEM-QL/XPath source operations; and three
  pin persistence, corruption/admission and aggregate-size cases.
- [x] Entered credential recovery refusal passed on worker/fallback runtimes.
  Passwords and one-time codes continue to block recovery; automatic
  whole-Studio recovery remains disabled.
- [x] Complete deployed catalog command acceptance passed: all 250
  configurations across 23 registered projects passed online and offline, with
  exact graph-byte comparisons and the original diagnostic/admission limits.
  Module Maps JSON asset publication also retained its explicit final newline.
- [x] Deployed CEM-QL/XPath navigation, saved copies and offline rebuilds
  passed, including exact JSON entry text/hash, independent data/query/parameter
  edits, source navigation, last-good failed-build retention and complete
  offline artifact identity. Original catalog sources remain unchanged.

The frozen catalog identity is
`9b8d10978dd31bb55ad8b91844440e3cffe2749d9fe2d1505510b0f7080e446f`;
the deployment SHA-256 is
`03657564d4941f684d5d31ec2fc4ceb5949e6cf93f505b176bad5df94f8c61d9`.
The catalog remains at 22 authored projects, 213 configurations, 218 documented
workflows and 32 registered native packages.

Native collection-control manifest/sidecar parity remains a separate TODO.
This change preserves the CLI sidecar contract and the common host's inline
collection metadata; it does not claim uniform collection-control bytes or
move/drop their source metadata. Reports, non-JSON formats and source exporters
retain their own contracts. No Studio UI behavior or component JavaScript was
introduced. Tests use isolated browser profiles and temporary project copies;
user projects/drafts and pinned snapshots are preserved. Generated screenshots,
Vitest attachments, reports, binaries and WASM files remain outside Git.
