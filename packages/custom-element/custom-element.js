import { CemElementRuntime } from '../cem-elements/dist/index.js';

const CUSTOM_ELEMENT_TAG = 'custom-element';
const LEGACY_TEMPLATE_LANG = 'custom-element-v0';
const runtimeByHost = new WeakMap();
const registeredDeclarations = new WeakSet();
const inlineInstances = new WeakMap();
let inlineTagSequence = 0;

export function mix(objTo, objFrom) {
    for (const key of Object.keys(objFrom)) {
        objTo[key] = objFrom[key];
    }
    return objTo;
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

export function xml2dom(xmlString) {
    return new DOMParser().parseFromString(xmlString, 'application/xml');
}

export function xmlString(node) {
    return new XMLSerializer().serializeToString(node);
}

export function obj2node(value, tag, doc = document) {
    const node = doc.createElement(tag);
    if (value === null || value === undefined) {
        return node;
    }
    if (typeof value !== 'object') {
        node.textContent = String(value);
        return node;
    }
    if (value instanceof Node) {
        node.append(value);
        return node;
    }
    for (const [key, childValue] of Object.entries(value)) {
        node.append(obj2node(childValue, key, doc));
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
        registerDeclarationElement(this);
    }
}

function customElementClassForHost(host, runtime) {
    return class HostCustomElement extends host.HTMLElement {
        static observedAttributes = CustomElement.observedAttributes;

        connectedCallback() {
            registerDeclarationElement(this, runtime);
        }
    };
}

function registerDeclarationElement(declaration, runtime = runtimeForTarget(declaration)) {
    if (registeredDeclarations.has(declaration)) {
        return;
    }
    const inline = !declaration.getAttribute('tag');
    if (inline) {
        declaration.setAttribute('tag', nextInlineTag(declaration));
    }
    normalizeLegacyDeclaration(declaration);
    runtime.registerDeclaration(declaration);
    registeredDeclarations.add(declaration);
    if (inline) {
        appendInlineInstance(declaration, runtime);
    }
}

function runtimeForTarget(target) {
    const host = target?.ownerDocument?.defaultView ?? globalThis;
    return getCustomElementRuntime(host);
}

function directTemplateChildren(element) {
    return Array.from(element.children).filter((child) => child.localName === 'template');
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

function appendInlineInstance(declaration, runtime) {
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
    inlineInstances.set(declaration, instance);
    runtime.whenDeclarationSettled(declaration).then(() => {
        if (!instance.isConnected && declaration.isConnected) {
            declaration.append(instance);
        }
    });
}

if (typeof window !== 'undefined' && window.customElements && !window.customElements.get(CUSTOM_ELEMENT_TAG)) {
    window.customElements.define( CUSTOM_ELEMENT_TAG, CustomElement );
}

export default CustomElement;
