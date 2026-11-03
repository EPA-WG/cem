import { expect, it, vi } from 'vitest';
import { prepareCemStudioRuntime } from './startup.js';

function deferred() {
    let resolve, reject;
    const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
    return { promise, resolve, reject };
}
const tick = async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); };

it('overlaps independent startup and publishes only after both original services are ready', async () => {
    const validator = deferred(), components = deferred(), owner = { close: vi.fn() };
    const installComponents = vi.fn(() => components.promise);
    let settled = false;
    const result = prepareCemStudioRuntime({ createValidator: () => validator.promise, installComponents });
    result.then(() => { settled = true; });
    await tick();
    expect(installComponents).toHaveBeenCalledOnce();
    components.resolve(); await tick(); expect(settled).toBe(false);
    validator.resolve(owner); expect(await result).toBe(owner); expect(owner.close).not.toHaveBeenCalled();
});
it('retains serial activation for profiles with shared package admission', async () => {
    const validator = deferred(), components = deferred(), owner = { close: vi.fn() };
    const installComponents = vi.fn(() => components.promise);
    const result = prepareCemStudioRuntime({ createValidator: () => validator.promise, installComponents, serial: true });
    await tick(); expect(installComponents).not.toHaveBeenCalled();
    validator.resolve(owner); await tick(); expect(installComponents).toHaveBeenCalledOnce();
    components.resolve(); expect(await result).toBe(owner);
});
it('releases a late validator when component startup fails and preserves the original error', async () => {
    const validator = deferred(), components = deferred(), owner = { close: vi.fn() }, failure = new Error('component source failed');
    const result = prepareCemStudioRuntime({ createValidator: () => validator.promise, installComponents: () => components.promise });
    const observed = result.catch(error => error);
    components.reject(failure); await tick(); validator.resolve(owner);
    expect(await observed).toBe(failure); expect(owner.close).toHaveBeenCalledOnce();
});
it('does not start package-scoped components after serial validator failure', async () => {
    const failure = new Error('native admission failed'), installComponents = vi.fn();
    await expect(prepareCemStudioRuntime({ createValidator: async () => { throw failure; }, installComponents, serial: true })).rejects.toBe(failure);
    expect(installComponents).not.toHaveBeenCalled();
});
it('retains both concurrent startup failures', async () => {
    const first = new Error('validator failed'), second = new Error('components failed');
    const error = await prepareCemStudioRuntime({ createValidator: async () => { throw first; }, installComponents: async () => { throw second; } }).catch(error => error);
    expect(error).toBeInstanceOf(AggregateError); expect(error.errors).toEqual([first, second]);
});
it('reports cleanup failure without losing the component error', async () => {
    const failure = new Error('components failed'), cleanup = new Error('native close failed');
    const owner = { close: vi.fn(() => { throw cleanup; }) };
    const error = await prepareCemStudioRuntime({ createValidator: async () => owner, installComponents: async () => { throw failure; } }).catch(error => error);
    expect(error).toBeInstanceOf(AggregateError); expect(error.errors).toEqual([failure, cleanup]);
});
