import { describe, expect, it } from 'vitest';
import { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';

const metadata = { instanceId: 'a', declarationIdentity: 'template:1', dataRevision: '1' };
const options = () => ({ runtimeId: 'host-session', generation: 1, policyStamp: 'private:1',
    allowOwner: () => true, seal: (candidate: string) => candidate });
const read = (coordinator: CemCheckpointPublicationCoordinator) => JSON.parse(new TextDecoder().decode(coordinator.current()!.bytes));

describe('all-owner publication checkpoint capture', () => {
    it('requires every publication and drained admission, then exports all islands together', async () => {
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => '<template>unsaved a</template>');
        const b = c.register('b', () => '<template>unsaved b</template>');
        a.published(metadata); await expect(c.capture()).rejects.toThrow('unavailable');
        b.published({ ...metadata, instanceId: 'b' });
        const release = c.admit(); await expect(c.capture()).rejects.toThrow('unavailable'); release();
        await c.capture(); expect(read(c).owners.map((o: { island: string }) => o.island)).toEqual(['<template>unsaved a</template>', '<template>unsaved b</template>']);
        a.input(); expect(c.current()).toBeUndefined(); expect(c.lastCommitted()).toBeDefined();
        a.published({ ...metadata, dataRevision: '2' }); await c.capture();
        expect(read(c).receipt.owners[0]).toMatchObject({ dataRevision: '2', acceptedEventSequence: 1 });
        b.remove(); expect(c.current()).toBeUndefined();
        await c.capture(); expect(read(c).owners).toHaveLength(1);
        a.remove();
        const reconnected = c.register('a', () => '<template>reconnected</template>', () => true, 42);
        reconnected.published(metadata); await c.capture();
        expect(read(c).receipt.owners[0].acceptedEventSequence).toBe(42);
        expect(() => a.published(metadata)).toThrow('stale');
        c.dispose();
    });
    it('fences a pending seal on input and coalesces repeated input without retaining payloads', async () => {
        let finish!: (text: string) => void;
        let held = false;
        const c = new CemCheckpointPublicationCoordinator({ ...options(), seal: text => held ? new Promise(resolve => { finish = () => resolve(text); }) : text });
        const a = c.register('a', () => '<template>current</template>'); a.published(metadata); await c.capture();
        held = true; const pending = c.capture();
        for (let i = 0; i < 1000; i++) a.input();
        finish(''); await expect(pending).rejects.toThrow('stale');
        expect(c.current()).toBeUndefined(); expect(c.lastCommitted()).toBeDefined();
        held = false; a.published({ ...metadata, dataRevision: 'latest' }); await c.capture();
        expect(read(c).receipt.owners[0].acceptedEventSequence).toBe(1000); c.dispose();
    });
    it('refuses privacy/partial exports, erases on policy changes and rejects stale owner handles', async () => {
        let allowed = false, exportCalls = 0;
        const c = new CemCheckpointPublicationCoordinator({ ...options(), allowOwner: () => allowed });
        const a = c.register('a', () => { exportCalls++; return '<template>private</template>'; });
        a.published(metadata); await expect(c.capture()).rejects.toThrow('privacy'); expect(exportCalls).toBe(0);
        allowed = true; await c.capture();
        c.setPolicy('private:2', () => false); expect(c.lastCommitted()).toBeUndefined();
        await expect(c.capture()).rejects.toThrow('unavailable');
        a.published(metadata); await expect(c.capture()).rejects.toThrow('privacy');
        a.remove(); const replacement = c.register('a', () => '<template>new</template>');
        expect(() => a.published(metadata)).toThrow('stale'); replacement.remove(); c.dispose();
    });
    it('checks live mutations before and after sealing, and refuses oversized/missing owner exports', async () => {
        let valid = true, island: string | undefined = '<template>good</template>';
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => island, () => valid); a.published(metadata); await c.capture();
        valid = false; expect(c.current()).toBeUndefined(); await expect(c.capture()).rejects.toThrow('unavailable');
        valid = true; a.published(metadata); island = undefined; await expect(c.capture()).rejects.toThrow('unavailable');
        island = 'é'.repeat(600_000); await expect(c.capture()).rejects.toThrow('limit');
        expect(c.lastCommitted()).toBeDefined(); c.dispose();
    });
    it('coalesces a newer automatic publication after a held seal and never rolls back', async () => {
        let release!: () => void, calls = 0;
        const c = new CemCheckpointPublicationCoordinator({ ...options(), automatic: true, seal: text => {
            calls++; return calls === 1 ? new Promise(resolve => { release = () => resolve(text); }) : text;
        } });
        const a = c.register('a', () => '<template>latest</template>');
        a.published(metadata); await Promise.resolve();
        a.input(); a.published({ ...metadata, dataRevision: 'new' }); release();
        for (let i = 0; i < 20 && !c.current(); i++) await Promise.resolve();
        expect(calls).toBe(2); expect(read(c).receipt.owners[0].dataRevision).toBe('new'); c.dispose();
    });
    it('fails closed on owner/admission overflow rather than silently omitting state', async () => {
        const c = new CemCheckpointPublicationCoordinator(options());
        for (let i = 0; i < 256; i++) c.register(`owner-${i}`, () => '<template/>').published({ ...metadata, instanceId: `owner-${i}` });
        await c.capture();
        expect(() => c.register('overflow', () => '<template/>')).toThrow('owners');
        expect(c.current()).toBeUndefined(); expect(c.lastCommitted()).toBeDefined(); c.dispose();
        const d = new CemCheckpointPublicationCoordinator(options());
        const release = Array.from({ length: 256 }, () => d.admit());
        expect(() => d.admit()).toThrow('limit'); release.forEach(finish => finish());
        await expect(d.capture()).rejects.toThrow('unavailable'); d.dispose();
    });

});
