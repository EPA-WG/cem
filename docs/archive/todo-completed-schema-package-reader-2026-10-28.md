# Full schema-package validation resource readers

Full package validation previously tried the local filesystem before its supplied
resource reader. A file-like manifest identity could therefore validate an
unadmitted local artifact instead of using the embedding host's source or
reporting its refusal.

The shared native package-validation cache now consults the supplied reader
first for artifacts, converter templates and examples. It forwards the original
reference, manifest base and MIME hint, preserving per-validation caching of
both success and failure. A reader error reaches the existing diagnostic builder
without a filesystem substitution. Local filesystem reads retain their previous
behavior when no reader is supplied.

This changes resource acquisition order only. Successful reads still undergo
full artifact/source contracts and expected-example checks. Metadata-only
owned-pack admission still performs no artifact/example reads. Strict runtime
artifact consumption, immutable source publication and replacement authority are
unchanged. Studio adds regression coverage and documentation; application and
shared component UI behavior remains declarative.

- [x] Reproduce disk overriding supplied artifact/example bodies for plain-path
  and file-URI manifests, then require the reader's source and original hints.
- [x] Require reader refusals despite readable disk files, retaining error
  severity, source ranges, structured diagnostics and shared refusal caching.
- [x] Preserve no-reader filesystem success/missing-file checks, full example
  result checks and metadata-only admission without resource reads.
- [x] Verify ordinary engine full validation against a partially admitted owned
  CSV pack whose canonical paths also exist in the checkout. Missing admitted
  bodies must produce resource diagnostics rather than borrowing those files.
- [x] Exercise Studio command-worker package validation with available bodies,
  missing artifact/example bodies, malformed XML and repair; retain the existing
  owned Login teaching-package validation/Parse/Inspect checks.
- [x] Rebuild, verify privacy and deployment, record the contract and close the
  active checklist without changing any recovery bound.

## Verification

The original precedence/refusal fixtures fail while the original no-reader case
passes. The owned-pack engine fixture also reproduces missing unreadable-artifact
diagnostics before the repair. Afterward:

- All 65 package-validation tests pass.
- All 13 owned-package activation tests and the persistent-session case pass.
- Five CLI package diagnostic groups and the detailed resource-payload case pass.
- Public browser/Node WASM ABI, all 658 integrity records and 32 schema packages
  pass verification. Release optimization retains the import/export interface.
- Both focused Studio worker cases pass. New assertions retain the existing
  example diagnostic shape (`invalidValues.path`) rather than changing it to
  match the artifact's additional top-level path field.
- Studio lint and whitespace checks pass.
- Complete live Login recovery passes on worker and fallback: three recoveries,
  twelve edits and two credential refusals per mode, within the existing 4 MiB
  checkpoint budget and original allocation/resume/test deadlines.
- Demo build/copy and deployment verification pass: 898 files, matching owned
  hashes and all 333 Studio files, four WASMs at 22.96/23.63 MiB, unchanged ten
  theme CSS outputs, 120 playgrounds, live tree gallery, independent saved
  project preview and offline rebuilds.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. Browser checks use
isolated storage; no saved user projects or drafts are cleared. Generated
screenshots and Vitest attachments remain ignored and excluded from Git.
