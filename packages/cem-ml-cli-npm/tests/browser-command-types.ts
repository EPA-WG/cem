import {
    buildBrowserCommandInvocation,
    createBrowserCommandServiceClient,
    parseCemMlCommand,
    projectBrowserCommandPresentation,
    type BrowserCommandArtifactReadOptions,
    type BrowserCommandServiceExecuteOptions,
    type CommandArtifactHandleV1,
    type CommandInvocationV1,
    type CommandPresentationV1,
    type CommandResolvedResourceV1,
    type CommandResolvedWriteV1,
    type CommandRevisionLedgerV1,
    type CommandServiceArtifactDisposeAckV1,
    type CommandServiceArtifactReadV1,
    type CommandServiceControlAckV1,
    type CommandServiceHostCapabilitiesV1,
    type CommandServiceProgressV1,
    type CommandServiceRequestV1,
    type CommandServiceResultV1,
} from '../dist/browser.js';

const bytes = new TextEncoder().encode('{root}');
const host = {
    currentRevision: async ({ project }): Promise<CommandRevisionLedgerV1> => ({
        project,
        resourceVersions: {},
    }),
    readResource: async ({ expected }): Promise<CommandResolvedResourceV1> => ({
        version: expected,
        bytes: [0, 1, 2],
        identity: null,
    }),
    prepareWrite: async ({ requestId }, bytes) => ({
        token: `${requestId}:${bytes.byteLength}`,
    }),
    commitWrite: async (token): Promise<CommandResolvedWriteV1> => ({ uri: `memory:${token}` }),
    rollbackWrite: async (_token): Promise<void> => undefined,
} satisfies CommandServiceHostCapabilitiesV1;

const request = {
    protocolVersion: 1,
    requestId: 'typed-browser-command',
    project: { projectId: 'typed-project', revision: 1 },
    resourceVersions: {},
    operation: { kind: 'version-capabilities' },
    runPlan: null,
    resources: {},
    policyStamp: {
        resolver: 'typed-resolver',
        safety: 'typed-safety',
        budget: 'typed-budget',
    },
} satisfies CommandServiceRequestV1;

const progress: CommandServiceProgressV1[] = [];
const executeOptions = {
    signal: new AbortController().signal,
    onProgress: (event) => progress.push(event),
} satisfies BrowserCommandServiceExecuteOptions;
const readOptions = { offset: 0, maxBytes: 1024 } satisfies BrowserCommandArtifactReadOptions;

async function compilePublicBrowserCommandContract(): Promise<void> {
    const parsed = parseCemMlCommand(['version'], { runtime: 'wasm-browser-worker' });
    const invocation: CommandInvocationV1 = await buildBrowserCommandInvocation(
        parsed,
        async (requirement) => [{ uri: requirement.uri, bytes }],
    );
    const client = await createBrowserCommandServiceClient({ host });
    const handle = client.execute(invocation.request, executeOptions);
    const unsubscribe: () => void = handle.subscribe((event) => progress.push(event));
    const resultFromThen: CommandServiceResultV1 = await handle;
    const result: CommandServiceResultV1 = await handle.result();
    const artifact: CommandArtifactHandleV1 | undefined = result.artifacts.items[0];
    if (artifact !== undefined) {
        const read: { readonly metadata: CommandServiceArtifactReadV1; readonly bytes: Uint8Array } =
            await handle.readArtifact(artifact, readOptions);
        read.bytes[0] = 0;
        const disposed: CommandServiceArtifactDisposeAckV1 = await handle.disposeArtifact(artifact);
        void disposed;
    }
    const cancellation: CommandServiceControlAckV1 = await handle.cancel('typed cancellation');
    const disposedRequest: CommandServiceArtifactDisposeAckV1 = await handle.dispose();
    const presentation: CommandPresentationV1 = projectBrowserCommandPresentation(
        invocation.presentation,
        result,
    );
    const runtime: 'wasm-browser-worker' = client.capability.runtime;
    const workerRuntime: string = client.worker.runtimeInstanceId;
    unsubscribe();
    await client.close();
    void [request, resultFromThen, cancellation, disposedRequest, presentation, runtime, workerRuntime];
}

void compilePublicBrowserCommandContract;
