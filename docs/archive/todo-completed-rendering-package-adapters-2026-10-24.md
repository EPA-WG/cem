# Completed: rendering-worker and owned-fallback package adapters

- [x] Add rendering-worker and owned-fallback package adapters with exact physical
  bindings and checked package release before native retirement.
- [x] Add real optional-profile worker/fallback fixtures for native source
  consumption, consecutive offline package handoffs, credential/ownership
  refusal, cleanup failure and stale aliases; extend retirement/transport unit
  coverage.

The shared processing host exposes `processingSchemaAssetBinding(host)` with the
original physical package endpoint and generation participant. Rendering-worker
endpoints use the actual Worker; owned fallback endpoints use their actual WASM
Memory. Both work with `CemSchemaAssetPackLoader` and
`CemSchemaAssetPackRecoveryBinding`. Native interpretation, source authentication
and package compilation remain in Rust. This adds no Studio-specific UI behavior.

Trusted rendering setup uses `packageHostConfiguration` on processing-host
options, or `processingPackageHostConfiguration` on `CemElementRuntime`. Preview
options retain that original string. Workers snapshot it before work and renew
it through their original fresh allocator. Different configurations use separate
worker pools; an existing logical root refuses changing configuration instead of
silently returning a differently authorized cached host. Owned fallback receives
its original setup through `isolateCemFallbackGeneration` before initialization;
its fresh allocator retains that setup. Legacy singleton fallback refuses an
owned package binding. Loader options and authored demand cannot replace grants.

Command and rendering adapters vendor one canonical original-byte/control
validator from `cem_ql/runtime/schema-asset-protocol.mjs`. Each uses its own
explicit channel. Unknown fields, native owner/grant fields, sparse source lists,
shared mutable buffers and oversized backing buffers remain refused. Existing
index, source and aggregate limits remain in effect. Packaging and the native
Studio module maps explicitly deliver the new shared protocol modules; there is
no application-local asset-copy assembler.

Ordinary package work joins the original processing domain drain and invalidates
stale coverage. Cleanup uses its checked retirement path without mutating the
closed proof. `retireNativeRenderingGeneration` checks original closed coverage,
complete client/source membership, the current bounded handoff and original owned
fallback embedding consent before releasing package owners. Resource release and
native retirement share the existing five-second deadline; the 4 MiB rendering
handoff bound and native empty-inventory permit remain mandatory. Failed cleanup,
changed credentials or ownership, missing peers and stale outcomes fence the
source instead of publishing a replacement.

Fresh package preparation joins the existing complete binding transaction before
any host route publishes. Fresh worker/fallback gates fence saved package aliases
on revocation, while exact native owner cleanup remains available before physical
release, including a fenced fallback heap. Native allocation pressure is sampled
after package calls, so the domain reports pressure before another operation or
fresh publication is admitted. Explicitly bound package hosts refuse implicit
worker failure fallback: another heap cannot borrow their package context.

## Verification

Passed 215 focused tests:

- 123 shared processing-domain, owned-fallback, worker-transport,
  checkpoint-resume and recovery-scope tests. New cases cover original setup,
  root/pool configuration isolation, pending package drain, cleanup after closure
  or fencing, asynchronous release failures, credential changes, malformed/late
  transport behavior, revocation and immediate allocation-pressure reporting.
- 52 Node native lifecycle/generation/transport tests with forced GC enabled,
  including both channels using the same strict byte validator.
- 40 optional native schema-asset/HTTP tests, preserving source provenance,
  native admission, unknown activation-outcome refusal and cleanup.

Also passed:

- Full CLI Chromium verifier after canonical protocol extraction, including
  explicit raw-worker dependency delivery, commands, callbacks, progress,
  cancellation, artifacts and owned invocation recovery.
- Default Studio native build/output verifier: 218 declared assets and 313 files,
  with matching native-emitted source/output byte lengths and digests.
- Optional Studio native build and isolated production-service-worker offline
  verifier, including the existing command/owned invocation package handoffs.
  Its `renderingPackageHandoffs.worker` and `.fallback` reports each record two
  consecutive handoffs across three distinct generations, ordinary native CSV
  import and template compilation, two-peer coverage, credential refusal,
  original handoff bounds, stale aliases and old-revocation isolation. Fallback
  additionally checks complete original embedding leases. Both factories and all
  205 pack assets remain paired; default/profile cache isolation is preserved.
- Independent offline schema-loader/declarative-demand verifier, including
  missing, incomplete and invalid deployment refusal.
- Clean `cem-elements` package import and asset verification: the package contains
  both new protocol artifacts and retains its package-owned native factories.
- CLI/shared runtime TypeScript builds, scoped ESLint (no errors; existing
  non-null-assertion warnings remain) and `git diff --check`.

The rendering probes exercise original host-binding transactions; they do not
claim full Studio workspace demand integration. Browser fixtures use isolated
storage and do not modify the user's saved projects, drafts or browser session.
Generated reports, screenshots and Vitest attachments are excluded from Git.

## Next boundaries

Wire explicit shared package demand across Studio's complete command/render
recovery groups, preserving entered-password/OTP refusal and disabled automatic
whole-Studio recovery. Then implement optional embedded-body exclusion while
retaining the default complete release and measure download, cold latency and
allocator/linear-memory headroom. This adapter work makes no binary, memory or
checkpoint-savings claim.
