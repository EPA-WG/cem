import { createHash } from 'node:crypto';
import { describe, expect, it, vi } from 'vitest';
import { readCommandOutput, readCommandOutputs, type CommandOutputMetadata } from './command-result.js';

const bytes = new TextEncoder().encode('catalog café');
const metadata = (data = bytes): CommandOutputMetadata => ({ uri: 'memory:output', kind: 'output',
    contentType: 'text/plain', byteLength: data.byteLength, sha256: createHash('sha256').update(data).digest('hex') });
const options = { maxBytes: 100, chunkBytes: 3 };

describe('bounded command output', () => {
    it('distinguishes failed absence from a published empty output', async () => {
        const handle = { readArtifact: vi.fn() };
        const absent = await readCommandOutput(handle, { exitCode: 1 }, options);
        expect(absent).toMatchObject({ available: false, text: '', byteLength: 0,
            sha256: metadata(new Uint8Array()).sha256 });
        expect(handle.readArtifact).not.toHaveBeenCalled();
        await expect(readCommandOutput(handle, { exitCode: 0 }, options)).rejects.toThrow('omitted');
        const empty = await readCommandOutput(handle, { exitCode: 0, artifacts: { items: [metadata(new Uint8Array())] } }, options);
        expect(empty).toMatchObject({ available: true, byteLength: 0, text: '' });
    });

    it('accepts a native parse refusal without masking its diagnostics or inventing output', async () => {
        const handle = { readArtifact: vi.fn() };
        const diagnostic = Object.freeze({ code: 'cem.csv.unclosed_quote', severity: 'error',
            uri: 'studio://inventory/data.csv', byteOffset: 10, line: 2, column: 3 });
        const result = Object.freeze({ exitCode: 1, artifacts: { originalCount: 0, items: [] },
            diagnostics: { originalCount: 1, items: [diagnostic] } });
        expect(await readCommandOutputs(handle, result, { ...options, maxArtifacts: 3 })).toEqual([]);
        expect(await readCommandOutput(handle, result, options)).toMatchObject({ available: false, byteLength: 0, text: '' });
        expect(result.diagnostics.items[0]).toBe(diagnostic);
        expect(handle.readArtifact).not.toHaveBeenCalled();
        // Diagnostics cannot excuse a successful command's missing publication.
        await expect(readCommandOutputs(handle, { ...result, exitCode: 0 }, { ...options, maxArtifacts: 3 })).rejects.toThrow('omitted');
    });

    it('reads bounded retained ranges and checks their exact digest', async () => {
        const handle = { readArtifact: vi.fn(async (_artifact, { offset, maxBytes }) => ({
            metadata: { offset }, bytes: bytes.slice(offset, offset + maxBytes),
        })) };
        const result = { exitCode: 0, artifacts: { items: [metadata()] } };
        expect(await readCommandOutput(handle, result, options)).toMatchObject({ available: true, bytes, text: 'catalog café' });
        expect(handle.readArtifact.mock.calls.every(([, range]) => range.maxBytes <= 3)).toBe(true);
        await expect(readCommandOutput(handle, { ...result, artifacts: { items: [{ ...metadata(), sha256: 'wrong' }] } }, options))
            .rejects.toThrow('hash');
        await expect(readCommandOutput(handle, result, { maxBytes: 1 })).rejects.toThrow('limit');
    });

    it.each(['empty', 'oversized', 'offset'])('rejects %s retained ranges even on failed commands', async kind => {
        const handle = { readArtifact: vi.fn(async () => ({ metadata: { offset: kind === 'offset' ? 1 : 0 },
            bytes: kind === 'empty' ? new Uint8Array() : kind === 'oversized' ? bytes : bytes.slice(0, 3) })) };
        await expect(readCommandOutput(handle, { exitCode: 1, artifacts: { items: [metadata()] } }, options))
            .rejects.toThrow('invalid artifact range');
    });

    it('verifies committed publications without reading retained artifacts', async () => {
        const handle = { readArtifact: vi.fn() };
        const committedPublications = [{ request: metadata(), bytes }];
        const value = await readCommandOutput(handle, { exitCode: 0 }, { ...options, committedPublications });
        expect(value).toMatchObject({ available: true, bytes });
        expect(value.bytes).not.toBe(bytes);
        expect(handle.readArtifact).not.toHaveBeenCalled();
        await expect(readCommandOutput(handle, { exitCode: 1 }, { ...options,
            committedPublications: [{ request: metadata(), bytes: bytes.slice(1) }] })).rejects.toThrow('length');
    });

    it('admits a bounded multi-output set with native identities and exact per-output bytes', async () => {
        const first = { ...metadata(), uri: 'memory:one', handleId: 3, sourceMapId: 'map-3' };
        const second = { ...metadata(new Uint8Array()), uri: 'memory:empty', handleId: 4 };
        const handle = { readArtifact: vi.fn(async (_artifact, { offset, maxBytes }) => ({
            metadata: { offset }, bytes: bytes.slice(offset, offset + maxBytes),
        })) };
        const result = { exitCode: 0, artifacts: { items: [first, second] } };
        expect(await readCommandOutputs(handle, result, { ...options, maxArtifacts: 2 }))
            .toMatchObject([{ uri: first.uri, handleId: 3, sourceMapId: 'map-3', bytes }, { uri: second.uri, byteLength: 0 }]);
        expect(await readCommandOutputs(handle, result, { ...options, maxArtifacts: 2,
            committedPublications: [{ request: { ...first, uri: 'memory:transport-alias' }, bytes }] })).toHaveLength(2);
        await expect(readCommandOutput(handle, result, { ...options, requestedUri: 'memory:missing', fallbackToFirstOutput: false }))
            .rejects.toThrow('omitted');
        await expect(readCommandOutputs(handle, result, { ...options, maxArtifacts: 1 })).rejects.toThrow('count');
        await expect(readCommandOutputs(handle, { ...result, artifacts: { items: [first, { ...first, uri: 'memory:two' }] } },
            { ...options, maxArtifacts: 2, maxBytes: bytes.length })).rejects.toThrow('total byte limit');
    });

    it('rejects ambiguous, truncated and corrupt sets without returning a partial manifest', async () => {
        const handle = { readArtifact: vi.fn() };
        const entry = metadata();
        const read = (items, extra = {}) => readCommandOutputs(handle, { exitCode: 0, artifacts: { items, ...extra } }, { ...options, maxArtifacts: 3 });
        await expect(read([entry, entry])).rejects.toThrow('duplicate');
        await expect(read([{ ...entry, uri: '' }])).rejects.toThrow('missing');
        await expect(read([entry], { originalCount: 2 })).rejects.toThrow('truncated');
        expect(handle.readArtifact).not.toHaveBeenCalled();
        await expect(readCommandOutputs(handle, { exitCode: 1 }, { ...options, maxArtifacts: 3 })).resolves.toEqual([]);
        await expect(readCommandOutputs(handle, { exitCode: 0 }, { ...options, maxArtifacts: 3,
            committedPublications: [{ request: entry, bytes }, { request: { ...entry, uri: 'memory:two', sha256: 'wrong' }, bytes }] }))
            .rejects.toThrow('hash');
    });
});
