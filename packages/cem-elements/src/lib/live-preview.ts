import { CemElementRuntime, type CemElementRuntimeOptions } from './cem-elements.js';
import { createCemDeclarationScope, type CemDeclarationScope } from './declaration-scope.js';

export interface CemLivePreviewResource { uri: string; contentType: string; text: string }
/** Explicit transport envelope. Resource bodies stay original text until shared native import. */
export interface CemLivePreviewResources {
    resources: readonly CemLivePreviewResource[];
    declarations?: readonly { tag: string; uri: string }[];
    styles?: readonly string[];
}
export interface CemLivePreviewOptions extends CemLivePreviewResources {
    source: string;
    sourceUri: string;
    onError?: (message: string) => void;
    /** Trusted parent setup, outside the authored resource transport envelope. */
    runtimeOptions?: Pick<CemElementRuntimeOptions, 'checkpointPublication' | 'recoveryScope' | 'processingWorkerFactory' | 'processingPoolPolicy'> & { dispose?: () => void };
}
export interface CemLivePreview {
    readonly ready: Promise<void>;
    dispose(): void;
}
const blank = `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'none'; style-src 'unsafe-inline'; base-uri 'none'; form-action 'none'; frame-src 'none'; connect-src 'none'; object-src 'none'"><meta name="referrer" content="no-referrer"><title>Live CEM preview</title></head><body></body></html>`;
const encoder = new TextEncoder();
const mounts = new WeakMap<HTMLIFrameElement, CemLivePreview>();

/** Parent-owned runtime, script-free frame, fresh registry/scope and explicit resources per mount. */
export function mountCemLivePreview(frame: HTMLIFrameElement, options: CemLivePreviewOptions): CemLivePreview {
    const dispose = options.runtimeOptions?.dispose;
    try { return mountPreview(frame, options); }
    catch (error) { dispose?.(); throw error; }
}
function mountPreview(frame: HTMLIFrameElement, options: CemLivePreviewOptions): CemLivePreview {
    if (!frame.isConnected) throw new Error('cem.preview.detached_frame');
    mounts.get(frame)?.dispose();
    // Keep original source bytes and callbacks outside WASM across retirement.
    // A caller cannot retarget a mounted preview by mutating its setup object.
    options = { ...options, declarations: options.declarations?.map(value => ({ ...value })),
        styles: options.styles ? [...options.styles] : undefined,
        runtimeOptions: options.runtimeOptions ? { ...options.runtimeOptions } : undefined };
    const sourceUri = new URL(options.sourceUri).href;
    if (encoder.encode(options.source).length > 256 * 1024 || options.resources.length > 128 || (options.declarations?.length ?? 0) > 128 || (options.styles?.length ?? 0) > 128) throw new RangeError('cem.preview.limit');
    const resources = new Map<string, CemLivePreviewResource>();
    let size = encoder.encode(options.source).length;
    for (const resource of options.resources) {
        const uri = new URL(resource.uri).href;
        size += encoder.encode(resource.text).length;
        if (size > 16 * 1024 * 1024 || resources.has(uri) || uri === sourceUri) throw new RangeError('cem.preview.resource_set');
        resources.set(uri, { ...resource, uri });
    }
    const read = (uri: string) => {
        const resource = resources.get(uri);
        if (!resource) throw new Error(`cem.preview.resource_denied: ${uri}`);
        return resource;
    };
    let active = true;
    let scope: CemDeclarationScope | undefined;
    let previewDocument: Document | undefined;
    let resize: ResizeObserver | undefined;
    let cancelLoad: (() => void) | undefined;
    // Reserve membership before frame loading yields. A preparing group must
    // not omit a child whose native runtime has not finished starting yet.
    const pendingEnrollment = options.runtimeOptions?.recoveryScope?.enroll({ id: `cem-preview-loading:${crypto.randomUUID()}`,
        kind: 'live-preview', prepare: async () => { throw Error('cem.preview.recovery.pending'); } });
    const current = () => { if (!active || !frame.isConnected) throw new Error('cem.preview.disposed'); };
    const ready = (async () => {
        frame.setAttribute('sandbox', 'allow-same-origin');
        frame.setAttribute('referrerpolicy', 'no-referrer');
        frame.setAttribute('allow', '');
        await new Promise<void>((resolve, reject) => {
            const loaded = () => { frame.removeEventListener('load', loaded); cancelLoad = undefined; resolve(); };
            cancelLoad = () => { frame.removeEventListener('load', loaded); reject(new Error('cem.preview.disposed')); };
            frame.addEventListener('load', loaded);
            frame.srcdoc = blank;
        });
        current();
        const document = frame.contentDocument, window = frame.contentWindow;
        if (!document || !window) throw new Error('cem.preview.document_unavailable');
        previewDocument = document;
        scope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({
            ...options.runtimeOptions,
            declarationScope: scope, restrictedPreview: true,
            logger: { warn() { /* Availability warnings remain on the native resource. */ }, error(message: string) { if (active) options.onError?.(message); } },
            moduleUrlRoot: { baseUrl: sourceUri, importMap: { imports: {} } },
            loadSrcDocument: async uri => {
                current();
                const resource = read(uri);
                return { resolvedUrl: uri, resolverIdentity: 'cem-live-preview-resources',
                    contentType: resource.contentType, body: stream(resource.text) };
            },
            resolveResourceUrl: request => { const uri = new URL(request.authoredUrl, request.baseUrl).href; read(uri); return uri; },
            loadHttpResource: async request => {
                current(); request.signal.throwIfAborted();
                const resource = read(request.resolvedUrl);
                return { response: { url: resource.uri, status: 200, statusText: 'OK', ok: true, redirected: false,
                    headers: { 'content-type': resource.contentType }, contentType: resource.contentType }, body: stream(resource.text) };
            },
            retainedStylesheets: { read: async () => { throw new Error('cem.preview.stylesheet_request_denied'); } },
        });
        runtime.install(window as Window & typeof globalThis);
        // Forms still dispatch local CEM events. No document navigation is admitted.
        document.addEventListener('submit', event => event.preventDefault(), true);
        for (const uri of options.styles ?? []) {
            const resource = read(uri);
            if (resource.contentType !== 'text/css') throw new Error('cem.preview.style_type');
            const style = document.createElement('style'); style.textContent = resource.text; document.head.append(style);
        }
        for (const { tag, uri } of options.declarations ?? []) {
            current();
            const declaration = document.createElement('cem-element');
            declaration.setAttribute('tag', tag); declaration.setAttribute('src', uri); document.head.append(declaration); runtime.registerDeclaration(declaration);
            await runtime.prepareDeclarationStylesheets(declaration);
            check(runtime, declaration);
        }
        current();
        // An authored template input boundary, not an external-data DOM parser.
        const wrapper = document.createElement('cem-element'); wrapper.setAttribute('tag', 'cem-live-view');
        const template = document.createElement('template'); template.id = 'cem-live-view'; template.setAttribute('type', 'text/cem-ml');
        template.content.append(document.createTextNode(options.source)); wrapper.append(template);
        resources.set(sourceUri, { uri: sourceUri, contentType: 'application/xhtml+xml', text: wrapper.outerHTML });
        const declaration = document.createElement('cem-element'); declaration.setAttribute('tag', 'cem-live-view');
        declaration.setAttribute('src', `${sourceUri}#cem-live-view`); document.head.append(declaration); runtime.registerDeclaration(declaration);
        await runtime.prepareDeclarationStylesheets(declaration); check(runtime, declaration); current();
        const view = document.createElement('cem-live-view'); document.body.append(view);
        resize = new ResizeObserver(() => {
            if (!active || !frame.isConnected) return;
            const bodyStyle = window.getComputedStyle(document.body);
            frame.height = String(Math.min(4096, Math.max(1, Math.ceil(document.body.getBoundingClientRect().height
                + parseFloat(bodyStyle.marginTop) + parseFloat(bodyStyle.marginBottom)))));
        });
        resize.observe(document.body);
        const visited = new Set<HTMLElement>();
        for (;;) {
            current();
            const child = [...document.body.querySelectorAll<HTMLElement>('*')].find(node => !visited.has(node) && runtime.hasDeclarationFor(node));
            if (!child) break;
            visited.add(child); await runtime.whenRenderSettled(child); check(runtime, child);
        }
        pendingEnrollment?.remove();
    })();
    const controller = Object.freeze({ ready, dispose() {
        if (!active) return;
        active = false; cancelLoad?.(); cancelLoad = undefined;
        pendingEnrollment?.remove();
        resize?.disconnect(); previewDocument?.body.replaceChildren(); scope?.dispose(); resources.clear();
        options.runtimeOptions?.dispose?.();
        if (mounts.get(frame) === controller) { mounts.delete(frame); frame.removeAttribute('srcdoc'); frame.removeAttribute('height'); }
    } });
    mounts.set(frame, controller);
    // Consumers may dispose before awaiting readiness.
    void ready.catch(() => undefined);
    return controller;
}
async function* stream(text: string): AsyncIterable<Uint8Array> { yield encoder.encode(text); }
function check(runtime: CemElementRuntime, element: HTMLElement): void {
    const errors = runtime.diagnosticsFor(element).filter(diagnostic => diagnostic.severity === 'error' || diagnostic.severity === 'fatal');
    if (errors.length) throw new Error(errors.map(diagnostic => `${diagnostic.code}: ${diagnostic.message}`).join('\n'));
}
