import { expect, it } from 'vitest';
import { snapshotInvocationGeometry } from './surface-invocation.js';

it('captures pointer and selection rectangles without retaining mutable geometry', () => {
    const selection = { left: 10, top: 20, right: 30, bottom: 40 };
    const result = snapshotInvocationGeometry('pointer', { x: 12, y: 24 }, undefined, selection);
    selection.left = 999;
    expect(result.pointer).toEqual({ left: 12, right: 12, top: 24, bottom: 24 });
    expect(result.selection?.left).toBe(10);
    expect(Object.isFrozen(result.selection)).toBe(true);
});
it('derives keyboard geometry from its owner and rejects unavailable programmatic geometry', () => {
    const owner = { left: 50, top: 60, right: 70, bottom: 80 };
    expect(snapshotInvocationGeometry('keyboard', { x: 999, y: 999 }, owner).pointer).toEqual(owner);
    expect(snapshotInvocationGeometry('programmatic', undefined, owner).pointer).toBeUndefined();
    for (const point of [{ x: NaN, y: 1 }, { x: 1, y: Infinity }]) expect(snapshotInvocationGeometry('pointer', point).pointer).toBeUndefined();
    expect(snapshotInvocationGeometry('pointer', undefined, undefined, { left: 1, right: 1, top: 1, bottom: 1 }).selection).toBeUndefined();
});
