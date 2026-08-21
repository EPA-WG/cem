import { describe, expect, it, vi } from 'vitest';

import {
    CEM_REPOSITORY_PROTOCOL_VERSION,
    CemRepositoryContractError,
    CemRepositoryRegistry,
    type CemRepositoryPort,
    type CemRepositoryRequest,
} from './repository.js';

const request: CemRepositoryRequest = {
    protocolVersion: CEM_REPOSITORY_PROTOCOL_VERSION,
    repository: 'studio-projects',
    operation: 'list-projects',
    requestRevision: 4,
    parameters: { includeTrash: false },
};

describe('CemRepositoryRegistry', () => {
    it('routes clone-safe queries without exposing host-owned values', async () => {
        const value = { projects: [{ id: 'tour' }] };
        const query = vi.fn(async (input: CemRepositoryRequest) => ({
            protocolVersion: CEM_REPOSITORY_PROTOCOL_VERSION,
            repository: input.repository,
            operation: input.operation,
            requestRevision: input.requestRevision,
            repositoryRevision: 7,
            value,
            diagnostics: [],
        }));
        const registry = new CemRepositoryRegistry();
        registry.register('studio-projects', port({ query }));

        const result = await registry.query(request);
        expect(query).toHaveBeenCalledWith(request, undefined);
        expect(result.value).toEqual(value);
        expect(result.value).not.toBe(value);
    });

    it('rejects duplicate, unknown, invalid, and non-clone-safe requests', async () => {
        const registry = new CemRepositoryRegistry();
        const registered = port();
        registry.register('studio-projects', registered);
        expect(() => registry.register('studio-projects', registered)).toThrowError(CemRepositoryContractError);
        await expect(registry.query({ ...request, repository: 'missing' })).rejects.toMatchObject({
            code: 'cem.repository.not_registered',
        });
        await expect(registry.query({ ...request, operation: 'List Projects' })).rejects.toMatchObject({
            code: 'cem.repository.invalid_operation',
        });
        await expect(registry.query({ ...request, parameters: { callback: () => undefined } })).rejects.toMatchObject({
            code: 'cem.repository.not_clone_safe',
        });
    });

    it('rejects mismatched host responses and unregisters only its own port', async () => {
        const registry = new CemRepositoryRegistry();
        const unregister = registry.register(
            'studio-projects',
            port({
                query: async () => ({
                    protocolVersion: CEM_REPOSITORY_PROTOCOL_VERSION,
                    repository: 'other-projects',
                    operation: request.operation,
                    requestRevision: request.requestRevision,
                    repositoryRevision: 0,
                    value: null,
                    diagnostics: [],
                }),
            }),
        );
        await expect(registry.query(request)).rejects.toMatchObject({ code: 'cem.repository.response_mismatch' });
        unregister();
        expect(registry.has('studio-projects')).toBe(false);
    });
});

function port(overrides: Partial<CemRepositoryPort> = {}): CemRepositoryPort {
    return {
        query: async (input) => ({
            protocolVersion: CEM_REPOSITORY_PROTOCOL_VERSION,
            repository: input.repository,
            operation: input.operation,
            requestRevision: input.requestRevision,
            repositoryRevision: 0,
            value: null,
            diagnostics: [],
        }),
        execute: async (input) => ({
            protocolVersion: CEM_REPOSITORY_PROTOCOL_VERSION,
            repository: input.repository,
            operation: input.operation,
            requestRevision: input.requestRevision,
            repositoryRevision: 0,
            value: null,
            diagnostics: [],
        }),
        subscribe: () => () => undefined,
        status: async () => ({
            protocolVersion: CEM_REPOSITORY_PROTOCOL_VERSION,
            repository: 'studio-projects',
            state: 'ready',
            repositoryRevision: 0,
            diagnostics: [],
        }),
        ...overrides,
    };
}
