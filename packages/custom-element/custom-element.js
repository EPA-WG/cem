import { CemElementRuntime } from '../cem-elements/dist/index.js';

const CUSTOM_ELEMENT_TAG = 'custom-element';
// Untyped legacy browser templates use the exact compatibility selector accepted by the substrate.
// The `custom-element-xslt` identity is reserved for native converter/CLI inputs and deliberately
// remains on the browser DOM path.
const LEGACY_TEMPLATE_LANG = 'custom-element-v0';
const runtimeByHost = new WeakMap();
const registeredDeclarations = new WeakSet();
const inlineInstances = new WeakMap();
let inlineTagSequence = 0;
const VALID_OBJECT_NODE_TAG = /^[_a-zA-Z][-_:a-zA-Z0-9]*$/;

export function mix(objTo, objFrom) {
    for (const key of Object.keys(objFrom)) {
        objTo[key] = objFrom[key];
    }
    return objTo;
}

export function deepEqual(a, b) {
    if (a === b) {
        return true;
    }
    if (typeof a !== 'object' || a === null || typeof b !== 'object' || b === null) {
        return false;
    }
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    if (aKeys.length !== bKeys.length) {
        return false;
    }
    return aKeys.every((key) => Object.prototype.hasOwnProperty.call(b, key) && deepEqual(a[key], b[key]));
}

export function cloneAs(sourceNode, tag) {
    const clone = sourceNode.ownerDocument.createElementNS(sourceNode.namespaceURI, tag);
    for (const attribute of sourceNode.attributes) {
        clone.setAttribute(attribute.name, attribute.value);
    }
    for (const child of sourceNode.childNodes) {
        clone.append(child.cloneNode(true));
    }
    return clone;
}

export function mergeAttr(from, to) {
    for (const attribute of from.attributes) {
        if (attribute.name.startsWith('xmlns')) {
            continue;
        }
        if (attribute.namespaceURI) {
            to.setAttributeNS(attribute.namespaceURI, attribute.name, attribute.value);
        } else {
            to.setAttribute(attribute.name, attribute.value);
        }
        if (attribute.localName === 'value' && 'value' in to) {
            to.value = attribute.value;
        }
    }

    for (const attribute of [...to.attributes]) {
        const sourceHasAttribute = attribute.namespaceURI
            ? from.hasAttributeNS(attribute.namespaceURI, attribute.localName)
            : from.hasAttribute(attribute.name);
        if (!sourceHasAttribute) {
            if (attribute.namespaceURI) {
                to.removeAttributeNS(attribute.namespaceURI, attribute.localName);
            } else {
                to.removeAttribute(attribute.name);
            }
        }
    }
}

export function xml2dom(xmlString) {
    return new DOMParser().parseFromString(xmlString, 'application/xml');
}

export function xmlString(node) {
    return new XMLSerializer().serializeToString(node);
}

export function obj2node(value, tag, doc = document) {
    const ownerDocument = doc.ownerDocument ?? doc;
    const node = createObjectNode(tag, ownerDocument);
    if (value === null || value === undefined) {
        return node;
    }
    if (['string', 'number', 'boolean', 'bigint'].includes(typeof value)) {
        node.textContent = String(value);
        return node;
    }
    if (typeof value === 'function' || typeof value === 'symbol') {
        return node;
    }
    if (isNode(value)) {
        node.append(value);
        return node;
    }
    if (Array.isArray(value)) {
        const arrayNode = ownerDocument.createElement('array');
        for (const item of value) {
            arrayNode.append(obj2node(item, tag, ownerDocument));
        }
        return arrayNode;
    }
    if (typeof FormData !== 'undefined' && value instanceof FormData) {
        const formDataNode = ownerDocument.createElement('form-data');
        for (const [name, entryValue] of value) {
            formDataNode.append(obj2node(entryValue, name, ownerDocument));
        }
        return formDataNode;
    }
    for (const [key, childValue] of Object.entries(value)) {
        if (typeof childValue === 'function' || (typeof Window !== 'undefined' && childValue instanceof Window)) {
            continue;
        }
        if (isNode(childValue) && !['data', 'value'].includes(key)) {
            continue;
        }
        if (typeof childValue !== 'object' && VALID_OBJECT_NODE_TAG.test(key)) {
            node.setAttribute(key, String(childValue));
        } else {
            node.append(obj2node(childValue, key, ownerDocument));
        }
    }
    return node;
}

export function tagUid(node) {
    let sequence = 1;
    for (const element of node.querySelectorAll?.('*') ?? []) {
        element.setAttribute('data-dce-id', String(sequence));
        sequence += 1;
    }
    return node;
}

export function getCustomElementRuntime(host = globalThis, options = {}) {
    const existing = runtimeByHost.get(host);
    if (existing) {
        return existing;
    }
    const runtime = new CemElementRuntime({
        ...options,
        declarationTag: CUSTOM_ELEMENT_TAG,
    });
    runtimeByHost.set(host, runtime);
    return runtime;
}

export const customElementRuntime = getCustomElementRuntime();

export function installCustomElementRuntime(host = globalThis, options = {}) {
    const runtime = getCustomElementRuntime(host, options);
    if (!host.customElements.get(CUSTOM_ELEMENT_TAG)) {
        const ElementClass = host === globalThis ? CustomElement : customElementClassForHost(host, runtime);
        host.customElements.define(CUSTOM_ELEMENT_TAG, ElementClass);
    }
    return runtime;
}

export function diagnosticsFor(target) {
    return runtimeForTarget(target).diagnosticsFor(target);
}

export function whenDeclarationSettled(declaration) {
    return runtimeForTarget(declaration).whenDeclarationSettled(declaration);
}

export function whenRenderSettled(instance) {
    return runtimeForTarget(instance).whenRenderSettled(instance);
}

export function normalizeLegacyDeclaration(declaration) {
    const templates = directTemplateChildren(declaration);
    if (templates.length !== 1) {
        return declaration;
    }
    const template = templates[0];
    if (!template.hasAttribute('lang') && !template.hasAttribute('type')) {
        template.setAttribute('lang', LEGACY_TEMPLATE_LANG);
    }
    return declaration;
}

export class CustomElement extends HTMLElement {
    static observedAttributes = ['src', 'tag', 'hidden'];

    connectedCallback() {
        wrapImplicitInlineTemplate(this);
        registerDeclarationElement(this);
    }
}

function customElementClassForHost(host, runtime) {
    return class HostCustomElement extends host.HTMLElement {
        static observedAttributes = CustomElement.observedAttributes;

        connectedCallback() {
            wrapImplicitInlineTemplate(this);
            registerDeclarationElement(this, runtime);
        }
    };
}

function registerDeclarationElement(declaration, runtime = runtimeForTarget(declaration)) {
    if (registeredDeclarations.has(declaration)) {
        return;
    }
    const inline = !declaration.getAttribute('tag');
    const srcPayloadNodes = declaration.hasAttribute('src') ? Array.from(declaration.childNodes) : [];
    const detachedSrcPayloads = detachSrcPayloadNodes(srcPayloadNodes);
    if (inline) {
        declaration.setAttribute('tag', nextInlineTag(declaration));
    }
    normalizeLegacyDeclaration(declaration);
    try {
        runtime.registerDeclaration(declaration);
        registeredDeclarations.add(declaration);
    } finally {
        restoreDetachedSrcPayloadNodes(detachedSrcPayloads);
    }
    if (inline) {
        appendInlineInstance(declaration, runtime, srcPayloadNodes);
    }
}

function runtimeForTarget(target) {
    const host = target?.ownerDocument?.defaultView ?? globalThis;
    return getCustomElementRuntime(host);
}

function directTemplateChildren(element) {
    return Array.from(element.children).filter((child) => child.localName === 'template');
}

function createObjectNode(tag, doc) {
    if (VALID_OBJECT_NODE_TAG.test(tag)) {
        return doc.createElement(tag);
    }
    const node = doc.createElement('dce-object');
    node.setAttribute('dce-object-name', tag);
    return node;
}

function isNode(value) {
    return value !== null && typeof value === 'object' && typeof value.nodeType === 'number';
}

function detachSrcPayloadNodes(nodes) {
    return nodes.map((node) => {
        const marker = node.ownerDocument.createComment('custom-element src payload');
        node.before(marker);
        node.remove();
        return { marker, node };
    });
}

function restoreDetachedSrcPayloadNodes(detachedPayloads) {
    for (const { marker, node } of detachedPayloads) {
        marker.replaceWith(node);
    }
}

function wrapImplicitInlineTemplate(declaration) {
    if (
        !declaration.hasAttribute('tag') ||
        declaration.hasAttribute('src') ||
        directTemplateChildren(declaration).length > 0
    ) {
        return;
    }
    const template = declaration.ownerDocument.createElement('template');
    for (const child of [...declaration.childNodes]) {
        template.content.append(child);
    }
    declaration.append(template);
}

function nextInlineTag(declaration) {
    const existing = declaration.getAttribute('data-custom-element-inline-tag');
    if (existing) {
        return existing;
    }
    inlineTagSequence += 1;
    const tag = `custom-element-inline-${inlineTagSequence}`;
    declaration.setAttribute('data-custom-element-inline-tag', tag);
    return tag;
}

function appendInlineInstance(declaration, runtime, payloadNodes = []) {
    const tag = declaration.getAttribute('tag');
    if (!tag || inlineInstances.has(declaration)) {
        return;
    }
    const instance = declaration.ownerDocument.createElement(tag);
    for (const attribute of declaration.attributes) {
        if (['tag', 'src', 'hidden', 'data-custom-element-inline-tag'].includes(attribute.name)) {
            continue;
        }
        instance.setAttribute(attribute.name, attribute.value);
    }
    for (const node of payloadNodes) {
        instance.append(payloadNodeContent(node));
    }
    inlineInstances.set(declaration, instance);
    runtime.whenDeclarationSettled(declaration).then(() => {
        if (!instance.isConnected && declaration.isConnected) {
            declaration.append(instance);
        }
    });
}

function payloadNodeContent(node) {
    if (node.nodeType === Node.ELEMENT_NODE && node.localName === 'template') {
        return node.content.cloneNode(true);
    }
    return node.cloneNode(true);
}

if (typeof window !== 'undefined' && window.customElements && !window.customElements.get(CUSTOM_ELEMENT_TAG)) {
    window.customElements.define(CUSTOM_ELEMENT_TAG, CustomElement);
}

export default CustomElement;
