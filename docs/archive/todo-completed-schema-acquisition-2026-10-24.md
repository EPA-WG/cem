# Completed shared schema acquisition — 2026-10-24

- [x] Add a shared byte-transport loader using the native-pinned bootstrap and native source admission. Cover bounded reads, offline/missing/tampered inputs, cancellation, stale runtime refusal and byte-only reimport into a distinct fresh heap; provide an explicit optional deployment inventory without activating automatic recovery or replacing the complete default runtime.
- [x] Verify the optional Studio service-worker inventory with a cold offline browser load and fresh-heap byte reimport; reject escaping inventory paths and incomplete delivery without accessing saved projects or drafts.
- [x] Repair Studio verifiers' obsolete compact shell/positive minimum-height expectations to require the already committed `super-compact` density, content-sized controls and normal spacing; preserve color/override assertions and rerun graph-output and ordinary shell checks without changing the theme or layout.

## Shared service boundary

`CemSchemaAssetPackLoader` uses the same stream-shaped HTTP transport as
declarative resources. The extracted `openCemHttpResource` preserves the public
request/response contract and closes HTTP-error/unused/partially consumed bodies.
Early return never waits for an uncooperative underlying cancellation promise.
The loader returns independent original-byte copies and native scalar provenance;
it does not decode CEM sources in JavaScript or substitute a JSON AST.

The compiled optional runtime bootstrap pins the downloaded index. Native summary
validation precedes catalog DTO use, and native memory/admission/import checks run
on cached accesses too. Default transport omits credentials, denies redirects and
cross-origin resolution, and has a 30-second deadline. It admits at most eight
concurrent loads, a 4 MiB index, 16 MiB entry and 32 MiB retained/pending bytes.
Caller-owned copies, transport chunks, catalog DTOs and native parsing allocations
remain separate; this is not a total process-memory ceiling.

Abort/dispose fences uncooperative resolver/opener/iterator completions and closes
late response bodies. Host currentness and physical memory identity fence runtime
rebinding. Disposal releases source buffers, callbacks and memory references.
`prepareFreshRuntime` revalidates the index and reimports retained bytes on a
distinct initialized heap without transferring native handles or using network
fallback. This is explicit source preparation, not checkpoint or publication
authority, automatic recovery, package activation or runtime retirement.

## Optional offline delivery

`deploymentUrls()` covers all 205 emitted immutable files. Studio's production
worker understands an explicit optional `schema-assets` inventory group, isolates
its cache by index digest, and rejects incomplete delivery and invalid paths.
The normal graph-emitted inventory still omits this group. Sidecar inventories
never replace native bootstrap/source validation.

The isolated browser fixture uses the production worker, generated optional
browser factory and built shared service. It installs the complete pack, reloads
cold offline, imports a source and reimports original bytes on an independent
heap. Page-level offline emulation left worker fetches reachable, so the fixture
also physically disconnects its HTTP transport. A deleted source then refuses
without selecting the embedded catalog. Separate contexts reject escaping
inventory paths and missing deployment files. User projects/drafts are unopened.

The worker also compares the delivery list with the explicit catalog DTO and its
index hash, so a self-consistent sidecar that omits a declared source refuses
installation. This establishes inventory consistency, not native schema trust.
Index/inventory control-body reads are bounded before JSON interpretation.

## Verification

- Twelve native asset-pack tests passed before browser wiring.
- `cem-elements:verify:schema-assets` passed all four Nx tasks. Native emission
  matched all 200 original sources and 205 offline files; default ABI exclusion,
  native admission/memory refusal, cleanup and independent WASM reimport checks
  passed. The optional build also verified the default WASM artifacts unchanged.
- All 28 shared-loader/HTTP-transport tests passed with the real optional native
  ABI, including source/cache admission, cancellation, bounds, stale/rebound
  memory, a zero-byte producer, hostile iterator return and fresh-heap reimport.
- The isolated production-worker browser fixture passed cold offline/fresh-heap
  import and refused missing bodies, escaping inventory entries and a source
  omitted from an otherwise self-consistent inventory.
- The 22-task Studio build and graph-output verifier passed (215 declared assets,
  310 output files). The existing CLI generated-schema preparation issue remains
  separate; verification temporarily supplied the exact native-generated input.
- Four focused HTTP/external-source declaration stories and five theme-storage/
  entered-credential refusal stories passed.
- The ordinary Studio shell gate passed theme inheritance, forced colors and
  offline reload without the optional group. Its old compact/minimum-height
  assertions also failed against committed source: the shell already declares
  super-compact with normal spacing. Expectations now require zero imposed
  minimum height, real content height for visible controls (allowing one measured
  layout subpixel of rounding), and the existing color/override behavior.
- Scoped lint and whitespace checks passed. No application UI behavior, theme
  tokens or layout markup changed in this increment.

## Remaining integration

Persistent native activation and ordinary command/render demand remain open,
followed by shared declarative binding, generation-group enrollment, an opt-in
Studio deployment graph and embedded-body exclusion. Both current runtime
profiles retain complete embedded bodies; no binary, memory or checkpoint savings
are claimed. The 4 MiB checkpoint budget, credential refusal until user clearing
and disabled automatic whole-Studio recovery remain unchanged.
