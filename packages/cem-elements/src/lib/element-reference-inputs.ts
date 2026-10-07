import type { CemValueArtifactLimits } from './native-values.js';

/** Explicit host authority, separate from authored documents and data-island JSON. */
export interface CemElementReferenceInputs {
    kind: 'cem-element-reference-inputs-v1';
    requesting: number;
    sources: readonly {
        /** Explicit debug CEMB reload boundary, including original lexical capture. */
        bundle: ArrayBuffer;
        primarySourceId: number;
        context: boolean;
        maxDepth?: number;
        maxWork?: number;
        unresolved?: 'mandatory' | 'warning' | 'ignore' | 'neutral';
    }[];
    /** Passive source selection supplies native slice values for declared names. */
    bindings: readonly { source: number; name: string; select: string }[];
    grants: readonly (readonly [number, number])[];
    /** Issued by the embedding placement coordinator, separate from source grants. */
    placements?: CemElementPlacementSnapshot;
}
export interface CemElementPlacementSnapshot {
    admissions: readonly { source: number; select: string; token: string; producer: string; path: readonly number[]; revision: string; id: string }[];
    grants: readonly { requester: string; token: string; properties: readonly string[] }[];
    committedRevisions: Readonly<Record<string, string>>;
    preparedTransaction?: { token: string; participants: readonly string[]; producerRevisions: Readonly<Record<string, string>> };
}
export interface CemElementPlacementUse {
    token: string;
    producer: string;
    revision: string;
    id: string;
    renderNodeId: string;
    attribute: string;
    transaction?: string;
}
export function assertElementReferenceInputs(input: CemElementReferenceInputs, limits: CemValueArtifactLimits): void {
    const index = (n: number) => Number.isSafeInteger(n) && n >= 0 && n < input.sources.length;
    if (input.kind !== 'cem-element-reference-inputs-v1' || !Array.isArray(input.sources) || !Array.isArray(input.bindings)
        || !Array.isArray(input.grants) || !index(input.requesting)
        || input.sources.length + input.bindings.length + input.grants.length > limits.maxValues) {
        throw new TypeError('Invalid element reference lifecycle input');
    }
    let bytes = 0;
    for (const source of input.sources) {
        if (!(source.bundle instanceof ArrayBuffer) || !Number.isSafeInteger(source.primarySourceId) || source.primarySourceId < 0
            || source.primarySourceId > 0xffffffff || typeof source.context !== 'boolean') throw new TypeError('Invalid retained reference source');
        bytes += source.bundle.byteLength;
        if (bytes > limits.maxBytes) throw new RangeError('Reference source bundle input byte limit exceeded');
        for (const bound of [source.maxDepth, source.maxWork]) {
            if (bound !== undefined && (!Number.isSafeInteger(bound) || bound < 1)) throw new RangeError('Invalid reference source bounds');
        }
        if (source.unresolved !== undefined && !['mandatory', 'warning', 'ignore', 'neutral'].includes(source.unresolved)) throw new TypeError('Invalid reference source policy');
    }
    const names = new Set<string>();
    for (const binding of input.bindings) {
        if (!index(binding.source) || typeof binding.name !== 'string' || !binding.name || names.has(binding.name)
            || typeof binding.select !== 'string') throw new TypeError('Invalid reference source binding');
        names.add(binding.name);
    }
    for (const grant of input.grants) {
        if (!Array.isArray(grant) || grant.length !== 2 || !index(grant[0]) || !index(grant[1])) throw new TypeError('Invalid reference crossing grant');
    }
}
