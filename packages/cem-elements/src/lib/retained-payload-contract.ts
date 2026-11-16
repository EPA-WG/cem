/** Declaration metadata only. Matching a node never grants source or recovery authority. */
export interface CemPayloadName { readonly namespace: string | null; readonly localName: string }
export interface CemPayloadVocabulary { readonly item: CemPayloadName; readonly group: CemPayloadName | null }
export interface CemRetainedPayloadContract {
    readonly contract: 'cem-retained-payload/1';
    readonly topology: 'flat' | 'grouped' | 'recursive';
    readonly vocabularies: readonly CemPayloadVocabulary[];
}
export const CEM_PAYLOAD_HEADER_NAMES = Object.freeze(['payload-item', 'payload-group', 'payload-topology'] as const);
type Header = Readonly<Record<string, string | undefined>>;
const start = 'A-Z_a-z\\u00C0-\\u00D6\\u00D8-\\u00F6\\u00F8-\\u02FF\\u0370-\\u037D\\u037F-\\u1FFF\\u200C-\\u200D\\u2070-\\u218F\\u2C00-\\u2FEF\\u3001-\\uD7FF\\uF900-\\uFDCF\\uFDF0-\\uFFFD\\u{10000}-\\u{EFFFF}';
// XML NCNames intentionally match combining marks as individual name characters.
// eslint-disable-next-line no-misleading-character-class
const ncname = new RegExp(`^[${start}][${start}0-9.\\-\\u00B7\\u0300-\\u036F\\u203F-\\u2040]*$`, 'u');
const invalid = (): never => { throw new TypeError('cem.payload_contract.invalid: expected bounded, unambiguous item/group declaration metadata'); };
const trim = (value: string) => value.replace(/^[\t\n\f\r ]+|[\t\n\f\r ]+$/g, '');
function name(value: string): CemPayloadName {
    let namespace: string | null = null, localName = value;
    if (value.startsWith('Q{')) {
        const end = value.indexOf('}');
        if (end < 2) return invalid();
        namespace = value.slice(2, end) || null; localName = value.slice(end + 1);
    }
    if ([...localName].length > 256 || !ncname.test(localName)
        || namespace !== null && ([...namespace].length > 1024 || /[\p{White_Space}\p{Cc}\uFEFF{}]/u.test(namespace))) return invalid();
    return Object.freeze({ namespace, localName });
}
function block(header: Header): CemRetainedPayloadContract | null {
    if (!CEM_PAYLOAD_HEADER_NAMES.some(key => Object.hasOwn(header, key))) return null;
    if (CEM_PAYLOAD_HEADER_NAMES.some(key => Object.hasOwn(header, key)
        && (typeof header[key] !== 'string' || header[key].length > 8192 || new TextEncoder().encode(header[key]).length > 8192))) return invalid();
    const items = trim(header['payload-item'] ?? '');
    if (!items) return invalid();
    const itemWords = items.split(/[\t\n\f\r ]+/);
    if (itemWords.length > 8) return invalid();
    const itemNames = itemWords.map(name);
    const topology = trim(header['payload-topology'] ?? 'flat');
    const groups = header['payload-group'];
    if (!['flat', 'grouped', 'recursive'].includes(topology) || itemNames.length > 8
        || (topology === 'grouped') !== (groups !== undefined)) return invalid();
    const groupWords = groups === undefined ? [] : trim(groups).split(/[\t\n\f\r ]+/);
    if (groupWords.length > 8) return invalid();
    const groupNames = groupWords.map(name);
    if (groups !== undefined && groupNames.length !== itemNames.length) return invalid();
    const seen = new Set<string>();
    for (const entry of [...itemNames, ...groupNames]) {
        const key = JSON.stringify([entry.namespace, entry.localName]);
        if (seen.has(key)) return invalid(); seen.add(key);
    }
    const vocabularies = Object.freeze(itemNames.map((item, index) => Object.freeze({ item, group: groupNames[index] ?? null })));
    return Object.freeze({ contract: 'cem-retained-payload/1', topology: topology as CemRetainedPayloadContract['topology'], vocabularies });
}
/** Blocks are atomic. A template cannot silently inherit individual fields from its owner. */
export function resolveRetainedPayloadContract(owner: Header, template: Header = {}): CemRetainedPayloadContract | null {
    const own = block(owner), inner = block(template);
    if (own && inner && JSON.stringify(own) !== JSON.stringify(inner)) return invalid();
    return own ?? inner;
}
/** Copy an already native-admitted compiled metadata value into immutable runtime state. */
export function freezeRetainedPayloadContract(value: CemRetainedPayloadContract | null | undefined): CemRetainedPayloadContract | null {
    if (!value) return null;
    return Object.freeze({ ...value, vocabularies: Object.freeze(value.vocabularies.map(vocabulary => Object.freeze({
        item: Object.freeze({ ...vocabulary.item }), group: vocabulary.group && Object.freeze({ ...vocabulary.group }),
    }))) });
}
/** Role classification only: do not traverse DOM, clone payloads or manufacture retained keys. */
export function classifyRetainedPayloadElement(contract: CemRetainedPayloadContract | null,
    node: { readonly tag: string; readonly namespace: string | null }): { role: 'item' | 'group'; vocabulary: number } | null {
    const localName = node.tag.slice(node.tag.lastIndexOf(':') + 1);
    for (const [vocabulary, names] of (contract?.vocabularies ?? []).entries()) for (const role of ['item', 'group'] as const) {
        const expected = names[role];
        if (expected?.localName === localName && expected.namespace === (node.namespace || null)) return { role, vocabulary };
    }
    return null;
}
