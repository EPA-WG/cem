import type { CommandNativeEmbeddingOwnership } from './native-embedding.js';
import type { CommandNativeDrain } from './native-admission.js';
import { closeCommandNativeEmbeddingAdmission, revalidateCommandNativeEmbedding, claimCommandNativeEmbedding, checkpointCommandNativeEmbedding } from './managed-native-runtime.js';
import { commandNativeAdmission, commandNativeRuntime as runtime } from './managed-native-runtime.js';
import type {
    CommandInvocationV1,
    CommandPresentationPlanV1,
    CommandPresentationV1,
    CommandServiceResultV1,
} from '@epa-wg/cem-ml/wasm';

import type { ParsedCemMlCommand } from './command.js';
import type { CemNativeRecoveryScope, CemNativeRecoveryEnrollment } from './generation-lifecycle.js';

/** The shared singleton is a real additional heap. It has managed census and
 * ownership controls but no owned fresh-generation allocator. Enroll it as an
 * explicit blocker instead of silently recovering only the command worker. */
export function enrollBrowserCommandInvocationRecovery(scope: Pick<CemNativeRecoveryScope, 'enroll'>, id: string): CemNativeRecoveryEnrollment {
    return scope.enroll({ id, kind: 'command-invocation', prepare: async () => {
        throw Error('cem.command.recovery.invocation-singleton-unsupported');
    } });
}
const initializeRuntime = (runtime as unknown as { readonly default: () => Promise<unknown> }).default;
let runtimeInitialization: Promise<unknown> | undefined;

export { BrowserCommandInvocationError } from './invocation-core.js';
export type { BrowserCommandInvocationOptions, BrowserCommandInvocationResolver, BrowserCommandInvocationResource } from './invocation-core.js';
import { buildCommandInvocationWith, projectCommandPresentationWith } from './invocation-core.js';
import type { BrowserCommandInvocationOptions, BrowserCommandInvocationResolver } from './invocation-core.js';

/** Legacy singleton aliases stay bound to their original heap. */
export async function createBrowserTransformConfigAuthor(): Promise<(requestJson: string) => string> {
    await initializeCommandRuntime(); return runtime.authorTransformConfigV1;
}
export async function buildBrowserCommandInvocation(parsed: ParsedCemMlCommand, resolver: BrowserCommandInvocationResolver,
    options: BrowserCommandInvocationOptions = {}): Promise<CommandInvocationV1> {
    await initializeCommandRuntime(); return buildCommandInvocationWith(runtime, commandNativeAdmission, parsed, resolver, options);
}
export function projectBrowserCommandPresentation(plan: CommandPresentationPlanV1, result: CommandServiceResultV1): CommandPresentationV1 {
    return projectCommandPresentationWith(runtime, plan, result);
}
async function initializeCommandRuntime(): Promise<void> {
    runtimeInitialization ??= initializeRuntime(); await runtimeInitialization;
}

/** Explicit ownership of the main-thread authoring heap, separate from command workers. */
export async function claimBrowserCommandNativeEmbedding(): Promise<CommandNativeEmbeddingOwnership> {
    await initializeCommandRuntime(); return claimCommandNativeEmbedding();
}
export async function checkpointBrowserCommandNativeEmbedding(
    leases: readonly CommandNativeEmbeddingOwnership[], timeoutMs = 5000,
): Promise<CommandNativeDrain> {
    await initializeCommandRuntime(); return checkpointCommandNativeEmbedding(leases, timeoutMs);
}
export {
    releaseCommandNativeEmbedding as releaseBrowserCommandNativeEmbedding,
    isCommandNativeInventoryCurrent as isBrowserCommandNativeInventoryCurrent,
} from './managed-native-runtime.js';


export async function closeBrowserCommandNativeAdmission(leases: readonly CommandNativeEmbeddingOwnership[], timeoutMs = 5000): Promise<CommandNativeDrain> {
    await initializeCommandRuntime(); return closeCommandNativeEmbeddingAdmission(leases, timeoutMs);
}
export async function revalidateBrowserCommandNativeCheckpoint(proof: CommandNativeDrain, leases: readonly CommandNativeEmbeddingOwnership[], timeoutMs = 5000): Promise<CommandNativeDrain> {
    await initializeCommandRuntime(); return revalidateCommandNativeEmbedding(proof, leases, timeoutMs);
}
