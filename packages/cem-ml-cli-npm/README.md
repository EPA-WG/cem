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

The browser export also provides `createBrowserHtmlBaseHost(document, options)`
for a bounded, exclusively controlled UTF-8 HTTP(S) execution document. It consumes
a retained native execution-snapshot handle, compares the complete live document
topology and keeps native source/DOM slot identity through WASM. `capture(handle,
policy)` explicitly reassigns the first base href to confirm a fresh freeze;
`read()` synchronously revalidates the evidence. Route every document, history and
policy write through `changeContext(() => { ... })`, then supply the resulting
native snapshot for a new capture. Unexpected DOM changes and unsupported browser
observations revoke the session; call `dispose()` when finished.

This API requires `contextOwnership: 'exclusive'`, an explicit `profile`
(`standard-agreement` or `chromium-148`) and `{ engine, version }` browser identity.
It does not infer historical freezes or observe unreported history/policy changes,
parse HTML into a JavaScript AST, or grant resource/edit admission. See the
[execution-host contract](../../docs/cem-studio.md#exclusive-html-base-execution-host)
for topology limits, policy ownership and measured compatibility rejections.

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

Configuration is snapshotted before worker startup. Later mutation of the
caller's object, request fields and execute options cannot replace the snapshot.
Each execution passes it separately to the Rust-owned host configuration decoder;
invalid setup rejects before host callbacks with
`cem.command.host_configuration_invalid`. Omitting setup supplies no grants.
Grants do not load manifests; the common engine still checks exact ownership.
A new client takes a new snapshot. Worker failure closes the existing client;
there is no implicit restart or authority change.

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


### Native stylesheet observations

After `createBrowserHtmlBaseHost` and a successful native HTML `capture`, use
`html.captureStylesheets(responses, { ownership: 'exclusive' })` or
`pool.captureStylesheets(html, responses, { ownership: 'exclusive', signal })`.
Each response supplies `requestedUrl`, `finalUrl`, `revision`, `contentType` and
UTF-8 `text` from the immutable acquisition snapshot. The adapter verifies actual
CSSOM/source correspondence and import order; capture itself does not fetch or install CSS.

The returned host supports `read()`, synchronous `change(() => { ... })`, fresh
`capture()` and `dispose()`. All CSSOM/selection/environment writes must use the
exclusive change boundary; DOM/base/history changes use the HTML host's
`changeContext`. Pool close, failure and generation replacement revoke attached
sessions. Summaries contain occurrence counts and remaining blockers, not an
edit grant or transferable native evidence.

The bounded HTML profile admits style elements/attributes and loaded external
sheets with readable CSSOM. For inactive titled/alternate sets supply
`titledSets: 'disabled'`; subsequent `host.disableTitled()` invalidates and requires
fresh `capture()`. For active external titled sets, use
`await html.selectStylesheets(nativeDocument, policy, responses, 'Night',
{ ownership: 'exclusive', signal, timeoutMs: 10000 })`, or
`pool.selectStylesheets(html, nativeDocument, policy, responses, 'Night', options)`.
An empty title disables all titled sets. The native title plan preserves persistent
and authored-disabled sheets; selected inline titled styles remain unsupported.
The host invalidates before explicitly reloading selected links, waits for trusted
loads, then recaptures HTML and verifies actual installations against the same native
source and policy. Chromium's `disabled=false` alone cannot prove activation.
Pending reads/changes reject. Load errors, timeout, abort, DOM/source/policy drift
and lifetime termination revoke the transition. Repeated selections create fresh
evidence; later observed CSS/environment changes require another selection.
Actually unloaded imports are admitted
only under inactive selection/conditions, with all source snapshots still required.
Active unloaded imports, unloaded roots and shadow/adopted sheets reject capture.

For compiled native CEM output, pass the QL WASM realm's opaque
`new CompiledStylesheetSession(artifact, consumer, '[0]')` and ordered installed
style nodes to `captureBrowserCompiledStylesheets(document, native, nodes, options)`
or `pool.captureCompiledStylesheets(document, native, nodes, options)`. These
factories own and dispose the session, including on failure. The same
`read/change/capture/dispose` lifecycle applies. Consumer release/replacement,
source or CSSOM mismatch, and installation replacement invalidate evidence.
Compiled imports form one real installation per output stylesheet. Inner
conditions, whole-output correspondence and runtime loads retain their blockers;
worker result strings cannot replace the native consumer lease. Recorded-body
compilation binding is also available through the native QL API. Native
`RecordedBodyStylesheetInstallations::compose_output(...)` creates a new retained
whole output with ordered declaration styles; `RecordedTemplateOutput::capture`
and its sealed evidence join complete HTML correspondence with those exact
installations. The native `RecordedPreparationWorker` retains this joined record
behind realm-local handles with bounded capacity, generation replacement,
release/close/drop and cancellation guards. Every read revalidates the live record
and returns diagnostic blockers; failed reads permanently revoke the handle.
XML/HTML exports remain delivery artifacts. Use the QL WASM
`RecordedOutputSession(requestJson, slot, generation)` with
`captureBrowserRecordedOutput(document, native, install, options)` or
`pool.captureRecordedOutput(...)` for whole-output preparation. The host owns the
session and performs original/composed HTML delivery through a synchronous
`install(html, stage)` callback, then validates the complete DOM and native inline
stylesheet installations. Options require exclusive ownership and a stamped base
policy, with optional cancellation. `read()` returns diagnostics; `change(write)`
revokes before writes and requires a fresh session. Pool failure, replacement and
closure revoke all dependent preparations. Native owners stay in the synchronous
DOM observation realm; worker result messages contain no evidence authority.
Context-sensitive stylesheet choices require explicit `hostPaths` into the native
output; native composition validates produced-host scope/island membership and
adds the resolver-context marker. Browser code does not patch semantic attributes.
The async factories `captureBrowserRecordedOutputAsync(...)` and
`pool.captureRecordedOutputAsync(...)` wait for external stylesheet loads before
publishing evidence. Supply the complete acquired CSS snapshot, an optional
`stylesheetTitle` (exact external title, or empty to disable titles), and
`timeoutMs` (1–30000, default 5000). The HTML installer remains synchronous.
Native selection commands, fresh link installations, acquired/imported CSSOM,
outer media and complete DOM ownership are verified before capture. Active inline
titled sheets, shadow/adopted sheets and unreadable CSSOM remain unsupported.
Changing an existing title/stylesheet preparation revokes it and requires a fresh
native session. Media changes, load failure, timeout, abort and pool generation
changes fail closed; runtime resource closure and atomic admission stay blocked.
See the [full observation contract](../../docs/cem-studio.md#shared-browser-stylesheet-observation-host)
and [compiled installation contract](../../docs/cem-studio.md#compiled-template-and-body-stylesheet-installations).


The whole-output async host also records responsive `img`/`picture` selected
URLs after decoding, then reacquires distinct same-origin raster sources with
a shared 4 MiB streaming limit. Native records retain the opaque bytes, SHA-256
revision and request/final metadata, and validate selection on every read.
`read().imageAcquisitions` reports the diagnostic count. Source/DOM/media drift,
timeout, abort and pool replacement revoke the record. Redirects, custom request
policies, SVG and cross-origin image acquisition remain outside this profile.
Reacquisition does not prove the browser's original response bytes; that identity
remains required before dependency closure or transaction admission. Native plans
retain authored candidate occurrences; selection rejects when multiple occurrences
resolve to currentSrc, including src/srcset and picture collisions. This bounded
profile intentionally does not guess the winner from dimensions or source order.
Original-response proof requires transport observation before initial delivery;
post-load reacquisition cannot supply it.


For opt-in pre-delivery byte capture, install
`installImageDeliveryTransport(self)` from
`@epa-wg/cem-ml-cli/image-delivery-worker` in your application's service worker.
The application owns registration, activation and scope; the library never
replaces a worker. Once that worker controls the target document, pass
`imageDelivery: 'service-worker'` to async recorded-output capture.

The worker intercepts that client's same-origin raster image requests, buffers
at most 4 MiB across 256 URLs, and sends byte ingress over a private MessagePort.
It waits for native acceptance before returning the same body to the browser.
Native code computes SHA-256, retains bytes under the execution cancellation
signal, and rejects conflicting repeated bodies. Repeated intercepted requests
for one URL share the first body within the session. Selection joins only
received bodies; it performs no second fetch in this mode. Missing receipts,
redirects, failures, unsupported MIME and cancellation reject capture. Sealing
closes ingress; further intercepted image loads revoke the preparation.
`read().svgPaintRoots` and `read().svgPaintLinks` count native paint-server
candidates and template edges retained by the preparation session. Reads validate
the owning document/base/style read set; target or href mutations and lifetime
changes revoke the plan. Missing/ambiguous targets, cycles, base uncertainty and
untraversed URLs remain explicit blockers. Counts do not prove rendering or
resource acquisition, and the graph never crosses the native boundary.
`read().svgPaintServers` and `read().svgPaintContentNodes` count distinct local
servers and retained source-node memberships (bounded at 256 and 4096). Content
plans share preparation ownership: stop, pattern-descendant and ancestor-style
mutations revoke them, including changes restored before a read. Cloned-content
style provenance, effective content selection and rendered resource evidence
remain explicit blockers.
`read().svgPaintStyleHosts` and `read().svgPaintStyleInstances` count native
SVG 2 provenance contexts and prospective cloned nodes, bounded at 256 and 4096.
They share the preparation lifetime. Chromium currently differs from the
structural profile for inherited colors and descriptive-only patterns; isolated
raster fixtures record that boundary. These counts do not assert browser clone
identity, instance computed style or rendered-resource closure.

Optional `cloneProfile: { name: 'chromium-148-solid-currentcolor-v1', observe }`
selects the exact Chromium profile explicitly. Collect its six raster probes
before capture; `observe(index)` synchronously returns twelve integer channels:
source RGBA, host RGBA, pixel RGBA. Probe order is own/inherited/descriptive
gradient, then own/inherited/descriptive pattern. Native validation rejects
unknown identities, mismatches and malformed reports. Both capture paths publish
`svgCloneProfileConfirmed` only after successful preparation, and revoke it
with the preparation lifetime. It confirms a compatibility report, never
instance-style or execution evidence; the compatibility/evaluation blockers stay.

Set `boundedPaintColors: true` with a confirmed `cloneProfile` to request
the strict native presentation-only evaluator. Unsupported content/cascade
rejects capture. The host validates derived source `fill`/`stop-color` against
live CSSOM at capture and around reads; mutations or mismatches revoke it.
`svgPaintColors` and `svgPaintSuppressedHosts` are bounded diagnostic counts.
Native declaration/inheritance provenance stays retained, and clone rendering,
winning-source and resource-evidence requirements remain separate.

`read().svgComputedSlots` counts retained SVG presentation attributes whose
computed values were captured and revalidated by the native session. Both capture
paths support static values, including cascade overrides. Active animations and
SMIL are rejected for this profile; owner/style/input-state changes revoke the
record. Use `host.change` before controlled writes, including changes restored to
their original value. These observations do not identify winning declarations or
consumed resource bytes, and do not grant transaction admission.

`read().imageDeliveries` counts selected URLs with pre-delivery receipts.

This is evidence of intercepted responses, not complete image-cache history.
Browser image-cache reuse can bypass a service worker, and a same-URL receipt
alone cannot identify every cache-satisfied consumer. Consequently the original
response-identity blocker remains, along with the broader dependency/admission
blockers. Native seals never travel through worker messages. The transport is
bounded to 32 concurrent client sessions with a 30-second preparation deadline.
Controller replacement, document/DOM changes, abort and pool disposal revoke
native execution evidence and close the private transport.


Use `imageDelivery: 'service-worker-replay'` when the caller explicitly permits
new delivery URLs. The controlling worker generates a fresh random namespace;
native render-plan composition replaces only image candidate URL spans and keeps
the authored tree, candidate ordinals and descriptors. The worker receives the
native alias-to-source routes before composed HTML is installed. Every selected
alias must have a native receipt before publication. Alias responses are immutable
within that execution, so reuse of that alias identifies the same captured body.
Duplicate authored URLs have distinct candidate aliases. Stale and foreign
reserved routes fail without a network fallback.

This profile uses the reserved same-origin `/__cem_image_delivery/` path, accepts
at most 256 native routes, and removes only the image response-identity blocker
once selected receipts join. Other dependency and transaction blockers remain.
It is a new execution, **not transparent historical capture**: reflected src and
currentSrc values, URL-based CSS selectors and cache-dependent source selection
can differ. Authored source URLs and provenance remain native. Normal delivery
and post-load observation retain their existing identity limitations.
