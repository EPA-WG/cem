# Persistent WASM package activation and declarative demand — 2026-10-24

- [x] Connect the shared optional-pack loader to persistent owned WASM activation
  endpoints and ordinary command/import consumers. Use existing declarative
  resources, native inventory and host generation/admission fences; keep sources
  native and do not serialize owners or turn metrics into retirement authority.
- [x] Add persistent owner fixtures for atomic activation, ordinary command/import
  use, immutable source enrichment, inventory and closed-admission refusal,
  stale owner/generation rejection, bounded declarative demand and cancellation,
  and byte-only reimport/recompilation on a distinct runtime. Preserve credential
  privacy and user projects/drafts.

## Ownership and consumers

`SchemaAssetSession` owns the admitted immutable pack and its published native
`EngineContext`. Source admission uses copy-on-write storage; existing contexts
retain their source owners. Unchanged admission/publication is idempotent by
immutable source identity. Additional bodies can republish a fresh binding;
failed activation leaves the incumbent intact.

The optional WASM ABI exposes `schemaAssetOwnerOpenV1`,
`schemaAssetOwnerAdmitV1`, `schemaAssetOwnerActivateV1` and
`schemaAssetOwnerDisposeV1`. There is one embedding-owned context per physical
heap, with monotonic owner IDs, native-pinned index validation, pre-copy body
length checks, bounded status DTOs and native work-memory admission. Replacement
grants come only from explicit trusted command-host setup, never project/template
resources or demand URLs. Default profiles omit these optional endpoints.

Ordinary command-service and resolver contexts clone the published context.
CSV document/value imports use its native parser contract before projecting
the typed AST through the existing import boundary and document limits. An
unbound parser model refuses rather than silently selecting an embedded model.
No JSON AST or browser-side format interpretation was introduced.

`schema-package-owners` is a nondiscardable native census root. Every new endpoint
has an explicit native admission guard; closure permits exact-owner cleanup but
refuses new stores, source admission and activation. Live roots block retirement.
Async commands independently retain native activity and their cloned context
through callbacks. Same-count mutations invalidate earlier inventory revisions.

## Admission and validation phases

The emitted runtime pack includes manifests, schemas and executable artifacts;
manifest-declared development examples are outside it. Runtime publication is
an explicit declaration/admission and native compilation phase. It checks the
same metadata rules and actual schema readiness without running development
fixtures or eagerly compiling every formatter. Full package `validate`/`check`
retains its existing artifact/example resource checks. Executing a selected
formatter/converter still requires the exact admitted body, with no embedded or
filesystem substitution for missing owned sources. No diagnostic results are
filtered to force readiness.

The native fixture demonstrates both boundaries: a schema-only package can
activate, a missing formatter refuses, admitting its exact body republishes the
source binding, and full package validation still reports unresolved example
resources. Existing unsupported/pending model and missing-artifact assertions
remain unchanged. Full validation's older preference for filesystem reads over
a supplied reader on file-like URIs is recorded separately in the active todo.

## Declarative demand and fresh preparation

`CemSchemaAssetPackLoader.activatePackages` acquires original bodies under the
existing transport bounds and publishes through the owned native API. It copies
bounded selections, serializes at most eight queued demands and fences abort,
disposal, host admission and physical-memory changes. Native trust and exact
replacement grants remain authoritative.

The shared `loadResource` opener recognizes its configured
`demand/<packageId>/<version>` endpoint and returns the named
`cem-schema-package-status/1` JSON resource. An ordinary `http-request` declaration
can gate dependent content on its loaded state. Other URLs retain the ordinary
host transport. There is no new control, component-specific JavaScript or
Studio-local UI behavior.

`prepareFreshRuntime` reimports original retained bytes and recompiles/reactivates
selected packages on a distinct initialized, admitted heap without network reads
or handle transfer. It keeps the old and target owners independent; both remain
inventory roots until explicit cleanup. Preparation neither retires a runtime
nor publishes a generation. The host must bind the loader to the actual runtime
executing its commands/imports; a demand never silently retargets another heap.

## Verification

- Native persistent-session coverage passes, including schema-only admission,
  strict missing formatter reads, source enrichment/republication, unchanged-store
  reuse, pending-parser refusal, CSV import diagnostics and full validation's
  unresolved example checks.
- All 12 existing native activation cases and 60 package-contract regressions
  pass, preserving unsupported/pending-model, strict artifact-source and atomic
  preparation-retirement assertions.
- Materialized document loader (3), document limits (4), raw native admission
  (5) and complete resource inventory (5) integration cases pass.
- Locked workspace checking and the optional WASM release build pass. The
  release verifier admits/reimports all 200 native sources and verifies all 205
  offline files, including independent heaps, memory/admission refusal and the
  absence of optional owner exports from the default ABI. The optional build
  verifies default WASM artifacts remain unchanged.
- All 35 shared-loader/HTTP integration cases pass against the real optional
  native runtime, including raw owner identity/length checks, concurrent demand,
  exact cleanup under closure, command-service validation, generation fences,
  credential-admission fencing and distinct-heap recompilation without network.
- The `cem-elements` Nx build, scoped TypeScript check, scoped ESLint and new
  Rust-file formatting checks pass; whitespace checks pass.
- The browser gate mounts an actual shared `CemElementRuntime` with an ordinary
  declarative `http-request` and observes `loaded` only after native activation.
  The production-worker fixture also passes cold offline demand with transport
  disconnected, independent target recompilation, owner cleanup, strict missing
  source refusal and invalid/incomplete deployment refusal. All browser work
  uses isolated contexts rather than the user's existing session.

## Boundaries

Default complete deployment and embedded source bodies remain intact. Studio's
opt-in deployment profile and automatic package-owner adapters for coordinated
generation publication remain active work. The current owner root deliberately
blocks retirement until released. The aggregate 4 MiB checkpoint budget,
entered-password/one-time-code recovery refusal and disabled automatic
whole-Studio recovery are unchanged. Browser verification uses isolated contexts,
without opening saved projects or drafts. No download, memory or checkpoint
savings are claimed; no generated screenshots or Vitest attachments are added.
