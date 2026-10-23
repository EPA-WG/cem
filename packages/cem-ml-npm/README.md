# `@epa-wg/cem-ml`

Low-level browser and Node WebAssembly deployment of the common Rust `cem_ml`
engine and its composed native CEM-QL adapters. One linked WASM binary retains
shared native owners across both query languages. This package intentionally owns no command parser, filesystem policy,
npm executable, or UI state.

Browser code imports the generated web loader:

```js
import init, { version } from '@epa-wg/cem-ml/wasm';

await init();
console.log(version());
```

Node hosts import the generated Node loader from the same subpath:

```js
import * as cemMl from '@epa-wg/cem-ml/wasm';

console.log(cemMl.version());
```

## Owned browser generations

`@epa-wg/cem-ml/wasm/factory` exports `createCemMlWasmGeneration()`. Each factory
owns its generated bindings, native memory, cached views and finalizers. It can
initialize from the adjacent WASM asset or explicit initialization bytes. Native
retirement preparation retains the original RAII permit; abort releases that
permit, while disposal checks the original current-owner callback before clearing
all memory/module/instance references. Native closure and census revisions still
apply. This is a low-level host API, with the same unmanaged boundary as raw
WASM exports. Browser command consumers should use the managed
[owned invocation binding](https://github.com/EPA-WG/cem/blob/develop/packages/cem-ml-cli-npm/README.md#recovery-group-enrollment),
which adds bounded admission, complete embedding ownership, group allocation and
explicit stable host rebinding.

## Static CEM-ML HTML rendering

`renderCemMlToHtmlV1(requestJson)` exposes the common Rust parser and light-DOM
renderer to small browser components and richer preview hosts such as CEM
Studio. The versioned JSON request contains `source` and an optional
`sourceUrl`. Its response contains:

- `status`: `rendered`, `invalid`, or `error`;
- escaped/static `html` produced by the implementation interpreter;
- structured `diagnostics`; and
- `outputSpans` mapping generated UTF-8 byte ranges back through source-map
  stacks.

```js
const result = JSON.parse(cemMl.renderCemMlToHtmlV1(JSON.stringify({
    source: '{article @class=message | {strong | Hello}}',
    sourceUrl: 'memory:example.cem',
})));

if (result.status === 'rendered') preview.replaceChildren();
if (result.status === 'rendered') preview.innerHTML = result.html;
```

The API renders trusted CEM-ML. Hosts remain responsible for deciding whether
the resulting live markup may be injected and must treat any status other than
`rendered` as non-renderable.

## Bounded transform configuration authoring

`authorTransformConfigV1(requestJson)` describes or edits CEM-ML graph source
using the native AST and graph validator. A request contains `source` and an
optional `edit: { node, attribute, value }`. A null value removes an optional
attribute; empty text remains a literal. The response contains canonical `source`,
`nodes` with field metadata, and the validated `graph`, or an `error` string.
Description and unchanged edits retain exact source bytes. Changed edits use
the native canonical formatter and are validated before returning.

The bounded API supports scalar attributes on named nodes, at most 100 nodes and
1 MiB of UTF-8 source. IDs, topology creation, parameter maps, alias spellings and
expressions remain source-only. Literals containing both quote delimiters are
rejected. It does not resolve dependencies, execute graphs or persist edits.
Browser callers initialize the WASM loader before calling it, as with other APIs.

## Lossless source highlighting

`highlightSourceToHtmlV1(requestJson)` formats authored HTML and CEM-ML as
escaped semantic HTML without executing it or changing its source bytes. The
request contains `source`, `contentType`, and an optional `sourceUrl`. The
response includes `html`, parser `diagnostics`, and byte-addressed `spans`.
Roles such as `syntax.punctuation`, `syntax.name`, `syntax.attribute`,
`syntax.string`, and `syntax.comment` use the same formatter/colorizer contract
as native CEM-ML output.

```js
const source = '<button type="button">Save</button>';
const result = JSON.parse(cemMl.highlightSourceToHtmlV1(JSON.stringify({
    source,
    contentType: 'text/html',
})));

if (result.status === 'highlighted') {
  code.classList.add('cem-source-code');
  code.innerHTML = result.html;
}
```

The `html` fragment carries no per-token classes or attributes. It uses `b` for
tag/name, `var` for attribute/variable, `strong` for keyword, `i` for string,
`u` for number, `small` for comment, `samp` for source text, and `mark` for
error. Punctuation and raw source remain unwrapped. The parallel `spans` array
retains the exact semantic roles and byte ranges for non-HTML consumers.

The generated `executeCommandServiceV1` binding is the low-level asynchronous
bridge used by the universal CLI package. It accepts the canonical JSON
`CommandServiceRequestV1`, a common capability request, and five host callbacks:
current revision, resource read, transactional write preparation, commit, and
rollback. Callback request/response models remain Rust-owned JSON; publication
bytes are passed separately as a `Uint8Array`. Applications should normally use
the typed adapters from `@epa-wg/cem-ml-cli` rather than call this raw boundary.

The browser and Node declarations re-export the same generated command-service
types. `CommandServiceRequestV1`, `CommandServiceResultV1`, operation unions,
capability callback payloads, progress/control acknowledgements, and artifact
metadata are derived from their serde-annotated Rust declarations during every
package build. `CommandServiceHostCapabilitiesV1` describes typed adapter
callbacks, while the `*JsonCallbackV1` aliases describe the raw generated WASM
boundary. This keeps both runtime targets and the native wire contract on one
declaration source of truth.

Hosts may supply an optional progress callback to
`executeCommandServiceV1`; it receives monotonic Rust-owned lifecycle JSON.
`cancelCommandServiceV1(requestId, reason?)` cooperatively cancels the matching
active request and returns an idempotent control acknowledgement. Active request
ids are unique per runtime instance and are released on every terminal or early
failure path. Progress callbacks are observational and cannot change command
semantics.

The optional final `host_configuration_json` argument supplies trusted embedding
setup for that execution. Its generated `CommandHostConfigurationV1` contract
accepts `schemaPackageReplacementGrants`:

```js
const hostConfiguration = {
  schemaPackageReplacementGrants: [{
    packageId: 'vendor',
    expectedOrigin: { kind: 'manifest', uri: 'vendor://old/package.cem' },
    replacementManifestUri: 'vendor://new/package.cem',
  }],
};
// Pass JSON.stringify(hostConfiguration) after the optional progress callback.
```

Origins are exactly `builtin`, `untracked`, or `manifest` with its current URI.
Unknown fields/kinds, positional arrays and empty identities fail with
`cem.command.host_configuration_invalid` before host callbacks run. Omission
supplies no grants; setup is confined to the execution's fresh `EngineContext`.
Grants neither load manifests nor bypass the shared engine's exact ownership
checks. Command requests and run configs cannot create this authority. The
Node/browser worker clients forward their constructor-owned `hostConfiguration`
snapshot separately from command requests; caller mutation after startup does
not alter that snapshot.

Committed artifact handles remain request-scoped inside Rust.
`readCommandArtifactV1(requestId, handleId, offset, maxBytes)` returns a plain
record with canonical metadata JSON in `json` and an owned `Uint8Array` copy in
`bytes`; it never exposes a WASM memory view. Chunk sizes are bounded by the
runtime transfer limit. `disposeCommandArtifactV1` releases one handle, while
`disposeCommandArtifactsV1` releases the request's remaining handles; both are
idempotent. Reusing a request id invalidates its prior artifact generation before
the new command performs host I/O.

`runtime.json` describes the package ABI and common capability projections.
`integrity.json` records SHA-256 hashes for every generated runtime and
schema-package asset. Schema assets are addressable below
`@epa-wg/cem-ml/schema-packages/`.

The Nx `package` target also emits a version-qualified release-evidence set for
the fixed `cem-ml-platform` family: the npm tarball, capability and integrity
manifests, SPDX 2.3 SBOM, provenance, checksums, signing state, and one release
index entry. The target validates that set before it succeeds; aggregate
staging remains blocked until both npm deployments and all three native
deployments identify the same Cargo version and source commit.

The uncached Nx `sign` target records unsigned-local state by default. In a
protected release job, provide `CEM_ML_GITHUB_ATTESTATION_BUNDLE` (and set
`CEM_ML_RELEASE_SIGNING=required`) so it verifies the packed tarball with `gh`,
copies the bundle, and marks the signing record publication-ready.

## Optional native schema asset packs (prototype)

The default package retains its complete embedded schema catalog. An opt-in
prototype emits an immutable catalog index and original source bodies addressed
by SHA-256, outside the default package output:

```sh
node packages/cem-ml-npm/scripts/build-schema-assets.mjs
node packages/cem-ml-npm/scripts/build-schema-assets-wasm.mjs
node packages/cem-ml-npm/scripts/verify-schema-assets.mjs --wasm=packages/cem-ml-npm/dist-schema-assets-wasm/wasm/node/cem_ml.js
```

Run these from the repository root after the ordinary package build, using the
repository Rust/Node toolchains and matching `wasm-bindgen` CLI. The emitter
writes `dist-schema-assets/`; the optional `schema-asset-packs` Cargo profile
writes `dist-schema-assets-wasm/`. Both are separate from `dist/` and excluded
from the default npm tarball. Deploy every URL in the pack's `integrity.json`
for disconnected use. That inventory is a complete delivery list, not a trust
anchor: use the native-compiled bootstrap pin to authenticate `index.json`.
After intentionally changing catalog sources, regenerate the pack and copy its
native-generated `bootstrap.json` to
`packages/cem_ml/schema/schema-assets-bootstrap-v1.json` before rebuilding the
optional profile. Native fixtures verify that committed pin against the catalog.

Only the optional profile exposes `schemaAssetPackBootstrapV1`,
`schemaAssetPackSummaryV1` and `schemaAssetPackImportV1`. These synchronous probes
accept bounded byte arrays, validate the native-pinned index, import original
sources natively and return scalar metadata. Each call drops its temporary store
and native owners; reimport in another physical WASM instance creates fresh
owners. Normal native admission and memory budgets apply. These exports provide
no checkpoint, retirement or credential authority.

The native per-consumer store bounds index bytes, source count, individual and
aggregate resident bytes. It checks package/version/path, MIME, UTF-8, byte length
and digest, and supports explicit resolver policy/cancellation. Missing/offline
or invalid inputs refuse without embedded fallback. The authoritative native
conversion reader likewise propagates pack failures; legacy reader behavior
remains available for existing consumers. JavaScript transports bytes and reads
named catalog/control metadata; source interpretation remains native.

This prototype does **not** remove embedded bodies, change Studio's production
resource map or enable automatic recovery. It therefore claims no WASM size,
resident-memory or checkpoint savings. Native consumer ownership must migrate
before embedded bodies can become optional. The verifier records raw/gzip
delivery, cold import time, allocator observations and linear-memory capacity
separately under `dist/reports/cem-ml-npm/`. See the
[Studio loading design](https://github.com/EPA-WG/cem/blob/develop/docs/cem-studio.md#checkpoint-capacity-and-optional-module-loading).

## Unmanaged embedding boundary

This package exposes raw generated WASM. Calling these exports, retaining native
reference handles, reinitializing a module or instantiating it directly does not
enroll that heap in the managed `cem-elements` rendering or `cem-ml-cli` command
checkpoint domains. Native memory budgets still apply, but idle command/render
adapters are not proof of complete raw-heap ownership. Embedders must explicitly
account for every client/resource, or keep raw heaps outside replacement.
Managed command checkpoint witnesses always refuse replacement authority.


## Native resource inventory

`commandNativeResourceInventory()` returns explicit counter-only control JSON
for all 29 registries in the linked command/reference runtime. Live resources,
callbacks, commands (including before first poll) and reservations block an
inventory-backed acknowledgement. Rebuildable caches and terminal journals have
separate discardable counts. Native revisions detect same-count mutations.
`commandNativeMemoryIdentity()` exposes the actual `WebAssembly.Memory` identity
for trusted in-memory ownership bookkeeping; neither export grants replacement.

Use the shared CLI embedding-lease APIs for complete opt-in owner consent.
Unregistered raw callers remain unmanaged. The coordinated closure API below
adds native admission and fresh census validation; generation replacement and
semantic resume are still required before replacing a heap. See the [ownership contract](https://github.com/EPA-WG/cem/blob/develop/docs/committed-checkpoints.md#separate-command-heaps-and-the-unmanaged-boundary).


## Closing native admission

`closeCommandNativeAdmission()` permanently closes ordinary entrypoints in that
linked memory. Raw aliases follow the same native gate. Already admitted async
commands retain their entry permits before the first poll and through callback
waits. Disposal, cancellation, existing resource settlement, cache release and
bounded checkpoint controls remain available and advance the inventory revision.
The inventory now includes the native entry count as its twenty-ninth root.

`validateCommandNativeClosure(expectedRevision)` refuses open/busy admission,
live resources or stale/incomplete inventories. It checks a freshly collected
inventory and the exact revision under the native admission lock and returns
bounded control JSON. Busy entrypoints are refused before collection to avoid
reentrant borrowing. No JavaScript callback runs during collection/validation.

Use the CLI's coordinated ownership APIs to bind confirmation to complete current
original embedding leases. A scalar native record is never ownership or permission
to replace a heap; initialization and unregistered/direct-memory embedding remain
unsupported replacement boundaries. The [checkpoint contract](https://github.com/EPA-WG/cem/blob/develop/docs/committed-checkpoints.md#separate-command-heaps-and-the-unmanaged-boundary)
defines freshness, cleanup and the remaining replacement/resume work.


## Native retirement barrier

`prepareCommandNativeRetirement(expectedRevision)` checks a fresh complete idle
closed inventory under the admission lock. It returns an original generated
`CommandNativeRetirement` object holding a Rust RAII permit. While retained,
ordinary work and cleanup both refuse with `cem.runtime.retirement.pending`.
Its bounded `confirmation()` is evidence only; a copied scalar record cannot
release the barrier. Calling `free()` on the original object aborts preparation,
advances the inventory revision and restores cleanup under closed admission.

The shared CLI worker lifecycle privately holds this object while checking
current original embedding ownership and host checkpoint/journal handoff, then
terminates that exact worker. Do not manually free a permit while its generation
is being retired. Raw callers must opt into the complete ownership contract;
this low-level object alone grants no permission to terminate, replace or resume
a heap. Direct memory mutation and initialization remain unsupported boundaries.

The prepared retirement permit remains visible as one live `native-entries`
root until abort or physical worker termination; a held barrier never produces
a misleading empty resource census.
