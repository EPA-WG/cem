import { installRetainedStylesheets, type CemStylesheetInstallation, type CemStylesheetInstallationOptions } from './stylesheet-installation.js';
import { cemProcessingFailureDiagnostics, createCemProcessingTextSource, type CemProcessingDiagnostic } from './processing-host.js';

export interface InstanceStylesheetInstallation {
    ready: Promise<readonly CemProcessingDiagnostic[]>;
    dispose(): void;
}

/** Own native loads and host-local style nodes independently of rendered content. */
export function installInstanceStylesheets(options: Pick<CemStylesheetInstallationOptions, 'host' | 'context' | 'baseUrl' | 'read'> & {
    element: HTMLElement;
    instanceId: string;
    sources: Array<{ css: string; scope: null }>;
    artifactId: string;
    scopePolicyStamp: string;
    signal: AbortSignal;
}): InstanceStylesheetInstallation {
    const { element, host } = options;
    const controller = new AbortController();
    const retainedNodes = Array.from(element.querySelectorAll<HTMLStyleElement>(':scope > style[data-cem-instance-style]'));
    for (const style of retainedNodes) style.remove();
    let nodes: HTMLStyleElement[] = [];
    let releaseNative: (() => void) | undefined;
    let installation: CemStylesheetInstallation | undefined;
    let compileJobId: number | undefined;
    const release = () => {
        for (const node of nodes) node.remove();
        nodes = [];
        releaseNative?.();
        releaseNative = undefined;
    };
    const dispose = () => {
        if (controller.signal.aborted) return;
        controller.abort();
        release();
        options.signal.removeEventListener('abort', dispose);
        if (compileJobId !== undefined) void host.cancel({ targetJobId: compileJobId, reason: 'superseded' }).result.catch(() => undefined);
        void installation?.dispose().catch(() => undefined);
    };
    options.signal.addEventListener('abort', dispose, { once: true });
    if (options.signal.aborted) dispose();
    const ready = (async (): Promise<readonly CemProcessingDiagnostic[]> => {
        if (controller.signal.aborted || !options.sources.length) return [];
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
            installation = installRetainedStylesheets({ ...options, artifact: compilation.artifact,
                consumer: `instance:${options.instanceId}:${crypto.randomUUID()}`,
                occurrences: (compilation.stylesheets ?? []).map((_, index) => ({ index, scope: { kind: 'instance' as const } })),
                lease: { signal: controller.signal, release,
                    commit(outputs, releaseGeneration) {
                        if (controller.signal.aborted || !element.isConnected) { releaseGeneration(); return false; }
                        releaseNative = releaseGeneration;
                        nodes = outputs.map((output, index) => {
                            const style = retainedNodes[index] ?? element.ownerDocument.createElement('style');
                            style.setAttribute('data-cem-instance-style', String(output.index));
                            if (style.textContent !== output.output.css) style.textContent = output.output.css;
                            element.append(style);
                            return style;
                        });
                        return true;
                    },
                },
            });
            const result = await installation.ready;
            return controller.signal.aborted ? [] : [...compilation.diagnostics, ...result.diagnostics];
        } catch (error) {
            return controller.signal.aborted ? [] : cemProcessingFailureDiagnostics(error);
        }
    })();
    return { ready, dispose };
}
