/** One bounded least-recently-used map backed by JavaScript's insertion-ordered Map. */
export class CemProcessingLruCache<TKey, TValue> {
    private readonly entries = new Map<TKey, TValue>();

    constructor(readonly capacity: number) {
        if (!Number.isSafeInteger(capacity) || capacity < 1) {
            throw new RangeError(`a CEM processing cache capacity must be a positive safe integer; received ${capacity}`);
        }
    }

    get size(): number {
        return this.entries.size;
    }

    get(key: TKey): TValue | undefined {
        const value = this.entries.get(key);
        if (value === undefined) {
            return undefined;
        }
        this.entries.delete(key);
        this.entries.set(key, value);
        return value;
    }

    set(key: TKey, value: TValue): { key: TKey; value: TValue } | undefined {
        this.entries.delete(key);
        this.entries.set(key, value);
        if (this.entries.size <= this.capacity) {
            return undefined;
        }
        const oldest = this.entries.entries().next().value as [TKey, TValue] | undefined;
        if (!oldest) {
            return undefined;
        }
        this.entries.delete(oldest[0]);
        return { key: oldest[0], value: oldest[1] };
    }

    clear(): void {
        this.entries.clear();
    }
}
