import type { CemModuleUrlContextWire } from './internal/runtime-support/module-url-resolution.js';
import type { CemProcessingDiagnostic, CemProcessingStylesheetResult } from './internal/runtime-support/processing-host.js';
import type { CemStylesheetInstallationOptions } from './internal/runtime-support/stylesheet-installation.js';

/** Initialized native bindings. Sources/control JSON and emitted CSS are named wire boundaries. */
export interface EdgeStylesheetNativeBindings {
    adoptDomStylesheets(source: string): string;
    adoptInstanceStylesheets(source: string, instanceId: string): string;
    beginTemplateStylesheet(owner: number, options: string): string;
    deliverTemplateStylesheet(owner: number, load: number, consumer: string, request: number,
        bytes: Uint8Array, finalUrl: string, contentType: string): string;
    failTemplateStylesheet(owner: number, load: number, consumer: string, request: number, message: string): string;
    disposeTemplate(owner: number): boolean;
}

type Ready = Extract<CemProcessingStylesheetResult<number>, { status: 'ready' }>;
type Progress = Exclude<CemProcessingStylesheetResult<number>, { status: 'released' }>
    | { status: 'error'; message: string; diagnostics: CemProcessingDiagnostic[] };
export interface EdgeStylesheetOutput {
    styles: Array<{ index: number; css: string; identity: Ready['identity'] }>;
    diagnostics: CemProcessingDiagnostic[];
}

export interface EdgeStylesheetLoadOptions {
    native: EdgeStylesheetNativeBindings;
    owner: { kind: 'instance'; identity: string } | { kind: 'declaration'; identity: string; tag: string };
    sources: Array<{ css: string; scope: string | null }>;
    baseUrl: string;
    context: CemModuleUrlContextWire;
    signal: AbortSignal;
    read: CemStylesheetInstallationOptions['read'];
}

/** Compile a request-owned native batch; no native handle survives this call. */
export async function loadEdgeStylesheets(options: EdgeStylesheetLoadOptions): Promise<EdgeStylesheetOutput> {
    const { native, read, signal } = options;
    signal.throwIfAborted();
    const { owner, sources, baseUrl, context } = structuredClone({
        owner: options.owner, sources: options.sources, baseUrl: options.baseUrl, context: options.context,
    });
    const adoption = JSON.parse(owner.kind === 'instance'
        ? native.adoptInstanceStylesheets(JSON.stringify(sources), owner.identity)
        : native.adoptDomStylesheets(JSON.stringify(sources))) as {
            artifactId?: number; stylesheets?: Array<{ scope: string | null }>; diagnostics?: CemProcessingDiagnostic[];
        };
    const output: EdgeStylesheetOutput = { styles: [], diagnostics: adoption.diagnostics ?? [] };
    const artifact = adoption.artifactId;
    if (!Number.isSafeInteger(artifact) || (artifact ?? 0) < 1) {
        if (output.diagnostics.length) return output;
        throw new Error('native stylesheet adoption did not retain an owner');
    }
    const handle = artifact as number;
    const consumer = `edge:${owner.identity}`;
    try {
        for (const [index, source] of (adoption.stylesheets ?? []).entries()) {
            signal.throwIfAborted();
            const scope = owner.kind === 'instance' ? { kind: 'instance' }
                : source.scope === null ? { kind: 'private', tag: owner.tag } : { kind: 'shared', name: source.scope };
            let progress = JSON.parse(native.beginTemplateStylesheet(handle, JSON.stringify({
                consumer, index, declarationIdentity: owner.identity, scope, baseUrl, context,
            }))) as Progress;
            while (progress.status === 'pending') {
                const pending = progress;
                let response: Awaited<ReturnType<typeof read>>;
                try {
                    response = await abortableRead(() => read(pending.request, signal), signal);
                } catch (error) {
                    signal.throwIfAborted();
                    progress = JSON.parse(native.failTemplateStylesheet(handle, pending.loadId, consumer,
                        pending.request.id, error instanceof Error ? error.message : String(error))) as Progress;
                    continue;
                }
                signal.throwIfAborted();
                progress = JSON.parse(native.deliverTemplateStylesheet(handle, pending.loadId, consumer,
                    pending.request.id, new Uint8Array(response.bytes), response.finalUrl, response.contentType ?? '')) as Progress;
            }
            signal.throwIfAborted();
            output.diagnostics.push(...progress.diagnostics);
            if (progress.status === 'ready') {
                output.styles.push({ index, css: progress.css, identity: progress.identity });
            } else if (!progress.diagnostics.length) {
                throw new Error(progress.message);
            }
        }
        return output;
    } finally {
        native.disposeTemplate(handle);
    }
}

async function abortableRead<T>(read: () => Promise<T>, signal: AbortSignal): Promise<T> {
    signal.throwIfAborted();
    let cancel: () => void = () => undefined;
    const aborted = new Promise<never>((_, reject) => {
        cancel = () => reject(signal.reason);
        signal.addEventListener('abort', cancel, { once: true });
    });
    try {
        return await Promise.race([read(), aborted]);
    } finally {
        signal.removeEventListener('abort', cancel);
    }
}
