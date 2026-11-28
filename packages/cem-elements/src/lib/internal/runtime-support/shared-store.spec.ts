import { describe, expect, it, vi } from 'vitest';
import { bindSharedStore, type SharedStoreTransaction } from './shared-store.js';

describe('write-only named store', () => {
    it('refuses read-dependent operations before contacting the provider', async () => {
        const transact = vi.fn<SharedStoreTransaction>();
        const writer = bindSharedStore(transact, { read: false, write: true }, undefined, () => true);
        for (const operation of [
            () => writer.getItem('item'), () => writer.read('item'), () => writer.keys(), () => writer.clear(),
            () => writer.setItem('item', 'replacement', 'token'), () => writer.removeItem('item', 'token'),
        ]) {
            await expect(operation()).rejects.toMatchObject({ name: 'NotAllowedError' });
        }
        expect(transact).not.toHaveBeenCalled();
    });

    it.each([
        ['QuotaExceededError', 'QuotaExceededError'],
        ['private provider code', 'OperationError'],
    ])('redacts provider details for %s', async (providerCode, publicCode) => {
        const failure = new DOMException('private provider detail', providerCode);
        const transact = vi.fn<SharedStoreTransaction>().mockRejectedValue(failure);
        const writer = bindSharedStore(transact, { read: false, write: true }, undefined, () => true);
        const expected = { name: publicCode, message: 'Shared store write failed' };
        await expect(writer.setItem('item', 'replacement')).rejects.toMatchObject(expected);
        await expect(writer.removeItem('item')).rejects.toMatchObject(expected);
    });
});
