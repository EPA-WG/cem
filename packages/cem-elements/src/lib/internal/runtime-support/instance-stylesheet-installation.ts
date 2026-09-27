import { installRetainedStylesheets, prepareRetainedStylesheets, type CemPreparedStylesheets, type CemStylesheetInstallation, type CemStylesheetInstallationOptions } from './stylesheet-installation.js';
import { cemProcessingFailureDiagnostics, createCemProcessingTextSource, type CemProcessingDiagnostic } from './processing-host.js';
import { notifyStylesheetLifecycle } from './stylesheet-notifications.js';

export interface InstanceStylesheetInstallation {
    ready: Promise<readonly CemProcessingDiagnostic[]>;
    dispose(): void;
}

/** Own native loads and host-local style nodes independently of rendered content. */
export type InstanceStylesheetInstallationOptions = Pick<CemStylesheetInstallationOptions, 'host' | 'context' | 'baseUrl' | 'read'> & {
    element: HTMLElement;
    instanceId: string;
    sources: Array<{ css: string; scope: null }>;
    artifactId: string;
    scopePolicyStamp: string;
    signal: AbortSignal;
};

type Generation = { dispose(): void };
const generations = new WeakMap<HTMLElement, { active?: Generation; pending?: Generation }>();
const nodeOwners = new WeakMap<HTMLStyleElement, Generation>();

export function installInstanceStylesheets(options: InstanceStylesheetInstallationOptions): InstanceStylesheetInstallation {
    return install(options, false);
}

/** Keep the committed generation (including SSR nodes) until a complete replacement is ready. */
export function stageInstanceStylesheets(options: InstanceStylesheetInstallationOptions): InstanceStylesheetInstallation {
    return install(options, true);
}

export interface PreparedInstanceStylesheets {
    readonly element: HTMLElement;
    ready: Promise<{ status: 'prepared' | 'cancelled'; diagnostics: readonly CemProcessingDiagnostic[] }>;
    check(): boolean;
    isPublished(): boolean;
    /** Cancel an unpublished candidate without disposing an active generation. */
    cancelPreparation(): void;
    commit(): boolean;
    dispose(): void;
}

/** Compile and load without publishing until the caller commits the current candidate. */
export function prepareInstanceStylesheets(options: InstanceStylesheetInstallationOptions): PreparedInstanceStylesheets {
    const candidate = install(options, true, true);
    let loaded = false;
    return {
        element: options.element,
        ready: candidate.ready.then(diagnostics => {
            loaded = candidate.isPrepared();
            return { status: loaded ? 'prepared' : 'cancelled', diagnostics };
        }),
        commit: () => loaded && candidate.commit(),
        check: () => loaded && options.element.isConnected && candidate.isPrepared(),
        isPublished: candidate.isPublished,
        cancelPreparation: candidate.cancelPreparation,
        dispose: candidate.dispose,
    };
}

function install(input: InstanceStylesheetInstallationOptions, staged: boolean, deferred = false):
    InstanceStylesheetInstallation & { commit(): boolean; isPrepared(): boolean; isPublished(): boolean; cancelPreparation(): void } {
    const options = { ...input, ...structuredClone({ sources: input.sources, context: input.context }) };
    const { element, host } = options;
    let state = generations.get(element);
    if (!state) { state = {}; generations.set(element, state); }
    const ownership = state;
    const controller = new AbortController();
    const retainedNodes = Array.from(element.querySelectorAll<HTMLStyleElement>(':scope > style[data-cem-instance-style]'));
    if (!staged) for (const style of retainedNodes) style.remove();
    let nodes: HTMLStyleElement[] = [];
    let releaseNative: (() => void) | undefined;
    let installation: CemStylesheetInstallation | CemPreparedStylesheets | undefined;
    let publish: (() => boolean) | undefined;
    let prepared = false;
    let nativeCurrent = () => !host.ownerScope.disposed;
    let compileJobId: number | undefined;
    const release = () => {
        options.signal.removeEventListener('abort', dispose);
        if (ownership.pending === generation) ownership.pending = undefined;
        if (ownership.active === generation) ownership.active = undefined;
        for (const node of nodes) {
            if (nodeOwners.get(node) === generation) { node.remove(); nodeOwners.delete(node); }
        }
        nodes = [];
        releaseNative?.();
        releaseNative = undefined;
    };
    const dispose = () => {
        if (controller.signal.aborted) return;
        prepared = false;
        publish = undefined;
        controller.abort();
        release();
        options.signal.removeEventListener('abort', dispose);
        if (compileJobId !== undefined) void host.cancel({ targetJobId: compileJobId, reason: 'superseded' }).result.catch(() => undefined);
        void installation?.dispose().catch(() => undefined);
    };
    const generation: Generation = { dispose };
    const previousPending = ownership.pending;
    const previousActive = ownership.active;
    ownership.pending = generation;
    options.signal.addEventListener('abort', dispose, { once: true });
    previousPending?.dispose();
    if (!staged) previousActive?.dispose();
    if (options.signal.aborted) dispose();
    const commit: CemStylesheetInstallationOptions<{ kind: 'instance' }>['lease']['commit'] = (outputs, releaseGeneration) => {
        if (controller.signal.aborted || !element.isConnected || ownership.pending !== generation) {
            releaseGeneration(); return false;
        }
        const previous = ownership.active;
        releaseNative = releaseGeneration;
        const candidates = staged
            ? Array.from(element.querySelectorAll<HTMLStyleElement>(':scope > style[data-cem-instance-style]'))
            : retainedNodes;
        nodes = outputs.map((output, index) => {
            const style = candidates[index] ?? element.ownerDocument.createElement('style');
            nodeOwners.set(style, generation);
            style.setAttribute('data-cem-instance-style', String(output.index));
            if (style.textContent !== output.output.css) style.textContent = output.output.css;
            element.append(style);
            return style;
        });
        for (const node of candidates.slice(outputs.length)) node.remove();
        ownership.active = generation;
        ownership.pending = undefined;
        if (previous) notifyStylesheetLifecycle([() => previous.dispose()]);
        return ownership.active === generation;
    };
    const ready = (async (): Promise<readonly CemProcessingDiagnostic[]> => {
        if (controller.signal.aborted) return [];
        if (!options.sources.length) {
            if (deferred) {
                prepared = true;
                publish = () => commit([], () => undefined);
            } else if (!staged || !commit([], () => undefined)) dispose();
            return [];
        }
        try {
            const job = host.compile({ language: 'css', instanceStylesheetIdentity: options.instanceId,
                producedTag: element.localName, templateArtifactId: options.artifactId,
                registrationIdentity: options.instanceId,
                source: createCemProcessingTextSource(JSON.stringify(options.sources)),
                sourceRef: { kind: 'inline', value: `instance:${options.instanceId}` },
                resolverIdentity: options.context.resolverIdentity, scopePolicyStamp: options.scopePolicyStamp, sourceMapMode: 'dev' });
            compileJobId = job.jobId;
            let cancelWait: () => void = () => undefined;
            const cancelled = new Promise<null>(resolve => {
                cancelWait = () => resolve(null);
                controller.signal.addEventListener('abort', cancelWait, { once: true });
            });
            const compilation = await Promise.race([job.result, cancelled]).finally(() => {
                controller.signal.removeEventListener('abort', cancelWait);
                compileJobId = undefined;
            });
            if (!compilation || controller.signal.aborted) return [];
            if (staged && compilation.diagnostics.some(item => item.severity === 'error' || item.severity === 'fatal')) {
                release();
                return compilation.diagnostics;
            }
            const installationOptions = { ...options, artifact: compilation.artifact, requireComplete: staged,
                consumer: `instance:${options.instanceId}:${crypto.randomUUID()}`,
                occurrences: (compilation.stylesheets ?? []).map((_, index) => ({ index, scope: { kind: 'instance' as const } })),
                lease: { signal: controller.signal, release, commit },
            };
            const pending = deferred ? prepareRetainedStylesheets(installationOptions) : installRetainedStylesheets(installationOptions);
            installation = pending;
            const result = await pending.ready;
            if (result.status === 'prepared' && 'commit' in pending) {
                const entry = pending.takeCommit();
                if (entry) {
                    prepared = true;
                    nativeCurrent = () => entry.isCurrent?.() !== false;
                    publish = () => nativeCurrent() && entry.lease.commit(entry.outputs, entry.release);
                }
            }
            return controller.signal.aborted ? [] : [...compilation.diagnostics, ...result.diagnostics];
        } catch (error) {
            release();
            return controller.signal.aborted ? [] : cemProcessingFailureDiagnostics(error);
        }
    })();
    const isPrepared = () => prepared && !controller.signal.aborted && nativeCurrent() && ownership.pending === generation;
    const isPublished = () => element.isConnected && !controller.signal.aborted && nativeCurrent()
        && ownership.active === generation && !ownership.pending;
    return { ready, dispose, isPrepared, isPublished,
        cancelPreparation() { if (ownership.active !== generation) dispose(); },
        commit() {
            if (!isPrepared() || !publish) {
                if (prepared) dispose();
                return false;
            }
            prepared = false;
            const commitPrepared = publish;
            publish = undefined;
            try {
                if (commitPrepared()) return true;
            } catch (error) {
                dispose();
                throw error;
            }
            dispose();
            return false;
        },
    };
}
