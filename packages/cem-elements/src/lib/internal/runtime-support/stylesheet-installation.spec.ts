import { describe, expect, it, vi } from 'vitest';
import type { CemStylesheetConsumerLease } from '../../declaration-style-ownership.js';
import { installRetainedStylesheets } from './stylesheet-installation.js';
import { CemProcessingDiagnosticError, type CemProcessingHost, type CemProcessingStylesheetInput,
    type CemProcessingStylesheetResult } from './processing-host.js';

function deferred<T>() {
    let resolve: (value: T) => void = () => undefined;
    const promise = new Promise<T>(done => { resolve = done; });
    return { promise, resolve };
}
const artifact = { kind: 'template-artifact-handle' as const, artifactId: 'card', cacheKey: 'source',
    registrationIdentity: 'registration', scopePolicyStamp: 'policy', sourceMapMode: 'dev' as const };
const ready = (loadId: string): CemProcessingStylesheetResult => ({ status: 'ready', loadId, css: '', diagnostics: [],
    identity: { ownerKey: loadId, cacheKey: loadId, contextMarker: null } });
const pending = (loadId: string): CemProcessingStylesheetResult => ({ status: 'pending', loadId,
    request: { id: 1, url: 'https://example.test/child.css', contentType: null, integrity: null } });

function fixture(run: (input: CemProcessingStylesheetInput) => Promise<CemProcessingStylesheetResult>) {
    const abort = new AbortController();
    const lease: CemStylesheetConsumerLease = { signal: abort.signal, commit: vi.fn(() => true),
        release: vi.fn(() => abort.abort()) };
    let jobId = 0;
    const host = { mode: 'worker', ownerScope: { disposed: false },
        stylesheet: vi.fn((input: CemProcessingStylesheetInput) => ({ jobId: ++jobId, result: run(input) })),
        cancel: vi.fn(({ targetJobId }: { targetJobId: number }) => ({ jobId: ++jobId,
            result: Promise.resolve({ targetJobId, accepted: true }) })),
    } as unknown as CemProcessingHost;
    const options = { host, artifact, consumer: 'card', lease, baseUrl: 'https://example.test/main.css',
        context: { identity: 'context', resolverIdentity: 'resolver', resourcePolicyStamp: 'policy', frames: [] },
        occurrences: [{ index: 0, scope: { kind: 'private' as const, tag: 'cem-card' } }],
        read: vi.fn(async () => ({ bytes: new ArrayBuffer(0), finalUrl: 'https://example.test/child.css' })),
    };
    return { host, lease, abort, options };
}

describe('retained stylesheet installation lifecycle', () => {
    it('settles cancellation before a native response arrives and releases a late generation', async () => {
        const response = deferred<CemProcessingStylesheetResult>();
        const f = fixture(async input => input.action === 'release' ? { status: 'released', count: 1 } : response.promise);
        const load = installRetainedStylesheets(f.options);
        f.abort.abort();
        await expect(load.ready).resolves.toMatchObject({ status: 'cancelled', installed: 0 });
        expect(f.host.cancel).toHaveBeenCalledWith({ targetJobId: 1, reason: 'superseded' });
        expect(f.lease.commit).not.toHaveBeenCalled();
        await load.dispose(); // Cancellation does not wait for an unresponsive native job.
        response.resolve(ready('late'));
        await vi.waitFor(() => expect(f.host.stylesheet).toHaveBeenCalledWith({
            action: 'release', artifact, consumer: 'card', loadId: 'late',
        }));
        await load.dispose();
    });

    it('does not wait for an uncooperative reader during disposal', async () => {
        const f = fixture(async input => input.action === 'release' ? { status: 'released', count: 1 } : pending('waiting'));
        f.options.read = vi.fn(() => new Promise(() => undefined));
        const load = installRetainedStylesheets(f.options);
        await vi.waitFor(() => expect(f.options.read).toHaveBeenCalledOnce());
        await load.dispose();
        await expect(load.ready).resolves.toMatchObject({ status: 'cancelled' });
        expect(f.lease.signal.aborted).toBe(true);
    });

    it('preserves structured failures and installs an independent occurrence', async () => {
        const diagnostic = { code: 'cem.css.import_content_type', severity: 'error' as const, message: 'CSS required',
            sourceUri: 'urn:source', stylesheetUrl: 'https://example.test/main.css', line: 1, column: 1, offset: 0, length: 20 };
        const f = fixture(async input => {
            if (input.action === 'begin') return input.index === 0 ? pending('failed') : ready('valid');
            if (input.action === 'deliver') throw new CemProcessingDiagnosticError([diagnostic]);
            return { status: 'released', count: 1 };
        });
        f.options.occurrences.push({ index: 1, scope: { kind: 'private', tag: 'cem-card' } });
        const load = installRetainedStylesheets(f.options);
        await expect(load.ready).resolves.toEqual({ status: 'ready', installed: 1, diagnostics: [diagnostic] });
        await load.dispose();
        expect(f.host.stylesheet).toHaveBeenCalledWith(expect.objectContaining({ action: 'release', loadId: 'failed' }));
        expect(f.host.stylesheet).toHaveBeenCalledWith(expect.objectContaining({ action: 'release', loadId: 'valid' }));
    });

    it('keeps delivery failure diagnostics after worker replacement without releasing its dead owner', async () => {
        const f = fixture(async input => {
            if (input.action === 'begin') return input.index === 0 ? pending('worker-owner') : ready('fallback-owner');
            if (input.action === 'deliver') {
                Object.defineProperty(f.host, 'mode', { value: 'main-thread' });
                throw new Error('restart the load after worker replacement');
            }
            return { status: 'released', count: 1 };
        });
        f.options.occurrences.push({ index: 1, scope: { kind: 'private', tag: 'cem-card' } });
        const load = installRetainedStylesheets(f.options);
        await expect(load.ready).resolves.toMatchObject({ status: 'ready', installed: 1,
            diagnostics: [expect.objectContaining({ message: 'restart the load after worker replacement' })] });
        await load.dispose();
        expect(f.host.stylesheet).not.toHaveBeenCalledWith(expect.objectContaining({ action: 'release', loadId: 'worker-owner' }));
        expect(f.host.stylesheet).toHaveBeenCalledWith(expect.objectContaining({ action: 'release', loadId: 'fallback-owner' }));
    });

    it('reports cleanup failure while the processing host remains active', async () => {
        const f = fixture(async input => {
            if (input.action === 'release') throw new Error('release transport failed');
            return ready('active');
        });
        const load = installRetainedStylesheets(f.options);
        await load.ready;
        await expect(load.dispose()).rejects.toThrow('native stylesheet cleanup failed');
    });

    it('accepts cleanup after host disposal has already dropped native owners', async () => {
        const f = fixture(async input => {
            if (input.action === 'release') throw new Error('host disposed');
            return ready('disposed');
        });
        const load = installRetainedStylesheets(f.options);
        await load.ready;
        Object.defineProperty(f.host.ownerScope, 'disposed', { value: true });
        await expect(load.dispose()).resolves.toBeUndefined();
    });
});

it.each(['throw', 'error', 'fatal'] as const)('rejects the complete replacement when an occurrence reports %s', async failure => {
    const diagnostic = { code: 'cem.css.failure', severity: failure === 'fatal' ? 'fatal' as const : 'error' as const, message: 'failed stylesheet' };
    const f = fixture(async input => {
        if (input.action === 'release') return { status: 'released', count: 1 };
        if (input.action === 'begin' && input.index === 1) {
            if (failure === 'throw') throw new CemProcessingDiagnosticError([diagnostic]);
            return { ...ready('invalid'), diagnostics: [diagnostic] };
        }
        return ready('valid');
    });
    f.options.occurrences.push({ index: 1, scope: { kind: 'private', tag: 'cem-card' } });
    const load = installRetainedStylesheets({ ...f.options, requireComplete: true });
    expect(await load.ready).toMatchObject({ status: 'cancelled', installed: 0, diagnostics: [diagnostic] });
    expect(f.lease.commit).not.toHaveBeenCalled();
    expect(f.lease.release).toHaveBeenCalled();
    await load.dispose();
    expect(f.host.stylesheet).toHaveBeenCalledWith(expect.objectContaining({ action: 'release', loadId: 'valid' }));
});
