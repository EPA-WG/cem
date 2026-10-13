const textDecoder = new TextDecoder('utf-8', { fatal: true });

export const CEM_STUDIO_LIMITS = Object.freeze({
    sourceBytes: 8 * 1024 * 1024,
    dependencyBytes: 8 * 1024 * 1024,
    resourceSetBytes: 16 * 1024 * 1024,
    resourceCount: 128,
    resultBytes: 16 * 1024 * 1024,
    inlinePreviewBytes: 256 * 1024,
    structuredRows: 100,
});

export class CemStudioLimitError extends Error {
    constructor(code, message, details) {
        super(message);
        this.name = 'CemStudioLimitError';
        this.code = code;
        this.details = Object.freeze({ ...details });
    }
}

export function assertCemStudioSourceSet(source, dependencies = []) {
    const sourceBytes = byteLength(source);
    assertMaximum(sourceBytes, CEM_STUDIO_LIMITS.sourceBytes, 'cem.studio.limit.source_bytes', 'source');
    if (dependencies.length > CEM_STUDIO_LIMITS.resourceCount) {
        throw new CemStudioLimitError(
            'cem.studio.limit.resource_count',
            `resource set has ${dependencies.length + 1} resources; the Studio maximum is ${CEM_STUDIO_LIMITS.resourceCount + 1}`,
            { actual: dependencies.length + 1, maximum: CEM_STUDIO_LIMITS.resourceCount + 1 },
        );
    }
    let total = sourceBytes;
    for (const dependency of dependencies) {
        const size = byteLength(dependency?.bytes);
        assertMaximum(size, CEM_STUDIO_LIMITS.dependencyBytes, 'cem.studio.limit.dependency_bytes', 'dependency');
        total += size;
        assertMaximum(total, CEM_STUDIO_LIMITS.resourceSetBytes, 'cem.studio.limit.resource_set_bytes', 'resource set');
    }
    return Object.freeze({ sourceBytes, resourceCount: dependencies.length + 1, totalBytes: total });
}

export function assertCemStudioResultSize(value) {
    const actual = typeof value === 'number' ? value : byteLength(value);
    assertMaximum(actual, CEM_STUDIO_LIMITS.resultBytes, 'cem.studio.limit.result_bytes', 'result');
    return actual;
}

export function assertCemStudioResourceUri(value, expectedScheme) {
    const url = new URL(value);
    const protocol = `${expectedScheme}:`;
    if (url.protocol !== protocol || url.username || url.password) {
        const error = new TypeError(
            `CEM Studio resource URL must use the ${protocol} virtual-resource scheme without credentials`,
        );
        error.code = 'cem.studio.security.resource_url';
        throw error;
    }
    return url.href;
}

export function createCemStudioPreview({ bytes, contentType = 'application/octet-stream', label = 'Result preview' }) {
    const source = toBytes(bytes);
    const byteLength = assertCemStudioResultSize(source);
    const normalizedContentType = normalizeContentType(contentType);
    const active = isActiveMarkup(normalizedContentType);
    const textual = active || isTextContentType(normalizedContentType);
    if (!textual) {
        return Object.freeze({
            kind: 'download',
            label,
            contentType: normalizedContentType,
            byteLength,
            reason: 'Binary or unknown output is not guessed; download the exact result to inspect it.',
        });
    }
    if (active && byteLength > CEM_STUDIO_LIMITS.inlinePreviewBytes) {
        return Object.freeze({
            kind: 'download',
            label,
            contentType: normalizedContentType,
            byteLength,
            reason: `Active markup exceeds the ${CEM_STUDIO_LIMITS.inlinePreviewBytes}-byte inline preview limit.`,
        });
    }
    const displayedBytes = source.subarray(0, CEM_STUDIO_LIMITS.inlinePreviewBytes);
    let text;
    try {
        text = decodeUtf8(displayedBytes, source.byteLength > displayedBytes.byteLength);
    } catch {
        return Object.freeze({
            kind: 'download',
            label,
            contentType: normalizedContentType,
            byteLength,
            reason: 'The declared text result is not valid UTF-8; download the exact bytes to inspect it.',
        });
    }
    if (active) {
        return Object.freeze({
            kind: 'sandboxed-html',
            label,
            contentType: normalizedContentType,
            byteLength,
            displayedBytes: displayedBytes.byteLength,
            truncated: false,
            source: text,
        });
    }
    return Object.freeze({
        kind: 'text',
        label,
        contentType: normalizedContentType,
        byteLength,
        displayedBytes: displayedBytes.byteLength,
        truncated: source.byteLength > displayedBytes.byteLength,
        text,
    });
}

export function redactCemStudioSecrets(value) {
    return String(value)
        .replace(/(https?:\/\/)[^\s/@:]+:[^\s/@]+@/giu, '$1[redacted]@')
        .replace(
            /([?&](?:access_token|api_key|apikey|authorization|credential|key|password|secret|signature|token)=)[^&#\s]*/giu,
            '$1[redacted]',
        )
        .replace(/(\bBearer\s+)[A-Za-z0-9._~+/=-]+/giu, '$1[redacted]')
        .replace(/(\bauthorization\s*:\s*)(?!Bearer\b)\S+/giu, '$1[redacted]')
        .replace(/(\bx-api-key\s*:\s*)\S+/giu, '$1[redacted]');
}

function assertMaximum(actual, maximum, code, label) {
    if (!Number.isSafeInteger(actual) || actual < 0 || actual > maximum) {
        throw new CemStudioLimitError(code, `${label} is ${actual} bytes; the Studio maximum is ${maximum}`, {
            actual,
            maximum,
        });
    }
}

function byteLength(value) {
    if (value instanceof ArrayBuffer) return value.byteLength;
    if (ArrayBuffer.isView(value)) return value.byteLength;
    if (Array.isArray(value)) return value.length;
    throw new TypeError('CEM Studio bytes must be an ArrayBuffer, view, or byte array');
}

function toBytes(value) {
    if (value instanceof Uint8Array) return value;
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    if (Array.isArray(value)) return Uint8Array.from(value);
    throw new TypeError('CEM Studio preview bytes are unavailable');
}

function normalizeContentType(value) {
    return String(value).split(';', 1)[0].trim().toLowerCase() || 'application/octet-stream';
}

function isActiveMarkup(contentType) {
    return contentType === 'text/html' || contentType === 'application/xhtml+xml' || contentType === 'image/svg+xml';
}

function isTextContentType(contentType) {
    return (
        contentType.startsWith('text/') ||
        contentType === 'application/json' ||
        contentType.endsWith('+json') ||
        contentType === 'application/xml' ||
        contentType.endsWith('+xml') ||
        contentType.includes('cem')
    );
}

function decodeUtf8(bytes, mayEndMidSequence) {
    if (!mayEndMidSequence) return textDecoder.decode(bytes);
    let end = bytes.byteLength;
    while (end > Math.max(0, bytes.byteLength - 4)) {
        try {
            return textDecoder.decode(bytes.subarray(0, end));
        } catch {
            end -= 1;
        }
    }
    return textDecoder.decode(bytes);
}
