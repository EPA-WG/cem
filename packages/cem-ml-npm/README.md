# `@epa-wg/cem-ml`

Low-level browser and Node WebAssembly deployment of the common Rust `cem_ml`
engine. This package intentionally owns no command parser, filesystem policy,
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

Hosts may supply an optional final progress callback to
`executeCommandServiceV1`; it receives monotonic Rust-owned lifecycle JSON.
`cancelCommandServiceV1(requestId, reason?)` cooperatively cancels the matching
active request and returns an idempotent control acknowledgement. Active request
ids are unique per runtime instance and are released on every terminal or early
failure path. Progress callbacks are observational and cannot change command
semantics.

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
