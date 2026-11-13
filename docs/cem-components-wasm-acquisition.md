# Full-runtime WASM acquisition diagnosis

The two full-WASM transfers in worker mode come from two initialized realms:
source declaration preparation on the page and retained compilation/rendering
in the processing worker. Each realm reuses its own cached instance. Fallback
performs both roles in one realm and transfers the binary once.

This diagnoses the existing source-component path. Runtime loading and native
ownership are unchanged by this work. The isolated selected-entry measurements
remain separate.

## Evidence and reproduction

```bash
yarn nx run cem_ql:measure:browser-startup -- --trace-wasm --runs=2 --write-doc
```

The committed [per-run evidence](cem-components-wasm-acquisition.json) records
12 fresh-browser cases: controls, JSON/CSV data and editor, worker/fallback,
two repetitions each. It includes artifact/served-source/harness hashes, page
request initiators, original worker resource entries, successful worker operation
counts and readiness-gated native heap/resource inventories. The full local
capture is `dist/reports/components-wasm/browser-wasm-acquisition.json`.
The existing [startup distributions](cem-components-wasm-startup.md) and their
archives are preserved. Diagnostic capture uses the debugger and must not be
used as a latency comparison. Worker call stacks are unavailable; worker
resource entries and native inventories are observed directly, while its
compile/render role is correlated with the original source and operation counts.

The binary is 24,476,684 bytes, SHA-256
`3fc8939ca8eef749b01e9a582dc642cf098ee7d7b0ef61275036f9d6d11e2f76`.
Every case uses one unique binary URL/hash. Worker cases have two transfers,
one live worker and two initialized unshared heaps; fallback has one transfer,
no worker and one initialized heap. No worker failures or retries were observed.
The local no-store policy makes the extra decoded WASM body cost 24,476,684 bytes
(23.3 MiB) per worker case. This is not a claim about cached/CDN wire delivery.

## Original callers and owners

The page request's captured asynchronous initiator chain is:

```text
installCemElementRuntime
  → registerDeclaration / registerResolvedDeclaration
  → prepareRetainedStylesheets
  → surfaceDeclarationDiagnostics
  → compileCemMlTemplateOperation
  → ensureRuntimeReady / initializeNativeRuntime
  → shared deployment initialize / wasm-bindgen __wbg_init
  → fetch(cem_ql_bg.wasm)
```

[`surfaceDeclarationDiagnostics`](../packages/cem-elements/src/lib/cem-elements.ts)
compiles source to obtain declaration diagnostics, stylesheet metadata, the
module map and legacy-resource classification. It installs declaration styles
and marks their metadata ready. `prepareRetainedStylesheets` subsequently asks
the processing host to compile a retained stylesheet/template artifact with the
original host bindings, registration/resolver/scope identities and module
closure. Simply deleting the first call loses those outputs and is not a
verified optimization.

The original [`processing-worker`](../packages/cem-elements/src/lib/internal/runtime-support/processing-worker.ts)
routes successful compile/render/stylesheet requests through
[`CemProcessingEngine`](../packages/cem-elements/src/lib/internal/runtime-support/processing-engine.ts).
Its native bindings initialize the same binary in the worker realm and retain
the template owners used by later jobs. Both the shared deployment initialization
promise and wasm-bindgen's cached exports belong to their importing realm;
sharing one URL/file does not make those per-realm instances identical.

Both repetitions have the following live-template census after first use:

| Workload | Worker mode: page / worker | Fallback mode: page |
| --- | --- | ---: |
| Controls | 0 / 5 | 5 |
| Data | 0 / 3 | 3 |
| Editor | 0 / 3 | 3 |

The page also reports
discardable template-cache entries from declaration preparation. Every inspected
heap returns the same instance/memory object on repeated cached initialization
and has an unshared `ArrayBuffer`. Main and worker inventories are independent.
Zero blocking resources in these page snapshots is not permission to retire a
general application heap; the original retirement/recovery contract still applies.

Inspection first probes a native version export without initializing WASM. If exports are
not initialized it refuses instead of calling an initializer. Tests verify that
refusal, repeated worker inspection, original page/worker transfers and worker
survival after debugger detachment. Each actual case also checks that inspection
causes no WASM acquisition and that the realm ledger matches the independently
observed transfers. The 16 collector/audit tests run serially so concurrent test
browsers cannot distort the touched-memory calibration through shared-page PSS.

## Next implementation boundary

Deduplicate immutable code acquisition/compilation while retaining independent
`WebAssembly.Instance`/memory, native handle registries, source/style leases and
physical recovery participants. The existing generated owned factory already
accepts a compiled module at initialization; its default module cache remains
local to a realm, and the current legacy path also initializes independently.
A host-to-worker compiled-code handoff needs explicit version/hash/feature
admission and tests for concurrent initialization, failed loads, worker loss,
fresh-heap recovery and no mutation replay. Do not share native instances or
live handles as a shortcut.

Separately, investigate reusing processing-host compilation metadata for the
page's declaration preparation, preserving diagnostics, module/style resolution
and all other main-thread native demands. Removing its duplicate source compile
does not by itself prove that a page heap is unnecessary. After implementing a
bounded change, rerun the ordinary uninstrumented startup/network measurements;
all-capabilities/offline and isolated npm-consumer totals remain later gates.
