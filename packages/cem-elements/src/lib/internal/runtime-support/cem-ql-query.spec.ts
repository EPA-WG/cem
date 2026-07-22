import { beforeEach, describe, expect, it, vi } from 'vitest';

const mocks = vi.hoisted(() => ({
    ensureRuntimeReady: vi.fn(),
    evaluateQuerySource: vi.fn(),
}));

vi.mock('./cem-ql-render.js', () => ({
    ensureRuntimeReady: mocks.ensureRuntimeReady,
}));

vi.mock('../../../../../cem_ql/dist/wasm/cem_ql.js', () => ({
    evaluateQuerySource: mocks.evaluateQuerySource,
}));

import {
    cemQlAtom,
    cemQlNode,
    cemQlResource,
    cemQlStream,
    evaluateCemQlQuery,
    mapCemQlQueryResult,
} from './cem-ql-query.js';

describe('CEM-QL query runtime support boundary', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('calls the WASM query export after runtime initialization', async () => {
        mocks.ensureRuntimeReady.mockResolvedValue(undefined);
        mocks.evaluateQuerySource.mockReturnValue(
            JSON.stringify({
                items: [{ kind: 'atomic', type: 'integer', value: 5 }],
                diagnostics: [],
                error: null,
            })
        );

        const result = await evaluateCemQlQuery('left + right', { left: 2, right: 3 });

        expect(mocks.ensureRuntimeReady).toHaveBeenCalledTimes(1);
        expect(mocks.evaluateQuerySource).toHaveBeenCalledWith('left + right', '{"left":2,"right":3}');
        expect(result).toEqual({
            items: [{ kind: 'atomic', type: 'integer', value: 5 }],
            diagnostics: [],
            error: null,
        });
    });

    it('rejects non-transport bindings before entering WASM', async () => {
        await expect(evaluateCemQlQuery('1', { callback: () => 1 })).rejects.toThrow(/function/);

        expect(mocks.evaluateQuerySource).not.toHaveBeenCalled();
    });

    it('maps typed items, diagnostics, and errors from the WASM JSON shape', () => {
        const result = mapCemQlQueryResult({
            items: [
                {
                    kind: 'record',
                    fields: {
                        name: [{ kind: 'atomic', type: 'string', value: 'Ada' }],
                        nodes: [{ kind: 'array', items: [{ kind: 'node', id: 'node-1' }] }],
                    },
                },
                {
                    kind: 'resource',
                    id: 'user',
                    contentType: 'application/vnd.cem.policy+json',
                    schema: null,
                    roles: ['admin'],
                    failAccessor: false,
                },
            ],
            diagnostics: [
                {
                    code: 'cem.ql.type_error',
                    severity: 'error',
                    message: 'type mismatch',
                    byteOffset: 7,
                },
            ],
            error: {
                kind: 'eval',
                type: 'type-error',
                message: 'type mismatch',
            },
        });

        expect(result.items).toEqual([
            {
                kind: 'record',
                fields: {
                    name: [{ kind: 'atomic', type: 'string', value: 'Ada' }],
                    nodes: [{ kind: 'array', items: [{ kind: 'node', id: 'node-1' }] }],
                },
            },
            {
                kind: 'resource',
                id: 'user',
                contentType: 'application/vnd.cem.policy+json',
                schema: null,
                roles: ['admin'],
                failAccessor: false,
            },
        ]);
        expect(result.diagnostics[0]).toEqual({
            code: 'cem.ql.type_error',
            severity: 'error',
            message: 'type mismatch',
            byteOffset: 7,
            sourceMapRef: { fidelity: 'author-byte-exact', frame: 'cem:7' },
        });
        expect(result.error).toEqual({
            kind: 'eval',
            type: 'type-error',
            message: 'type mismatch',
        });
    });

    it('builds plain JSON-compatible tagged bindings for parity stories', () => {
        expect(cemQlStream([1, 2])).toEqual({ $stream: [1, 2] });
        expect(cemQlNode('node-1')).toEqual({ $node: 'node-1' });
        expect(cemQlAtom('decimal', '1.25')).toEqual({ $atom: { type: 'decimal', value: '1.25' } });
        expect(
            cemQlResource({
                id: 'user',
                contentType: 'application/vnd.cem.policy+json',
                roles: ['admin'],
            })
        ).toEqual({
            $resource: {
                id: 'user',
                contentType: 'application/vnd.cem.policy+json',
                schema: null,
                roles: ['admin'],
                failAccessor: false,
            },
        });
    });
});
