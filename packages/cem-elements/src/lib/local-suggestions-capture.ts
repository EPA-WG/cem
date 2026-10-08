import type { CemValueArtifactLimits } from './native-values.js';

/** Explicit authored DOM-to-XML ingress. Labels and options share one original native owner. */
export function captureLocalSuggestions(document: Document, options: HTMLTemplateElement | undefined,
    labels: readonly HTMLTemplateElement[], limits: CemValueArtifactLimits): string {
    const ns = 'http://www.w3.org/1999/xhtml', xmlns = 'http://www.w3.org/2000/xmlns/';
    const capture = document.createElementNS(ns, 'capture');
    let count = 0, bytes = 0;
    const append = (source: HTMLTemplateElement | undefined, tag: string) => {
        const wrapper = document.createElementNS(ns, tag);
        if (source) {
            const walker = document.createTreeWalker(source.content, NodeFilter.SHOW_ALL);
            const depths = new WeakMap<Node, number>(); depths.set(source.content, 0);
            for (let node = walker.nextNode(); node; node = walker.nextNode()) {
                const depth = (node.parentNode ? depths.get(node.parentNode) ?? 0 : 0) + 1; depths.set(node, depth);
                if (++count > limits.maxValues || depth + 2 > limits.maxDepth) throw new RangeError('Local suggestions capture node/depth limit exceeded');
                if (node instanceof HTMLTemplateElement) throw new Error('Local suggestions cannot contain nested source templates');
                if ((node.nodeValue?.length ?? 0) > limits.maxBytes) throw new RangeError('Local suggestions capture byte limit exceeded');
                bytes += new TextEncoder().encode(node.nodeValue ?? '').byteLength;
                if (node instanceof Element) for (const attribute of node.attributes) {
                    if (attribute.name.length + attribute.value.length > limits.maxBytes) throw new RangeError('Local suggestions capture byte limit exceeded');
                    bytes += new TextEncoder().encode(attribute.name + attribute.value).byteLength;
                }
                if (bytes > limits.maxBytes) throw new RangeError('Local suggestions capture byte limit exceeded');
            }
            const namespaces = new Map<string, string>();
            for (let ancestor: Element | null = source; ancestor; ancestor = ancestor.parentElement) {
                if (++count > limits.maxValues) throw new RangeError('Local suggestions capture work limit exceeded');
                for (const attribute of ancestor.attributes) if ((attribute.namespaceURI === xmlns || attribute.name === 'xmlns' || attribute.name.startsWith('xmlns:')) && !namespaces.has(attribute.name)) namespaces.set(attribute.name, attribute.value);
            }
            for (const [name, value] of namespaces) wrapper.setAttributeNS(xmlns, name, value);
            if (tag !== 'options') {
                if (source.getAttribute('type') !== 'text/cem-ml' || source.content.childElementCount) throw new Error('Local labels require inert text/cem-ml template text');
                wrapper.setAttribute('type', 'text/cem-ml');
            }
            wrapper.append(source.content.cloneNode(true));
        }
        capture.append(wrapper);
    };
    append(options, 'options');
    for (const source of labels) append(source, source.slot === 'option' ? 'option-label' : 'group-label');
    const text = new XMLSerializer().serializeToString(capture);
    if (new TextEncoder().encode(text).byteLength > limits.maxBytes) throw new RangeError('Local suggestions capture byte limit exceeded');
    return text;
}

/** A source moving under a different namespace environment starts a fresh capture. */
export function localSuggestionsNamespaceStamp(sources: readonly HTMLTemplateElement[], limits: CemValueArtifactLimits): string {
    let work = 0, bytes = 0;
    return JSON.stringify(sources.map(source => {
        const bindings = new Map<string, string>();
        for (let node: Element | null = source; node; node = node.parentElement) {
            if (++work > limits.maxValues) throw new RangeError('Local namespace capture work limit exceeded');
            for (const attribute of node.attributes) {
                if (++work > limits.maxValues) throw new RangeError('Local namespace capture work limit exceeded');
                if (attribute.name === 'xmlns' || attribute.name.startsWith('xmlns:')) {
                    bytes += attribute.name.length + attribute.value.length;
                    if (bytes > limits.maxBytes) throw new RangeError('Local namespace capture byte limit exceeded');
                    if (!bindings.has(attribute.name)) bindings.set(attribute.name, attribute.value);
                }
            }
        }
        return [...bindings].sort(([a], [b]) => a.localeCompare(b));
    }));
}
