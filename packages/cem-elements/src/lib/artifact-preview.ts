import { remapCemArtifactCss } from './internal/runtime-support/cem-ql-render.js';

export interface CemPreviewArtifact { path: string; contentType: string; bytes: Uint8Array }
export interface CemArtifactTree { readonly resourceSet: string; dispose(): void }
interface Tree { artifacts: Map<string, CemPreviewArtifact>; revoked: Set<() => void> }
const trees = new Map<string, Tree>();
const mounts = new WeakMap<HTMLIFrameElement, CemArtifactPreview>();
const root = 'https://cem-artifact.invalid/';
const decoder = new TextDecoder('utf-8', { fatal: true });
const htmlTypes = new Set(['text/html', 'application/xhtml+xml']);
const imageTypes = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/avif', 'image/bmp', 'image/x-icon', 'image/svg+xml']);
const mediaType = (type: string) => type.split(';')[0].trim().toLowerCase();
const htmlNamespace = 'http://www.w3.org/1999/xhtml';
const svgNamespace = 'http://www.w3.org/2000/svg';
const mathNamespace = 'http://www.w3.org/1998/Math/MathML';
const xmlnsNamespace = 'http://www.w3.org/2000/xmlns/';
const svgElements = new Set(['svg', 'g', 'title', 'desc', 'rect', 'circle', 'ellipse', 'line', 'polyline', 'polygon', 'path', 'text', 'tspan']);
const mathElements = new Set(['math', 'mrow', 'mi', 'mo', 'mn', 'mspace', 'mfrac', 'msup', 'msub', 'msubsup', 'msqrt', 'mroot', 'mover', 'munder', 'munderover']);
const mathTokens = new Set(['mi', 'mo', 'mn', 'mspace']);
const foreignCommonAttributes = new Set(['id', 'class', 'role', 'lang', 'dir', 'aria-label', 'aria-labelledby', 'aria-describedby', 'aria-hidden']);
const svgAttributes = new Set(['viewBox', 'preserveAspectRatio', 'x', 'y', 'width', 'height', 'rx', 'ry', 'cx', 'cy', 'r', 'x1', 'x2', 'y1', 'y2',
    'd', 'points', 'pathLength', 'transform', 'fill', 'fill-rule', 'fill-opacity', 'stroke', 'stroke-width', 'stroke-opacity', 'stroke-linecap', 'stroke-linejoin',
    'stroke-miterlimit', 'stroke-dasharray', 'stroke-dashoffset', 'opacity', 'color', 'vector-effect', 'dx', 'dy', 'rotate', 'text-anchor', 'dominant-baseline',
    'font-family', 'font-size', 'font-style', 'font-weight', 'textLength', 'lengthAdjust']);
const mathAttributes = new Set(['display', 'alttext', 'mathvariant', 'mathsize', 'mathcolor', 'mathbackground', 'displaystyle', 'scriptlevel', 'stretchy',
    'symmetric', 'fence', 'separator', 'accent', 'accentunder', 'movablelimits', 'largeop', 'lspace', 'rspace', 'minsize', 'maxsize', 'linethickness', 'bevelled',
    'subscriptshift', 'superscriptshift', 'width', 'height', 'depth']);
const paintAttributes = new Set(['fill', 'stroke', 'color', 'mathcolor', 'mathbackground']);

function literalPaint(value: string): boolean {
    const paint = value.trim();
    if (/^(none|currentcolor)$/i.test(paint)) return true;
    if (/^(inherit|initial|unset|revert)$/i.test(paint)) return false;
    // No url(), var(), escapes, paint servers or cross-document references.
    return (/^[a-z]+$/i.test(paint) || /^#[\da-f]{3,8}$/i.test(paint)
        || /^(rgba?|hsla?)\([\d\s.,%+\-/]+\)$/i.test(paint)) && CSS.supports('color', paint);
}

/** A passive browser projection subset, independent of native source validation. */
function admitForeignElement(element: Element, report: (message: string) => void): boolean {
    const namespace = element.namespaceURI;
    const elements = namespace === svgNamespace ? svgElements : namespace === mathNamespace ? mathElements : undefined;
    if (!elements?.has(element.localName) || element.hasAttribute('xmlns') && element.getAttribute('xmlns') !== namespace) {
        report(`Blocked artifact element: ${element.localName}`); return false;
    }
    const attributes = namespace === svgNamespace ? svgAttributes : mathAttributes;
    for (const attribute of [...element.attributes]) {
        const declaration = attribute.name === 'xmlns' && (!attribute.namespaceURI || attribute.namespaceURI === xmlnsNamespace);
        if (declaration || !attribute.namespaceURI && (foreignCommonAttributes.has(attribute.name) || attributes.has(attribute.name))
            && (!paintAttributes.has(attribute.name) || literalPaint(attribute.value))) continue;
        report(`Blocked artifact attribute: ${element.localName}[${attribute.name}]`);
        element.removeAttributeNode(attribute);
    }
    return true;
}

function permittedForeignChild(element: Element): boolean {
    const parent = element.parentElement;
    if (!parent || ![svgNamespace, mathNamespace].includes(parent.namespaceURI ?? '')) return true;
    // HTML integration points never grant the HTML or another foreign namespace
    // inside a token/description. Those admitted nodes retain plain text only.
    if (parent.namespaceURI !== element.namespaceURI) return false;
    if (parent.namespaceURI === mathNamespace && mathTokens.has(parent.localName)) return false;
    if (parent.namespaceURI === svgNamespace) {
        if (['title', 'desc'].includes(parent.localName)) return false;
        if (['text', 'tspan'].includes(parent.localName) && element.localName !== 'tspan') return false;
    }
    return true;
}
// Paths are output metadata, never ambient source addresses. URL canonicalization
// also catches dot aliases, encoded traversal and percent-encoding collisions.
function pathUrl(path: string): URL {
    if (!path || path.startsWith('/') || path.includes('\\') || /^[a-z][\w+.-]*:/i.test(path) || /[?#]/.test(path) || [...path].some(ch => ch.charCodeAt(0) < 32 || ch.charCodeAt(0) === 127)) throw new Error(`cem.preview.artifact_path: ${path}`);
    const url = new URL(path.split('/').map(encodeURIComponent).join('/'), root);
    if (url.href === root || path.split('/').some(part => !part || part === '.' || part === '..')) throw new Error(`cem.preview.artifact_path: ${path}`);
    return url;
}
/** Complete verified byte sets are registered atomically, outside UI/AST payloads. */
export function registerCemArtifactTree(artifacts: readonly CemPreviewArtifact[]): CemArtifactTree {
    if (artifacts.length > 128) throw new RangeError('cem.preview.artifact_limit');
    const tree: Tree = { artifacts: new Map(), revoked: new Set() };
    let length = 0;
    for (const artifact of artifacts) {
        const uri = pathUrl(artifact.path).href;
        length += artifact.bytes.byteLength;
        if (length > 16 * 1024 * 1024 || tree.artifacts.has(uri)) throw new RangeError('cem.preview.artifact_set');
        tree.artifacts.set(uri, { ...artifact, contentType: mediaType(artifact.contentType), bytes: artifact.bytes.slice() });
    }
    const id = crypto.randomUUID(); trees.set(id, tree);
    return Object.freeze({ resourceSet: JSON.stringify({ artifactTree: id }), dispose() {
        if (!trees.delete(id)) return;
        for (const revoke of tree.revoked) revoke();
        tree.revoked.clear(); tree.artifacts.clear();
    } });
}
/** Small declarative demonstrations may supply literal text/base64 instead of a retained handle. */
export function registerCemInlineArtifactTree(envelope: { artifacts?: readonly { path: string; contentType: string; text?: string; base64?: string }[] }): CemArtifactTree {
    if (!Array.isArray(envelope.artifacts) || envelope.artifacts.length > 128) throw new Error('cem.preview.artifact_set');
    let size = 0;
    return registerCemArtifactTree(envelope.artifacts.map(artifact => {
        if ((typeof artifact.text === 'string') === (typeof artifact.base64 === 'string')) throw new Error('cem.preview.artifact_encoding');
        size += (artifact.text?.length ?? artifact.base64?.length ?? 0);
        if (size > 24 * 1024 * 1024) throw new RangeError('cem.preview.artifact_limit');
        return { path: artifact.path, contentType: artifact.contentType,
            bytes: typeof artifact.text === 'string' ? new TextEncoder().encode(artifact.text) : Uint8Array.from(atob(artifact.base64!), ch => ch.charCodeAt(0)) };
    }));
}
export interface CemArtifactPreview {
    readonly ready: Promise<void>;
    /** Retain only the original settled document and live byte registry. */
    checkpoint(): (() => boolean) | undefined;
    /** Update query/fragment within the mounted file, retaining its document. */
    setLocation(path: string): void;
    dispose(): void;
}
export interface CemArtifactPreviewOptions {
    resourceSet: string;
    path: string;
    onNavigate?: (value: string) => void;
    onStatus?: (message: string) => void;
}

/** Parent-managed, script-free HTML output; no CEM runtime is installed in the frame. */
export function mountCemArtifactPreview(frame: HTMLIFrameElement, options: CemArtifactPreviewOptions): CemArtifactPreview {
    if (!frame.isConnected) throw new Error('cem.preview.detached_frame');
    options = { ...options };
    const envelope = JSON.parse(options.resourceSet) as { artifactTree?: string; artifacts?: Parameters<typeof registerCemInlineArtifactTree>[0]['artifacts'] };
    const inline = envelope.artifactTree ? undefined : registerCemInlineArtifactTree(envelope);
    const id = envelope.artifactTree ?? JSON.parse(inline!.resourceSet).artifactTree;
    const tree = trees.get(id);
    if (!tree) { inline?.dispose(); throw new Error('cem.preview.artifact_set_released'); }
    mounts.get(frame)?.dispose();
    let active = true, settled = false, revision = 0, cancelLoad: (() => void) | undefined, document: Document | undefined;
    const blobs = new Set<string>(), images = new Map<string, string>(), styles = new Map<string, string>();
    const messages = new Set<string>();
    const current = () => { if (!active || !frame.isConnected) throw new Error('cem.preview.disposed'); };
    const report = (message: string) => { if (messages.size < 100) messages.add(message.length > 1024 ? message.slice(0, 1024) + '…' : message); };
    const publish = () => { if (active) options.onStatus?.(messages.size ? [...messages].join('\n') : 'Artifact view ready'); };
    const blob = (bytes: Uint8Array | string, type: string) => {
        const url = URL.createObjectURL(new Blob([typeof bytes === 'string' ? bytes : new Uint8Array(bytes)], { type }));
        blobs.add(url); return url;
    };
    const location = (path: string) => {
        const suffix = path.search(/[?#]/), file = suffix < 0 ? path : path.slice(0, suffix);
        return new URL(file.split('/').map(encodeURIComponent).join('/') + (suffix < 0 ? '' : path.slice(suffix)), root);
    };
    const selected = location(options.path), selectedFile = new URL(selected); selectedFile.hash = ''; selectedFile.search = '';
    let selectedArtifact = tree.artifacts.get(selectedFile.href);
    const scroll = (fragment: string) => {
        try { document?.getElementById(decodeURIComponent(fragment))?.scrollIntoView(); }
        catch { report(`Invalid artifact fragment: ${fragment}`); }
    };
    const resolve = (specifier: string, base: string): URL | undefined => {
        try {
            const url = new URL(specifier, base);
            if (url.origin !== new URL(root).origin || url.username || url.password || specifier.includes('\\')) throw new Error();
            return url;
        } catch { report(`Blocked artifact reference: ${specifier}`); return undefined; }
    };
    const resource = (url: URL | undefined) => {
        if (!url) return undefined;
        const key = new URL(url); key.hash = ''; key.search = '';
        if (key.pathname.endsWith('/')) key.pathname += 'index.html';
        const artifact = tree.artifacts.get(key.href);
        if (!artifact) report(`Missing artifact: ${decodeURIComponent(key.pathname.slice(1))}`);
        return artifact;
    };
    const image = (specifier: string, base: string) => {
        const url = resolve(specifier, base), artifact = resource(url);
        if (!artifact || !url) return 'about:invalid#cem-artifact';
        if (!imageTypes.has(artifact.contentType)) { report(`Unsupported image type: ${artifact.path}`); return 'about:invalid#cem-artifact'; }
        const key = pathUrl(artifact.path).href;
        let value = images.get(key);
        if (!value) { value = blob(artifact.bytes, artifact.contentType); images.set(key, value); }
        return value + url.hash;
    };
    const css = async (source: string, base: string, inline: boolean, ancestors: readonly string[]): Promise<string> => {
        try {
            const plan = await remapCemArtifactCss(source, inline); current();
            const replacements: Record<string, string> = Object.create(null);
            for (const reference of plan.resources) {
                current();
                if (!reference.import && reference.specifier.startsWith('#')) continue;
                replacements[reference.specifier] = reference.import ? await sheet(reference.specifier, base, ancestors) : image(reference.specifier, base);
            }
            const result = await remapCemArtifactCss(source, inline, replacements); current(); return result.css;
        } catch (error) { current(); report(`CSS unavailable: ${String(error)}`); return ''; }
    };
    const sheet = async (specifier: string, base: string, ancestors: readonly string[]): Promise<string> => {
        const url = resolve(specifier, base), artifact = resource(url);
        if (!url || !artifact) return 'about:invalid#cem-artifact';
        const key = pathUrl(artifact.path).href;
        if (artifact.contentType !== 'text/css') { report(`Unsupported stylesheet type: ${artifact.path}`); return 'about:invalid#cem-artifact'; }
        if (ancestors.includes(key)) { report(`Cyclic stylesheet import: ${artifact.path}`); return 'about:invalid#cem-artifact'; }
        if (styles.has(key)) return styles.get(key)!;
        let source: string;
        try { source = decoder.decode(artifact.bytes); }
        catch { report(`Invalid UTF-8 stylesheet: ${artifact.path}`); return 'about:invalid#cem-artifact'; }
        const output = await css(source, key, false, [...ancestors, key]); current();
        const value = blob(output, 'text/css'); styles.set(key, value); return value;
    };
    const links = new WeakMap<Element, string>();
    const navigate = (event: Event) => {
        const target = event.target as Element | null;
        const link = target?.closest?.('a,area');
        if (!link) return;
        event.preventDefault(); event.stopPropagation();
        if (event.type === 'auxclick') return;
        const href = links.get(link); if (href === undefined) return;
        const url = resolve(href, selected.href); if (!url) { publish(); return; }
        const artifact = resource(url);
        const value = (artifact?.path ?? decodeURIComponent(url.pathname.slice(1))) + url.search + url.hash;
        if (url.pathname === selected.pathname && !url.search) {
            if (url.hash) scroll(url.hash.slice(1));
        }
        options.onNavigate?.(value); publish();
    };
    const stopForm = (event: Event) => event.preventDefault();
    const ready = (async () => {
        if (!selectedArtifact) throw new Error(`cem.preview.artifact_missing: ${options.path}`);
        if (!htmlTypes.has(selectedArtifact.contentType) || selectedArtifact.bytes.length > 256 * 1024) throw new Error('cem.preview.artifact_document_policy');
        // Template parsing is inert: no resource downloads or custom-element upgrades.
        const inert = frame.ownerDocument.createElement('template');
        inert.innerHTML = decoder.decode(selectedArtifact.bytes);
        for (const element of [...inert.content.querySelectorAll('*')]) {
            current();
            if (!inert.content.contains(element)) continue;
            if (!permittedForeignChild(element)) { report(`Blocked artifact element: ${element.localName}`); element.remove(); continue; }
            if (element.namespaceURI !== htmlNamespace) {
                if (!admitForeignElement(element, report)) element.remove();
                continue;
            }
            if (['script', 'base', 'meta', 'iframe', 'frame', 'frameset', 'object', 'embed', 'portal', 'template'].includes(element.localName)) { element.remove(); continue; }
            if (element.localName.includes('-')) { element.replaceWith(...element.childNodes); continue; }
            const style = element.getAttribute('style'), src = element.localName === 'img' ? element.getAttribute('src') : null;
            const href = element.getAttribute('href');
            if (element.localName === 'link') {
                if (element.getAttribute('rel')?.toLowerCase() === 'stylesheet' && href) {
                    const url = await sheet(href, selected.href, []); current();
                    const replacement = frame.ownerDocument.createElement('link'); replacement.rel = 'stylesheet'; replacement.href = url;
                    if (element.hasAttribute('media')) replacement.media = element.getAttribute('media')!;
                    element.replaceWith(replacement);
                } else element.remove();
                continue;
            }
            for (const attr of [...element.attributes]) {
                if (attr.name.toLowerCase().startsWith('on') || ['src', 'srcset', 'href', 'xlink:href', 'srcdoc', 'action', 'formaction', 'target', 'formtarget', 'ping', 'background', 'poster', 'data', 'codebase', 'manifest', 'autofocus', 'is', 'nonce', 'integrity', 'style'].includes(attr.name.toLowerCase())) element.removeAttributeNode(attr);
            }
            if (element.localName === 'style') element.textContent = await css(element.textContent ?? '', selected.href, false, []);
            if (style) element.setAttribute('style', await css(style, selected.href, true, []));
            if (src !== null) element.setAttribute('src', image(src, selected.href));
            if (['a', 'area'].includes(element.localName) && href !== null) { links.set(element, href); element.setAttribute('href', '#'); }
        }
        current();
        frame.setAttribute('sandbox', 'allow-same-origin'); frame.setAttribute('referrerpolicy', 'no-referrer'); frame.setAttribute('allow', '');
        // CSP source syntax admits the blob scheme, not individual object URLs.
        // Native CSS remapping and HTML admission supply only this mount's blobs.
        const policy = "default-src 'none'; script-src 'none'; style-src 'unsafe-inline' blob:; img-src blob:; base-uri 'none'; form-action 'none'; frame-src 'none'; connect-src 'none'; object-src 'none'";
        const owner = crypto.randomUUID();
        await new Promise<void>((resolve, reject) => {
            const loaded = () => {
                if (frame.contentDocument?.querySelector('meta[name="cem-artifact-owner"]')?.getAttribute('content') !== owner) return;
                frame.removeEventListener('load', loaded); cancelLoad = undefined; resolve();
            };
            cancelLoad = () => { frame.removeEventListener('load', loaded); reject(new Error('cem.preview.disposed')); };
            frame.addEventListener('load', loaded);
            frame.srcdoc = `<!doctype html><html><head><meta charset="utf-8"><meta http-equiv="Content-Security-Policy" content="${policy}"><meta name="referrer" content="no-referrer"><meta name="cem-artifact-owner" content="${owner}"><title>Artifact preview</title></head><body></body></html>`;
        });
        current(); document = frame.contentDocument ?? undefined;
        if (!document) throw new Error('cem.preview.document_unavailable');
        document.addEventListener('click', navigate, true); document.addEventListener('auxclick', navigate, true); document.addEventListener('submit', stopForm, true);
        // Move, rather than clone/import, to preserve inert link metadata. Adoption
        // happens only into the child realm, which has no component registrations.
        document.body.append(inert.content);
        if (selected.hash) scroll(selected.hash.slice(1));
        publish(); settled = true;
    })();
    const controller: CemArtifactPreview = Object.freeze({ ready, checkpoint() {
        if (!active || !settled || !document || trees.get(id) !== tree) return undefined;
        const retained = document, srcdoc = frame.srcdoc, version = revision;
        return () => active && settled && revision === version && trees.get(id) === tree && frame.isConnected
            && mounts.get(frame) === controller && frame.contentDocument === retained && frame.srcdoc === srcdoc
            && frame.getAttribute('sandbox') === 'allow-same-origin' && frame.getAttribute('referrerpolicy') === 'no-referrer'
            && frame.getAttribute('allow') === '';
    }, setLocation(path: string) {
        if (!active) return;
        const next = location(path);
        if (next.pathname !== selected.pathname) throw new Error('cem.preview.artifact_location');
        if (next.href === selected.href) return;
        revision++; selected.search = next.search; selected.hash = next.hash;
        if (selected.hash) scroll(selected.hash.slice(1));
        publish();
    }, dispose() {
        if (!active) return;
        active = false; cancelLoad?.(); cancelLoad = undefined;
        document?.removeEventListener('click', navigate, true); document?.removeEventListener('auxclick', navigate, true); document?.removeEventListener('submit', stopForm, true);
        document?.body.replaceChildren();
        for (const url of blobs) URL.revokeObjectURL(url);
        blobs.clear(); images.clear(); styles.clear(); selectedArtifact = undefined; tree.revoked.delete(revoke); inline?.dispose();
        if (mounts.get(frame) === controller) { mounts.delete(frame); frame.removeAttribute('srcdoc'); }
    } });
    const revoke = () => {
        controller.dispose();
        try { options.onStatus?.('Artifact run released'); }
        catch { /* A consumer notification must not interrupt release of sibling mounts. */ }
    };
    tree.revoked.add(revoke); mounts.set(frame, controller);
    void ready.catch(() => undefined);
    return controller;
}
