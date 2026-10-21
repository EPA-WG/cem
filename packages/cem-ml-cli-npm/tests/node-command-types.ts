import {
    buildNodeCommandInvocation,
    createNodeCommandHost,
    createNodeCommandService,
    createNodeCommandServiceClient,
    closeNodeCommandNativeAdmission,
    revalidateNodeCommandNativeCheckpoint,
    claimNodeCommandNativeEmbedding,
    checkpointNodeCommandNativeEmbedding,
    releaseNodeCommandNativeEmbedding,
    isNodeCommandNativeInventoryCurrent,
    parseCemMlCommand,
    projectNodeCommandPresentation,
    type CommandInvocationV1,
    type CommandNativeEmbeddingOwnership,
    type CommandNativeInventory,
    type CommandNativeClosureValidation,
    type CommandPresentationV1,
    type CommandServiceResultV1,
    type NodeCommandHostOptions,
    type NodeCommandServiceExecuteOptions,
    type ParsedCemMlCommand,
} from '../dist/node.js';

const bytes = new TextEncoder().encode('{root}');
const writable = {
    write: (_chunk: Uint8Array): boolean => true,
    once: (_event: 'drain', listener: () => void): void => listener(),
};
const hostOptions = {
    cwd: '/workspace',
    readStreams: {
        'stream:input': (async function* () {
            yield bytes;
        })(),
    },
    writeStreams: { 'stream:output': writable },
    deferStreamCommits: true,
} satisfies NodeCommandHostOptions;

async function compilePublicNodeCommandContract(): Promise<void> {
    const parsed: ParsedCemMlCommand = parseCemMlCommand(['version'], { runtime: 'wasm-node' });
    const host = createNodeCommandHost(hostOptions);
    const invocation: CommandInvocationV1 = await buildNodeCommandInvocation(parsed, host);
    const executeOptions = {
        signal: new AbortController().signal,
        onProgress: (event) => void event.sequence,
    } satisfies NodeCommandServiceExecuteOptions;
    const client = await createNodeCommandServiceClient({ host, hostConfiguration: {
        schemaPackageReplacementGrants: [{ packageId: 'vendor',
            expectedOrigin: { kind: 'manifest', uri: 'vendor://old/package.cem' },
            replacementManifestUri: 'vendor://new/package.cem' }],
    } });
    const handle = client.execute(invocation.request, executeOptions);
    const result: CommandServiceResultV1 = await handle;
    const presentation: CommandPresentationV1 = projectNodeCommandPresentation(
        invocation.presentation,
        result,
    );
    await handle.dispose();
    const embedding: CommandNativeEmbeddingOwnership = client.claimNativeEmbedding();
    const nativeCheckpoint = await client.checkpointNative(5000, [embedding]);
    const inventory: CommandNativeInventory | undefined = nativeCheckpoint.nativeInventory;
    const blockers: number | undefined = inventory?.blockingResources;
    const coordinated = await client.closeNativeAdmission([embedding], 5000);
    const confirmed = await client.revalidateNativeCheckpoint(coordinated, [embedding], 5000);
    const closure: CommandNativeClosureValidation | undefined = confirmed.nativeClosure;
    void closure;
    const handoff: import('../dist/native-embedding.js').CommandNativeRetirementHandoff = {
        current: () => true, retain: () => new Uint8Array([1]),
    };
    const retired: import('../dist/native-embedding.js').CommandNativeRetiredGeneration =
        await client.retireNativeGeneration(confirmed, [embedding], handoff, 5000);
    const checkpointBytes: Uint8Array = retired.checkpoint;
    const noReplacement: false = retired.replacementAllowed;
    void [checkpointBytes, noReplacement];


    const mainEmbedding = await claimNodeCommandNativeEmbedding();
    const mainProof = await checkpointNodeCommandNativeEmbedding([mainEmbedding]);
    const mainCurrent: boolean = isNodeCommandNativeInventoryCurrent(mainProof);
    const mainClosed = await closeNodeCommandNativeAdmission([mainEmbedding], 5000);
    await revalidateNodeCommandNativeCheckpoint(mainClosed, [mainEmbedding], 5000);
    releaseNodeCommandNativeEmbedding(mainEmbedding);
    void [blockers, mainCurrent];
    const scope: 'managed-cli-only' = nativeCheckpoint.scope;
    const replacementAllowed: false = nativeCheckpoint.replacementAllowed;
    const current: boolean = client.isNativeCheckpointCurrent(nativeCheckpoint);
    void [scope, replacementAllowed, current];
    await client.close();

    const service = await createNodeCommandService({ host, hostConfiguration: { schemaPackageReplacementGrants: [] } });
    const run = await service.run(parsed, {}, executeOptions);
    const runResult = await run.handle;
    const published: CommandPresentationV1 = await service.publish(run.invocation, runResult);
    await service.close();
    void [presentation, published];
}

void compilePublicNodeCommandContract;
