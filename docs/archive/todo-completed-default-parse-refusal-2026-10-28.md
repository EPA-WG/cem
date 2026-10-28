# Default Parse refusals and original CSV diagnostics — 2026-10-28

## Contracts

- [x] Reproduce the default CLI exit-zero/no-output failure and the common native
  product's missing refusal status before changing production behavior.
- [x] Verify shared native refusal policy, original schema diagnostic identity,
  existing destinations and recoverable source inspection.
- [x] Verify shared output readers, Node workers and Studio default Parse/repair.
- [x] Verify optional-profile default Parse online/offline and isolated Login
  credential-policy recovery before commit/push.

## Cause and repair

CSV import correctly refused malformed quoting with its original schema-owned
error and no retained tree. CLI dispatch and common command execution already
suppressed output publication, but still applied the diagnostic-only Parse fail
level, which fails only fatal diagnostics. A native refusal could consequently
return success with no artifact. Studio's strict shared publication reader then
reported the missing publication before the native diagnostic reached its caller.

`ParseResponse::is_refused` shares the existing native refusal predicate: no
presentation bytes, a null compatibility tree and a hard native diagnostic.
CLI dispatch and common command execution now return exit 1 for that refusal at
all failure levels. The command service settles a failed result once, with the
original diagnostics and no output artifacts or publication attempt. Existing
output destinations remain intact. This changes terminal policy, not parser
severity, grammar, schema fact classification or source admission.

Publishable partial-source inspection retains its selected diagnostic failure
level. Default Parse can still return a readable partial CEM-QL source view with
nonfatal errors; explicit validation level fails those errors. Control-source
semantic refusals without an artifact now also fail at Parse-only level; their
original diagnostics remain unchanged. Readable diagnostic Inspect views retain
their existing behavior. The output reader still refuses exit-zero commands with
missing publications and distinguishes failed absence from a published empty artifact.
No format-specific host branch, diagnostic-code matching, fabricated empty output
or host parser is added.

The optional Studio verifier again calls ordinary `parseResource` for its
malformed CSV probe, asserting exit 1, absent output and original coordinates;
it no longer needs the temporary strict-policy probe from the XSLT acceptance.
Studio and shared components remain declarative; production JavaScript UI code
is unchanged. Entered passwords/one-time codes continue to block recovery until
explicitly cleared. Automatic recovery, checkpoint limits and deadlines remain
unchanged.

## Acceptance fixture alignment

The full Node cohort exposed an obsolete census length: it expected 29 although
`schema-package-owners` has been the thirtieth native registry since `aa9abee63`
(2026-10-24). The fixture now checks exactly 30 plus the package-owner registry's
empty counters. Native registry code and DTO/authority bounds are unchanged.
The new Node Parse test's output uses an explicit file URI, as required by the
service's existing prepared-URI admission contract; a relative `--out` destination
initially refused before reaching Parse. That Node argv-path canonicalization
limitation is recorded separately in the TODO for contract/fixture work. Neither
issue required weakening URI admission or a runtime workaround for this repair.

## Verification

- Native tests-first evidence: the CLI fixture reproduces exit 0 with the original
  error and no artifact; the common command fixture reproduces missing forced
  refusal. Recoverable query-source inspection passes before and after repair.
- Common native command execution: 21/21 tests pass, including a retained original
  CSV schema that renames the quote diagnostic to `example.quote`, original
  source coordinates/fact details, exactly one terminal and no refused writes.
- CLI: 7/7 Parse/control/structured-data tests pass. CSV unclosed-quote and invalid
  escape cases exercise six output formats and four failure-level selections
  (including omitted/default), preserving existing destination bytes and report
  diagnostics. Repaired Unicode CSV publishes; query-source partial inspection
  keeps default success and explicit validation failure.
- Shared output reader: 9/9 focused units pass, preserving native failed absence,
  successful-empty artifacts, strict missing-publication refusal and bounded
  range/digest/manifest validation. An initial aggregate target was interrupted
  when it began rebuilding WASM prerequisites; the focused unit command executes
  the actual cases directly.
- Full Node command-service cohort: 54/54 tests pass after the fixture alignment
  above. Worker Parse retains the original CSV error/URI/coordinates/fact, reports
  failure without artifacts, preserves an existing file and publishes repaired
  Unicode CSV. All portable-operation, pool, ownership, inventory, admission,
  closure and retirement cases remain green.
- Default optimized binaries: CEM-QL 23,959,029 bytes; common CEM-ML 24,639,778
  bytes. Existing optimizer import/export descriptors and 25 MiB gates pass.
- Default Studio browser acceptance: 5/5 focused cases pass (58 unrelated cases
  skipped), including both CSV errors across five projection choices, readable
  diagnostic Inspect, repair, inventory execution, native source ownership,
  typed control sources and recoverable Login query-source inspection.
- Full optional-profile Studio verifier passes online/offline: ordinary default
  CSV Parse rejects with the original error and coordinates, absent output and
  no artifact; normal parsing, output-driven/selected-resource demand, all four
  format packs and native XSLT graphs remain green. It verifies all 205 assets,
  strict runtime/pack pairing, cache isolation, cold offline commands, independent
  source replay across invocation/command/rendering/fallback cohorts, and
  credential/stale/retirement refusals. Automatic recovery remains disabled.
- Complete browser/Node ABI and distribution verification passes: 658 integrity
  records and all 32 schema packages.
- Deployed demo acceptance passes: 898 managed/total files and all hashes, all 333
  Studio files and paired runtimes, shared initialization/isolated memories,
  native rendering/highlighting, Studio selection/saved copies/previews/offline
  builds/worker scope, ten unchanged theme generators, 120 canonical playgrounds
  and the live tree gallery. All four default deployed WASM files are below
  25 MiB; the separate optional prototype retains its existing opt-in size policy.
- Authored-script lint and `git diff --check` pass.
- Final isolated Login recovery passes 2/2 worker/fallback cases (6 unrelated
  cases skipped): three recoveries, twelve edits and two credential refusals per
  runtime. Complete workspace/group checkpoints remain within 4 MiB; the
  32,768-unit witness limit, allocation/resume deadlines and disabled automatic
  recovery are unchanged. Generated screenshots and Vitest attachments remain
  excluded from Git.
