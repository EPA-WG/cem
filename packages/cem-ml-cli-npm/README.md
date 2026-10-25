# `@epa-wg/cem-ml-cli`

Universal host adapters for the common `@epa-wg/cem-ml` WebAssembly runtime.
The current Phase 2.5 slice exposes bounded browser dedicated-worker and Node
worker-thread hosts plus one generated, versioned command grammar shared by
both APIs.

```js
import {
    commandSchema,
    parseCemMlCommand,
    parseCemMlCommandResource,
    parseCemMlCommandText,
    serializeCemMlCommand,
    serializeCemMlCommandResource,
    serializeCemMlCommandText,
} from '@epa-wg/cem-ml-cli/node';

const command = parseCemMlCommand(
    ['query', 'data.xml', '--query', '//item', '--query-content-type', 'application/vnd.cem.xpath'],
    { runtime: 'wasm-node' },
);
console.log(commandSchema.schemaVersion, command);
console.log(serializeCemMlCommand(command));
const text = serializeCemMlCommandText(command);
console.log(text, parseCemMlCommandText(text, { runtime: 'wasm-node' }));
const resource = serializeCemMlCommandResource(command);
console.log(parseCemMlCommandResource(resource, { runtime: 'wasm-node' }));
```

The schema is generated from the built native Clap command graph and joined to
the common Rust runtime-capability matrix. Options, defaults, enum values,
required groups, conflicts, and runtime availability are therefore not copied
into a separate TypeScript flag table. The same parser and serializer are
exported from `./browser` and `./node`.

The authored-resource helpers use
`application/vnd.cem.cli-command+json` with schema
`https://cem.dev/ns/cli/command/1`. The resource stores generated
command-schema/common versions, the `cem-ml` binary identity, and ordered
literal argv. It intentionally excludes lowered requests, effective config,
resolver snapshots, results, and normalized run plans.

The command-text helpers add a literal argv projection for editable command
surfaces. They preserve quoted and empty arguments with canonical,
POSIX-compatible quoting, but never perform variable expansion, substitution,
redirection, pipelines, globbing, or command execution. Parsed options still
flow through the same generated command grammar.

```js
import { createNodeWorkerPool } from '@epa-wg/cem-ml-cli/node';

const source = (uri, contentType, schema, text) => ({
    uri,
    bytes: [...new TextEncoder().encode(text)],
    identity: { contentType, schema },
});
const xmlSource = source(
    'memory:data.xml',
    'application/xml',
    'https://cem.dev/ns/data/xml/1',
    '<root><item id="one"/><item id="two"/></root>',
);
const xpathSource = source(
    'memory:query.xpath',
    'application/vnd.cem.xpath',
    'https://cem.dev/ns/query/xpath/1',
    '//item',
);
const pool = await createNodeWorkerPool({ workerCount: 2 });
console.log(pool.capability, pool.workers);
const operation = pool.run({
    kind: 'query',
    data: xmlSource,
    query: xpathSource,
});
operation.subscribe((event) => console.log(event));
console.log(await operation);
await pool.close();
```

```js
import { createBrowserWorkerPool } from '@epa-wg/cem-ml-cli/browser';

const pool = await createBrowserWorkerPool({ workerCount: 2 });
console.log(pool.mode, pool.capability, pool.workers);
await pool.close();
```

Every worker owns one isolated WASM runtime. Stable slot/generation identities,
strict initialization envelopes, Rust-derived protocol limits, exact runtime
versioning, bounded startup, and explicit one-worker modes are established here.
The browser host falls back from the bounded pool to one dedicated worker and
then to one main-thread WASM runtime when workers cannot initialize; it does not
require shared-memory WASM or cross-origin isolation.

The browser command-service API is a separate strict boundary: it always owns
exactly one dedicated worker and never falls back to pool scheduling or
main-thread execution. Callers provide explicit asynchronous revision, read,
and transactional-write capabilities; URLs and virtual resources have no
implicit filesystem behavior.

```js
import {
    buildBrowserCommandInvocation,
    createBrowserCommandServiceClient,
    parseCemMlCommand,
    projectBrowserCommandPresentation,
} from '@epa-wg/cem-ml-cli/browser';

const parsed = parseCemMlCommand(['parse', 'document.cem'], {
    runtime: 'wasm-browser-worker',
});
const invocation = await buildBrowserCommandInvocation(
    parsed,
    async ({ uri }) => [{ uri, bytes: await readApplicationBytes(uri) }],
    { cwd: '/workspace' },
);

const client = await createBrowserCommandServiceClient({
    host: {
        currentRevision: async ({ project }) => ({ project, resourceVersions: {} }),
        readResource: async (request) => resolveApplicationResource(request),
        prepareWrite: async (request, bytes) => stageApplicationWrite(request, bytes),
        commitWrite: async (token) => commitApplicationWrite(token),
        rollbackWrite: async (token) => rollbackApplicationWrite(token),
    },
});
const operation = client.execute(invocation.request, {
    signal: abortController.signal,
    onProgress: (progress) => console.log(progress),
});
const result = await operation;
const presentation = projectBrowserCommandPresentation(invocation.presentation, result);
consumePresentation(presentation);
for (const artifact of result.artifacts.items) {
    const { metadata, bytes } = await operation.readArtifact(artifact);
    consumeCopiedArtifactChunk(metadata, bytes);
}
await operation.dispose();
await client.close();
```

The request, result, callback, progress, cancellation, and artifact types are
re-exported directly from the Rust-generated `@epa-wg/cem-ml/wasm`
declarations. Pre-terminal command errors reject with
`BrowserCommandServiceError`; canonical terminal statuses remain typed results.

Both command-service clients accept optional constructor-owned
`hostConfiguration: CommandHostConfigurationV1`. `createNodeCommandService`
forwards the same option to its worker client:

```js
const client = await createBrowserCommandServiceClient({
    host,
    hostConfiguration: {
        schemaPackageReplacementGrants: [{
            packageId: 'vendor',
            expectedOrigin: { kind: 'manifest', uri: 'vendor://old/package.cem' },
            replacementManifestUri: 'vendor://new/package.cem',
        }],
    },
});
```

Trusted host setup also accepts optional `rootQueueSize` (a positive integer).
It bounds the operation root; omission retains 64. An authored command budget can
constrain that ceiling but cannot raise it. For a larger bounded query, the host
can admit `rootQueueSize: 1024` and the command can request
`--scope-budget queueSize=1024`. Studio uses that combination for the Login
CEM-QL comparison. Request/config data does not supply host configuration.

Configuration is snapshotted before worker startup. Later mutation of the
caller's object, request fields and execute options cannot replace the snapshot.
Each execution passes it separately to the Rust-owned host configuration decoder;
invalid setup rejects before host callbacks with
`cem.command.host_configuration_invalid`. Omitting setup supplies no grants.
Grants do not load manifests; the common engine still checks exact ownership.
A new client takes a new snapshot. Worker failure closes the existing client;
there is no implicit restart or authority change.

The optional schema-asset profile also exposes the browser client's original
`packageResourceBinding()` for `CemSchemaAssetPackLoader` from
`@epa-wg/cem-elements/schema-assets`. This shared byte transport sends the pinned
index and original source bodies, never native owner IDs or source ASTs. The
worker uses its startup `hostConfiguration` for activation. Command and rendering
workers vendor the same bounded original-byte/control protocol validator. Demand URLs and
loader configuration cannot supply replacement grants. The default complete
WASM profile does not acquire optional packages implicitly.

Use `CemSchemaAssetPackRecoveryBinding` with the same original worker participant
and prepare complete host resume tickets before closure. With attached packages,
`client.closeNativeAdmission(owners, timeout, handoff)` requires the original
complete embedding acknowledgment and a current checkpoint handoff within the
existing 1 MiB per-command budget. It closes raw/managed native admission, awaits
package-owner release and validates the empty native inventory. Cleanup replies
never grant retirement independently; timeouts, unknown work, stale ownership
and late replies refuse. Native package owners remain worker-local, and the
shared complete binding transaction restores packages on the fresh worker before
publishing command routes. Entered password/OTP controls continue to block
recovery; automatic whole-Studio recovery remains disabled. See the
[shared loader contract](../cem-elements/README.md#optional-native-schema-source-loading).

The Node command service adds explicit filesystem, local `file://`, HTTPS, and
application-stream resolution plus prepared file/stream writes. Parsed command
lowering, resource discovery, canonical requests, diagnostics, reports,
terminal presentation, and exit codes remain owned by the common Rust runtime.

```js
import {
    createNodeCommandHost,
    createNodeCommandService,
    parseCemMlCommand,
} from '@epa-wg/cem-ml-cli/node';

const host = createNodeCommandHost({ stdout: process.stdout, stderr: process.stderr });
const service = await createNodeCommandService({ host });
try {
    const command = parseCemMlCommand(['parse', 'document.cem'], { runtime: 'wasm-node' });
    const { invocation, handle } = await service.run(command);
    const result = await handle;
    await service.publish(invocation, result);
    await handle.dispose();
} finally {
    await service.close();
}
```

The package also installs `cem-ml`. It uses one worker-thread runtime, maps
`SIGINT`/`SIGTERM` to cooperative cancellation, publishes stdout/stderr and
report files through the Node host, and preserves the stable common exit policy.

`inspect --format json` keeps its explicit presentation choice across browser,
Node and worker command transport. Omitted format and `--format cem` retain the
normal CEM-ML inspector output. The common native request/preparation owns that
choice and the JSON output identity; hosts do not serialize an intermediate AST
or infer JSON from the source format.

`pool.run(request)` returns an awaitable operation handle with `result()`,
`subscribe()`, `cancel()`, `pause()`, `continue()`, `step()`, and `dispose()`.
The common Rust coordinator retains continuation and deterministic commit order;
workers receive bounded stateless transform/query packets. Dedicated workers are
terminated and replaced when cancellation exceeds the negotiated hard-cancel
grace. Main-thread browser fallback uses the same bounded packets and cooperative
controls, while truthfully reporting hard cancellation as unavailable.

The aggregate package checks run the shared parse, validate, check, inspect,
convert, query, transform, trace, and version/capabilities matrix through the
browser and Node command-service workers and the executable. Packaging then
installs both archives into a clean consumer, proves one resolved common runtime,
and repeats the executable matrix through the installed `cem-ml` bin.

The package build writes its runtime/capability projection and complete
SHA-256 integrity manifest. Its Nx `package` target adds the version-qualified
npm tarball, SPDX 2.3 SBOM, provenance, checksums, signing state, and release
index entry, then validates those artifacts against the common platform
contract before returning success.

The uncached Nx `sign` target verifies and records a supplied
`CEM_ML_GITHUB_ATTESTATION_BUNDLE`; `CEM_ML_RELEASE_SIGNING=required` turns a
missing bundle into a hard release failure. Unsigned development packages
remain explicitly non-publishable.

The `@epa-wg/cem-ml-cli/browser-commands` entry point exposes command parsing,
invocation and dedicated-worker command execution for Studio.
`await createBrowserTransformConfigAuthor()` initializes the same shared runtime
and returns its native `authorTransformConfigV1` JSON function for graph
description and edits. Applications can use the CLI facade without importing
the runtime package directly; validation and source writing remain native.

## Native admission and checkpoint boundary

CLI native calls share an admission adapter for their actual linked module;
command workers, main-thread authoring and rendering keep separate heap coverage.
Soft/hard native pressure or unavailable telemetry permanently closes fresh
command work. Cancellation/disposal remain available, and waiting host callbacks
stay enrolled until completion and terminal publication. Pressure never restarts
or replays a command. Cached graph-author functions check admission on each call.

`await client.checkpointNative()` explicitly closes ordinary command admission
and waits for native work and host callbacks. Retained artifacts and resumable
hosts refuse capture until disposed; release artifacts after copying any bytes
you need. A failed capture leaves cleanup available. The default timeout is five
seconds; positive integer options up to 60 seconds cannot extend the native or
host five-second safety deadlines. Timeout permanently fences the adapter.
`client.isNativeCheckpointCurrent(witness)` accepts only its current original
witness; copying it, later work/cleanup, client closure or worker failure makes
it unusable.

Witnesses have `scope: "managed-cli-only"` and `replacementAllowed: false`.
Raw `@epa-wg/cem-ml/wasm` calls and direct/generated WASM instantiation remain
unmanaged. This API does not authorize heap replacement, preserve arbitrary raw
native handles or replay external actions. See the
[checkpoint contracts](https://github.com/EPA-WG/cem/blob/develop/docs/committed-checkpoints.md#separate-command-heaps-and-the-unmanaged-boundary).


## Native inventories and embedding ownership

For an explicit native census, claim a lease for each embedding owner, then pass
all originals to the checkpoint call:

```js
const owner = client.claimNativeEmbedding();
const witness = await client.checkpointNative(5000, [owner]);
console.log(witness.nativeInventory.blockingResources); // 0 after acknowledgement
client.releaseNativeEmbedding(owner); // invalidates the witness
```

The Rust census includes raw retained resources and pending commands that the
CLI ledger cannot see. Dispose live resources explicitly before retrying;
refusal keeps cleanup available. Copies, foreign leases and partial owner sets
are rejected. Main-thread authoring has equivalent `claimNodeCommandNativeEmbedding`
and `claimBrowserCommandNativeEmbedding` families, keyed by actual WASM memory.

This is snapshot evidence with `replacementAllowed: false`. Worker currentness
covers parent activity/ownership; raw calls after acknowledgement require a fresh
native check. Raw callers must opt in. Inventory-only capture leaves ordinary
generated exports open; coordinated closure below closes them in the native gate.
See the [complete inventory and ownership contract](https://github.com/EPA-WG/cem/blob/develop/docs/committed-checkpoints.md#separate-command-heaps-and-the-unmanaged-boundary).


## Coordinated native closure and revalidation

Inventory-only checkpoints keep their existing behavior. Use coordinated closure
when all embedding owners can consent to stopping new work in that memory:

```js
const owner = client.claimNativeEmbedding();
const closed = await client.closeNativeAdmission([owner]);
const confirmed = await client.revalidateNativeCheckpoint(closed, [owner]);
console.log(confirmed.nativeClosure.closed); // true
```

Closure is permanent for that native generation. Every ordinary generated-WASM
entrypoint checks the native gate, so a raw alias also refuses new work.
Previously admitted commands remain enrolled through callbacks. Cleanup,
cancellation, existing resource settlement and bounded checkpoint controls remain
available. A retained root or raw pending command refuses acknowledgement; settle
or dispose it explicitly, then request a fresh coordinated close witness.

Revalidation requires the current original witness and every original owner lease.
Native validation checks the fresh complete inventory and its exact revision under
one admission lock; the host requires unchanged ownership across that call.
Cleanup invalidates the native revision even when live counts stay zero. A copied,
foreign, timed-out or owner-stale witness cannot be revalidated. Worker synchronous
`isNativeCheckpointCurrent` observes host activity only; use the asynchronous
revalidation endpoint for native freshness.

Main-thread authoring provides `closeNodeCommandNativeAdmission` /
`closeBrowserCommandNativeAdmission` and `revalidateNodeCommandNativeCheckpoint` /
`revalidateBrowserCommandNativeCheckpoint`, with the same original-lease rules.
Witnesses still have `replacementAllowed: false`. Native confirmation describes
one validation instant; explicit retirement below consumes a new native barrier.
Semantic resume still requires new generation authority. Unregistered
embedders, direct memory writes and independent initialization remain outside the
ownership contract. No command or resource writer is replayed.


## Retiring a command worker with checkpoint handoff

Retirement is explicit on browser and Node command-service clients. A trusted host
supplies its current native-sealed checkpoint and terminal external-operation
journal through a lifecycle adapter. For a shared `cem-elements` publication
coordinator, use its `handoff()` directly:

```js
// Capture after all owners publish and pending operations settle or reconcile.
await publications.capture();
const handoff = publications.handoff();
const closed = await client.closeNativeAdmission(allOriginalEmbeddingLeases);
const retired = await client.retireNativeGeneration(
    closed, allOriginalEmbeddingLeases, handoff,
);
// Independent host bytes, including the journal, survive worker termination.
const checkpoint = retired.checkpoint;
```

The adapter's `retain()` returns native-sealed bytes; `current()` must reject
new edits, publications, policy changes and journal changes. This is trusted
host code, never an authored callback, receipt flag or serialized checkpoint
label. The CLI transports opaque bytes, bounded to 1 MiB, and does not parse or
validate an application AST. The host must cover all affected owners and heaps;
a rendering checkpoint does not establish command-heap coverage.

The client checks the original coordinated witness and all original embedding
leases, retains an independent byte copy, then asks the worker to freeze its
exact native inventory revision. Stale raw cleanup cancels preparation. Only the
original native RAII permit can reopen cleanup; its copyable confirmation cannot.
While frozen, every mutating entrypoint, including cleanup, refuses work. Final
handoff and ownership checks and transport termination run in one host turn.
Successful retirement revokes old ownership and returns the retained bytes.

A changed handoff/owner, refusal, timeout or worker loss never starts replacement
or retries an operation. An aborted preparation drops its native permit, restores
cleanup under closed admission and invalidates the previous witness. Unknown
external effects must be reconciled under current policy before handoff. Native
artifact-disposal tombstones are discarded only with the retired heap, rather
than imported into another generation. The existing host journal remains in the
checkpoint; no external writer or command is replayed. Callers retain last-good
bytes after failure, but they cannot authorize rollback of newer edits.

The returned record retains `replacementAllowed: false`. Main-thread memories
are not physically retired by this API. Automatic replacement/resume, rendering
worker retirement and declarative recovery status remain separate work. Raw
initialization, direct memory mutation, unregistered embeddings and direct
transport-control injection remain unsupported retirement boundaries.

The prepared retirement permit remains visible as one live `native-entries`
root until abort or physical worker termination; a held barrier never produces
a misleading empty resource census.

Dedicated command-service workers can join `CemNativeGenerationGroup` through
`client.generationParticipant()` before retirement. Enroll every managed physical
heap in the group, retire each with its original complete embedding ownership and
current checkpoint/terminal-journal handoff, then pass the original retirement
results to `group.allocate(results)`. The returned command runtimes are new
clients; previous clients remain closed. Allocation admits the complete set
only after fresh workers initialize with native memory headroom. Failed or
expired startup fences all candidates and retains independent handoff bytes.
`admission.revoke()` fences the entire set and requests cleanup. Native command
semantics, checkpoints and external actions are not automatically resumed.
See [the coordinated allocation contract](https://github.com/EPA-WG/cem/blob/develop/docs/committed-checkpoints.md#coordinated-fresh-generation-allocation-and-admission).

## Recovery group enrollment

Use the shared `CemNativeRecoveryScope` and
`client.enrollRecovery(scope, id, hostWitness)` to account for command workers
alongside rendering/preview domains.
`createEnrolledBrowserCommandServiceClient(options, scope, id, hostWitness)`
reserves membership before worker startup and returns `{ client, enrollment }`;
failed startup removes the reservation after terminating its worker. Include host ledgers, staged writes and
unknown outcomes in the original witness. Active work and uncertain writes
refuse preparation; native ownership, retained artifacts and closure are still
checked separately. Fresh workers retain the original snapshotted host services,
while old aliases/handles are never retargeted or replayed. Explicitly enroll the
fresh client after coordinated binding.

Use `createBrowserCommandInvocationBinding({ recoveryScope, id, hostWitness })`
for owned invocation discovery, synchronous `authorTransformConfigV1(json)` and
`projectPresentation(plan, result)`. Startup reserves membership before loading
WASM. Each binding owns a lexical wasm-bindgen generation with separate memory,
caches and finalizers; it never initializes or retargets the legacy singleton.
`binding.runtime` exposes the original generation's ownership, closure and
retirement controls. Discovery holds admission across asynchronous resolution;
active discovery or changed host ledgers refuse recovery preparation.

For a complete pure-authoring group, claim original embedding ownership, close
native admission, retain the explicit terminal host handoff, retire, allocate and
call `binding.resume(originalAdmission)`. Only the stable binding changes target;
captured methods on an old `binding.runtime` stay fenced. Resume consumes the
original admission once and refuses copied, foreign or mixed-group admissions.
Revocation fences the binding and disposes the admitted generation. No operation,
resolver, command, write or presentation is replayed during allocation/resume.
For combined rendering/preview/command handoff, both the invocation binding and
`createBrowserCommandServiceBinding(options, { recoveryScope, id, hostWitness })`
expose `prepareResume()` tickets. Capture those before retirement and pass every
original binding ticket to the shared cem-elements owner transaction. It claims
one complete admission and binds all routes before provider renewal; raw aliases
and artifact handles remain attached to their original workers. See
[the combined handoff contract](https://github.com/EPA-WG/cem/blob/develop/docs/committed-checkpoints.md#explicit-runtime-recovery-enrollment).

Legacy `buildBrowserCommandInvocation`, `createBrowserTransformConfigAuthor` and
`projectBrowserCommandPresentation` retain their original singleton behavior.
Consumers retaining those aliases must still call
`enrollBrowserCommandInvocationRecovery(scope, id)` to record its unsupported
allocator boundary. See [the shared enrollment contract](https://github.com/EPA-WG/cem/blob/develop/docs/committed-checkpoints.md#explicit-runtime-recovery-enrollment).
