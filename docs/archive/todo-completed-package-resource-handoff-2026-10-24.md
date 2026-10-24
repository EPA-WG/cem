# Completed: shared package resource handoff and owned invocation adapter

- [x] Add original resource-handoff fixtures for generation groups and the shared
  schema-pack binding: checked release, fresh compilation, complete binding,
  stale/credential/policy refusal, offline dependency delivery and revocation.
- [x] Implement the shared package-owner adapter and its first concrete owned
  browser invocation transport, including consecutive offline handoffs.

The shared generation registry attaches original resources to an existing
physical participant. It captures source witnesses when constructing a complete
group, rejects unknown/copied participants and changed membership, and requires
checked resource release before recording successful native retirement. A weak
physical identity map binds both source and target to their actual memory
without retaining retired heaps. Busy capture leaves participants available for
another complete preparation after work settles.

`CemSchemaAssetPackRecoveryBinding` provides stable HTTP/source routes. Replay
bytes remain bounded outside WASM. Exact-owner cleanup runs inside the owned
invocation host's existing current-checkpoint/complete-embedding confirmation;
its source loader then drops all old native endpoint and memory references.
The native empty-inventory closure and original retirement permit remain
mandatory. Missing/stale handoff, incomplete ownership and credential/source
witness changes refuse release. Explicit host shutdown can discard resources
but does not produce a retirement receipt.

The complete binding claim includes resource preparation before host/semantic
owners. Fresh package owners authenticate and recompile the original bytes on
the exact new heap under the original trusted grants. All routes stage inactive
and bind together. Failure, deadline expiry or revocation fences and cleans the
whole fresh group. `resumeNativeGenerationBindings` handles complete host-only
groups; mixed rendering transactions use the existing
`resumeCemCheckpointOwners` path. The legacy invocation `resume` refuses attached
resources. Native controls and normalized transport policy are captured so
subsequent changes to caller-owned objects cannot alter restoration or cleanup.

The concrete owned invocation adapter exposes only a restricted package ABI,
fences saved aliases, closes ordinary admission before cleanup, and rechecks
embedding and checkpoint witnesses. Resource routes re-enroll on the fresh
participant, allowing the next coordinated handoff without preserving the
previous heap. Expected cleanup does not count as new authoring activity.

## Verification

Passed with repository Node/Rust-generated runtime outputs:

- 48 Node host lifecycle/native generation tests, including forced-GC identity
  release, resource target matching, release/refusal, retry after busy capture,
  failed staging and late-preparation cleanup.
- 38 optional native schema-asset/HTTP tests: actual package compilation,
  independent heaps, original source provenance, credential/stale-source refusal,
  native method/policy snapshots and exact native owner cleanup.
- 27 shared checkpoint-resume/recovery-scope tests.
- Full CLI Chromium verifier: owned invocation recovery plus worker-pool control,
  dedicated-worker commands, callbacks, cancellation, artifacts and cleanup.
- Studio default native build/output verification, optional native profile build
  and its isolated offline Chromium verifier. Its `packageHandoff` evidence
  records two consecutive handoffs across three different runtime generations,
  credential refusal, complete ownership, required checkpoint handoff and its
  existing 1 MiB command limit, partial
  legacy resume refusal and stale-alias fencing. Normal authoring output stays
  identical and ordinary package demand remains available after each handoff.
- Shared service-worker offline fixture: all 205 pack assets, explicit delivery
  of the loader's lifecycle dependencies, declarative demand, fresh reimport,
  cleanup and invalid/missing/incomplete source/inventory refusal.
- TypeScript through the CLI and shared runtime builds, scoped ESLint and
  `git diff --check`.

Focused Nx builds/verification used `--excludeTaskDependencies` after preparing
current prerequisites, with the Nx cache disabled to inspect freshly emitted
assets. Generated machine-readable profile evidence is at
`dist/reports/cem-studio/schema-assets-profile.json`. Browser contexts are
isolated; user projects, drafts and caches were untouched.

## Remaining work

The shared contract and owned invocation consumer are complete. Command-service
worker transport, rendering worker/fallback adapters, and Studio's complete
shared demand routing remain unfinished. Packages activated on an invocation
heap never silently supply a worker's native context. Each transport must own,
release and restore its packages under its existing original proofs before
Studio can adopt demand throughout the workspace.

Keep these integrations ahead of embedded-body exclusion and measurements. The
default complete release and embedded sources remain intact. This change adds
no UI JavaScript, checkpoint/status native handles or source bodies, or automatic
whole-Studio recovery. Entered passwords and one-time codes still block recovery
until explicitly cleared. The 4 MiB workspace checkpoint budget, per-loader
32 MiB source residency and five-second generation handoff deadlines remain.
