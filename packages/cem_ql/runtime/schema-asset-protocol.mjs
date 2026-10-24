/** Shared bounded original-byte/control protocol. No native handle or grant fields. */
export function parseSchemaAssetPackageRequest(value, type = 'cem-command-package') {
    if (value.type !== type || !Number.isSafeInteger(value.actionId) || value.actionId < 1) throw Error('Invalid package action');
    const operation = value.operation;
    const fields = operation === 'bootstrap' || operation === 'release' ? []
        : operation === 'summary' ? ['index', 'pin'] : operation === 'import' ? ['index', 'pin', 'source']
        : operation === 'activate' ? ['index', 'pin', 'sources', 'selection'] : undefined;
    if (!fields || Object.keys(value).length !== fields.length + 3) throw Error('Invalid package message');
    if (fields.length) {
        if (!(value.index instanceof Uint8Array) || !(value.index.buffer instanceof ArrayBuffer) || value.index.byteLength < 1 || value.index.buffer.byteLength > 4 * 1024 * 1024
            || typeof value.pin !== 'string' || !/^[a-f0-9]{64}$/u.test(value.pin)) throw Error('Invalid package index');
    }
    // Structured clone copies whole backing buffers, including bytes outside a view.
    let total = value.index?.buffer.byteLength ?? 0;
    const source = entry => {
        if (!isRecord(entry) || Object.keys(entry).length !== 4 || typeof entry.packageId !== 'string' || !entry.packageId || entry.packageId.length > 256
            || typeof entry.version !== 'string' || !entry.version || entry.version.length > 64 || typeof entry.path !== 'string' || !entry.path || entry.path.length > 4096
            || !(entry.bytes instanceof Uint8Array) || !(entry.bytes.buffer instanceof ArrayBuffer) || entry.bytes.byteLength < 1 || entry.bytes.buffer.byteLength > 16 * 1024 * 1024) return false;
        total += entry.bytes.buffer.byteLength; return total <= 32 * 1024 * 1024;
    };
    if (operation === 'import' && !source(value.source)) throw Error('Invalid package source');
    if (operation === 'activate' && (!Array.isArray(value.sources) || value.sources.length < 1 || value.sources.length > 1024
        || !Array.from(value.sources).every(source) || typeof value.selection !== 'string' || value.selection.length > 32 * 1024)) throw Error('Invalid package selection');
    return value;
}

function isRecord(value) {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}
