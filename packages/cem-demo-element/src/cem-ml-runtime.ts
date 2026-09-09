export interface CemMlDiagnostic {
    code: string;
    severity: string;
    message: string;
    uri?: string | null;
    line?: number | null;
    column?: number | null;
    byteOffset?: number | null;
}

export interface CemMlOutputSpan {
    outputRange: {
        start: number;
        len: number;
    };
    origin: {
        frames: unknown[];
    };
}

export interface CemMlHtmlRenderResult {
    schemaVersion: 1;
    status: 'rendered' | 'invalid' | 'error';
    html: string;
    diagnostics: CemMlDiagnostic[];
    outputSpans: CemMlOutputSpan[];
}

export interface CemMlSourceHighlightSpan {
    byteOffset: number;
    byteLength: number;
    role: string;
}

export interface CemMlSourceHighlightResult {
    schemaVersion: 1;
    status: 'highlighted' | 'unsupported' | 'invalid' | 'error';
    html: string;
    diagnostics: CemMlDiagnostic[];
    spans: CemMlSourceHighlightSpan[];
}

interface CemMlWasmRuntime {
    renderCemMlToHtmlV1(request: string): string;
    highlightSourceToHtmlV1(request: string): string;
}

let runtimePromise: Promise<CemMlWasmRuntime> | undefined;

/** Render CEM-ML through the shared Rust/WASM light-DOM implementation. */
export async function renderCemMlSource(
    source: string,
    sourceUrl?: string
): Promise<CemMlHtmlRenderResult> {
    const runtime = await loadRuntime();
    const response = JSON.parse(runtime.renderCemMlToHtmlV1(JSON.stringify({ source, sourceUrl })));
    if (!isRenderResult(response)) {
        throw new TypeError('CEM-ML WASM returned an unsupported HTML-render response');
    }
    return response;
}

/** Format authored HTML or CEM-ML as lossless semantic source markup. */
export async function highlightCemSource(
    source: string,
    contentType: string,
    sourceUrl?: string
): Promise<CemMlSourceHighlightResult> {
    const runtime = await loadRuntime();
    const response = JSON.parse(runtime.highlightSourceToHtmlV1(JSON.stringify({
        source,
        contentType,
        sourceUrl,
    })));
    if (!isSourceHighlightResult(response)) {
        throw new TypeError('CEM-ML WASM returned an unsupported source-highlight response');
    }
    return response;
}

async function loadRuntime(): Promise<CemMlWasmRuntime> {
    runtimePromise ??= import('@epa-wg/cem-ml/wasm').then(async (runtime) => {
        const browserInitializer = (runtime as unknown as { default?: unknown }).default;
        if (typeof browserInitializer === 'function') await browserInitializer();
        return runtime as unknown as CemMlWasmRuntime;
    });
    return runtimePromise;
}

function isRenderResult(value: unknown): value is CemMlHtmlRenderResult {
    if (!value || typeof value !== 'object') return false;
    const candidate = value as Partial<CemMlHtmlRenderResult>;
    return candidate.schemaVersion === 1
        && (candidate.status === 'rendered'
            || candidate.status === 'invalid'
            || candidate.status === 'error')
        && typeof candidate.html === 'string'
        && Array.isArray(candidate.diagnostics)
        && Array.isArray(candidate.outputSpans);
}

function isSourceHighlightResult(value: unknown): value is CemMlSourceHighlightResult {
    if (!value || typeof value !== 'object') return false;
    const candidate = value as Partial<CemMlSourceHighlightResult>;
    return candidate.schemaVersion === 1
        && (candidate.status === 'highlighted'
            || candidate.status === 'unsupported'
            || candidate.status === 'invalid'
            || candidate.status === 'error')
        && typeof candidate.html === 'string'
        && Array.isArray(candidate.diagnostics)
        && Array.isArray(candidate.spans);
}
