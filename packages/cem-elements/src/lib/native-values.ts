/** Opaque native CEM artifact. JavaScript transports bytes; CEM-ML owns decoding. */
export interface NativeCemValue {
    kind: 'cem-native-value-v1';
    artifact: ArrayBuffer;
    contentHash: string;
    index: number;
}
export interface NativeCemAttributeBinding { name: string; value: NativeCemValue }
export interface NativeCemSliceBinding extends NativeCemAttributeBinding {
    /** Native event attribute whose authoritative values form this slice. */
    attribute?: string;
}
export interface CemValueArtifactLimits { maxBytes: number; maxValues: number; maxDepth: number }
export const DEFAULT_CEM_VALUE_ARTIFACT_LIMITS: Readonly<CemValueArtifactLimits> = Object.freeze({
    maxBytes: 16 * 1024 * 1024, maxValues: 100_000, maxDepth: 128,
});
export function sameNativeCemValue(a?: NativeCemValue, b?: NativeCemValue): boolean {
    return a === b || !!a && !!b && a.contentHash === b.contentHash && a.index === b.index;
}
export function lowerCemValueArtifactLimits(parent: CemValueArtifactLimits, local: Partial<CemValueArtifactLimits> = {}): CemValueArtifactLimits {
    const result = { ...parent, ...local };
    for (const key of ['maxBytes', 'maxValues', 'maxDepth'] as const) {
        if (!Number.isSafeInteger(result[key]) || result[key] < 1) throw new RangeError(`nativeValueLimits.${key} must be a positive safe integer`);
        if (result[key] > parent[key]) throw new RangeError(`cem.a.cap_relaxation_denied: nativeValueLimits.${key} exceeds its parent ceiling`);
    }
    return result;
}

/** Explicit binary envelope for DOM hydration/JSON state exports, never an AST projection. */
export function exportNativeCemAttributes(bindings: readonly NativeCemAttributeBinding[]): Record<string, unknown> {
    const artifacts: { contentHash: string; base64: string }[] = [];
    const indexes = new Map<ArrayBuffer, number>();
    const attributes = bindings.map(({ name, value }) => {
        let artifact = indexes.get(value.artifact);
        if (artifact === undefined) {
            artifact = artifacts.length;
            indexes.set(value.artifact, artifact);
            let binary = '';
            for (const byte of new Uint8Array(value.artifact)) binary += String.fromCharCode(byte);
            artifacts.push({ contentHash: value.contentHash, base64: btoa(binary) });
        }
        return { name, artifact, index: value.index };
    });
    return { kind: 'cem-native-attributes-v1', artifacts, attributes };
}
export function importNativeCemAttributes(input: unknown, limits = DEFAULT_CEM_VALUE_ARTIFACT_LIMITS): NativeCemAttributeBinding[] {
    if (!input || typeof input !== 'object') throw new TypeError('Invalid native CEM attribute envelope');
    const envelope = input as { kind?: unknown; artifacts?: unknown; attributes?: unknown };
    if (envelope.kind !== 'cem-native-attributes-v1' || !Array.isArray(envelope.artifacts) || !Array.isArray(envelope.attributes)
        || envelope.attributes.length > limits.maxValues || envelope.artifacts.length > limits.maxValues) throw new TypeError('Invalid native CEM attribute envelope');
    let total = 0;
    const artifactCount = envelope.artifacts.length;
    const artifacts = envelope.artifacts.map((record: { contentHash?: unknown; base64?: unknown }) => {
        if (typeof record?.base64 !== 'string' || typeof record.contentHash !== 'string') throw new TypeError('Invalid native CEM artifact envelope');
        total += Math.ceil(record.base64.length * 3 / 4);
        if (total > limits.maxBytes + 2 * artifactCount) throw new RangeError('Native CEM artifact byte limit exceeded');
        const binary = atob(record.base64);
        return { artifact: Uint8Array.from(binary, c => c.charCodeAt(0)).buffer, contentHash: record.contentHash };
    });
    const seen = new Set<string>();
    return envelope.attributes.map((record: { name?: unknown; artifact?: unknown; index?: unknown }) => {
        if (typeof record?.name !== 'string' || !record.name || seen.has(record.name) || !Number.isSafeInteger(record.artifact)
            || !Number.isSafeInteger(record.index) || (record.index as number) < 0 || !artifacts[record.artifact as number]) throw new TypeError('Invalid native CEM attribute binding');
        seen.add(record.name);
        return { name: record.name, value: { kind: 'cem-native-value-v1', ...artifacts[record.artifact as number], index: record.index as number } };
    });
}

/** Saved slices carry opaque artifacts, including an optional event-value wrapper. */
export function exportNativeCemSlices(bindings: readonly NativeCemSliceBinding[]): Record<string, unknown> {
    return { kind: 'cem-native-slices-v1', values: exportNativeCemAttributes(bindings),
        wrappers: bindings.filter(b => b.attribute !== undefined).map(b => ({ name: b.name, attribute: b.attribute })) };
}
export function importNativeCemSlices(input: unknown, limits = DEFAULT_CEM_VALUE_ARTIFACT_LIMITS): NativeCemSliceBinding[] {
    const envelope = input as { kind?: unknown; values?: unknown; wrappers?: unknown } | null;
    if (envelope?.kind !== 'cem-native-slices-v1' || !Array.isArray(envelope.wrappers) || envelope.wrappers.length > limits.maxValues) throw new TypeError('Invalid native CEM slice envelope');
    const bindings: NativeCemSliceBinding[] = importNativeCemAttributes(envelope.values, limits);
    const byName = new Map(bindings.map(binding => [binding.name, binding]));
    const names = new Set<string>();
    for (const wrapper of envelope.wrappers) {
        const binding = byName.get(wrapper?.name);
        if (!binding || names.has(binding.name) || typeof wrapper.attribute !== 'string' || !wrapper.attribute) throw new TypeError('Invalid native slice wrapper');
        binding.attribute = wrapper.attribute;
        names.add(binding.name);
    }
    return bindings;
}
