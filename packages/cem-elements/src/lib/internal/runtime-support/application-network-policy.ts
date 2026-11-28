/** A compiled rule matches a complete request. Match every layer against the same tuple. */
interface Operation {
    readonly transport: 'http' | 'websocket';
    readonly url: URL;
    readonly method: string;
    readonly credentials: RequestCredentials;
}
type Layer = readonly ((operation: Operation, following: boolean) => boolean)[];

function invalid(): never { throw new TypeError('Invalid application network policy'); }
export function denyNetwork(): never {
    throw new DOMException('Application network operation denied', 'NotAllowedError');
}

function attributes(element: Element, allowed: readonly string[]) {
    if (Array.from(element.attributes).some(attr => !allowed.includes(attr.name))) invalid();
}

/** Asterisks operate within a single DNS label or path segment. */
function glob(mask: string, text: string): boolean {
    const parts = mask.split('*');
    if (parts.length === 1) return mask === text;
    if (mask === '*') return text.length > 0;
    const first = parts[0], last = parts[parts.length - 1];
    if (!text.startsWith(first) || !text.endsWith(last)) return false;
    let position = first.length;
    for (const part of parts.slice(1, -1)) {
        const next = text.indexOf(part, position);
        if (next === -1) return false;
        position = next + part.length;
    }
    return position <= text.length - last.length;
}

function hostMatcher(mask: string) {
    // URL supplies IDNA normalization for literals. Reject URL syntax in a host field.
    if (!mask || /[\s\\/@%?#]/u.test(mask)) invalid();
    mask = mask.toLowerCase().replace(/\.$/, '');
    if (!mask.includes('*')) {
        if (mask.includes(':') && !(mask.startsWith('[') && mask.endsWith(']'))) invalid();
        const host = new URL('http://' + mask).hostname;
        return (value: string) => value === host;
    }
    const parts = mask.split('.');
    const recursive = parts[0] === '**';
    if (recursive) parts.shift();
    if (!parts.length || parts.at(-1)!.includes('*') || parts.some(part => !part || part.includes('**'))) invalid();
    const labels = parts.map(part => {
        if (part.includes('*')) {
            if (!/^[a-z0-9*-]+$/.test(part)) invalid();
            return part;
        }
        // Prefix a fixed label so a numeric label is not interpreted as an IPv4 address.
        const normalized = new URL('http://a.' + part + '.invalid').hostname.split('.');
        if (normalized.length !== 3 || !/^[a-z0-9-]+$/.test(normalized[1])) invalid();
        return normalized[1];
    });
    if (recursive && labels.some(label => label.includes('*'))) invalid();
    return (value: string) => {
        if (value.startsWith('[') || /^\d+\.\d+\.\d+\.\d+$/.test(value)) return false;
        const actual = value.split('.');
        const start = actual.length - labels.length;
        return (recursive ? start > 0 : start === 0) && labels.every((label, i) => glob(label, actual[start + i]));
    };
}

function ambiguousPath(path: string) { return /%(?:2f|5c|25|00)/i.test(path); }

function pathMatcher(mask: string) {
    if (!mask.startsWith('/') || /[\\?#\s]/.test(mask) || ambiguousPath(mask)
        || new URL('https://example.invalid' + mask).pathname !== mask) invalid();
    const segments = mask.slice(1).split('/');
    const recursive = segments.at(-1) === '**';
    if (recursive) segments.pop();
    if (segments.some(s => s.includes('**'))) invalid();
    return (path: string) => {
        const actual = path.slice(1).split('/');
        return (recursive ? actual.length >= segments.length : actual.length === segments.length)
            && segments.every((segment, i) => glob(segment, actual[i]));
    };
}

function rules(element: Element): Layer {
    return Array.from(element.children, node => {
        if (node.localName !== 'allow' || node.namespaceURI !== element.namespaceURI || node.childElementCount) invalid();
        attributes(node, ['transport', 'scheme', 'host', 'port', 'path', 'methods', 'credentials', 'redirect']);
        const get = (key: string) => node.getAttribute(key);
        const transport = get('transport');
        const scheme = get('scheme');
        const host = hostMatcher(get('host') ?? '');
        const path = pathMatcher(get('path') ?? '');
        const port = get('port') ?? (scheme === 'https' || scheme === 'wss' ? '443' : '80');
        const credentials = get('credentials') ?? 'omit';
        const redirect = get('redirect') ?? 'error';
        if (!['http', 'websocket', 'event-stream'].includes(transport ?? '')
            || !(transport === 'websocket' ? ['ws', 'wss'] : ['http', 'https']).includes(scheme ?? '')
            || !/^[1-9]\d{0,4}$/.test(port) || +port > 65535
            || !['omit', 'same-origin', 'include'].includes(credentials) || !['error', 'checked'].includes(redirect)) invalid();
        const methods = get('methods')?.trim().split(/\s+/);
        if (transport === 'http' ? !methods?.every(method => ['GET', 'HEAD', 'POST'].includes(method)) : methods !== undefined) invalid();
        const admitsCredentials = (requested: RequestCredentials) =>
            requested === 'omit' || credentials === 'include' || requested === credentials;
        return ({ url, method, credentials: requested, transport: requestedTransport }, following) => transport === requestedTransport && url.protocol === scheme + ':'
            && port === (url.port || (['https:', 'wss:'].includes(url.protocol) ? '443' : '80'))
            && admitsCredentials(requested) && (transport === 'websocket' || methods!.includes(method)) && host(url.hostname.replace(/\.$/, '')) && path(url.pathname)
            && (!following || redirect === 'checked');
    });
}

export function isNetworkDeclaration(node: Element) {
    return node.getAttribute('api') === 'network' && ['filter', 'export'].includes(node.localName);
}

/** Capture local policy before first admission. Only a trusted host call activates an export. */
export class ApplicationNetworkPolicy {
    private readonly declarations: Element[];
    binding?: ApplicationNetworkGrant;

    constructor(policy?: Element) {
        this.declarations = Array.from(policy?.children ?? []).filter(isNetworkDeclaration).map(node => node.cloneNode(true) as Element);
    }

    exportFor(id: string, scope: string | null) {
        const entries = this.declarations.filter(node => node.localName === 'export'
            && node.getAttribute('to') === id && node.getAttribute('scope') === scope);
        if (!id || entries.length !== 1) denyNetwork();
        return entries[0];
    }

    install(element: Element, parent?: ApplicationNetworkGrant) {
        if (element.localName !== 'export' || element.getAttribute('api') !== 'network') invalid();
        attributes(element, ['api', 'to', 'scope', 'delegate']);
        if (parent && (!parent.delegate || parent.signal.aborted)) denyNetwork();
        const delegate = element.getAttribute('delegate') ?? 'false';
        if (delegate !== 'false' && delegate !== 'true') invalid();
        const layers = [...(parent?.layers ?? []), rules(element)];
        const filters = this.declarations.filter(node => node.localName === 'filter');
        if (filters.length > 1) invalid();
        if (filters[0]) {
            attributes(filters[0], ['api']);
            layers.push(rules(filters[0]));
        }
        const lifetime = new AbortController();
        const signal = parent ? AbortSignal.any([parent.signal, lifetime.signal]) : lifetime.signal;
        const binding = { layers, delegate: delegate === 'true', signal, revoke: () =>
            lifetime.abort(new DOMException('grant-revoked', 'AbortError')) };
        const previous = this.binding;
        this.binding = binding;
        previous?.revoke();
        return { revoke: binding.revoke };
    }
}

export interface ApplicationNetworkGrant {
    readonly layers: readonly Layer[];
    readonly signal: AbortSignal;
    readonly delegate: boolean;
    revoke(): void;
}

export function checkNetwork(grant: ApplicationNetworkGrant, operation: Operation, following = false) {
    grant.signal.throwIfAborted();
    const { url } = operation;
    if (url.username || url.password || ambiguousPath(url.pathname)
        || !grant.layers.every(layer => layer.some(rule => rule(operation, following)))) denyNetwork();
}
