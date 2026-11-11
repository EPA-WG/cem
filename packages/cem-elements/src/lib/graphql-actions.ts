import type { CemCheckpointPublicationOwner } from './checkpoint-publication.js';
import type { CemHttpResourceLoadResult } from './cem-elements.js';
import { bindCemHostCommands, type CemHostCommandRequest } from './host-view.js';
import { openCemHttpResource, readByteStream } from './http-resource-transport.js';

export interface CemGraphqlActionRoute {
    readonly event: 'change' | 'click';
    readonly variables: Readonly<Record<string, 'binding' | 'key' | 'positive-number'>>;
}

export interface CemGraphqlResponse {
    readonly operation: string;
    readonly bytes: Uint8Array;
    readonly uri: string;
    readonly html: string;
}

export interface CemGraphqlGuard {
    applicationDocument(): string;
    validateVariables(operation: string, bytes: Uint8Array, uri: string): void;
    renderResponse(operation: string, bytes: Uint8Array, uri: string): string;
}

export interface CemGraphqlQueryOptions {
    readonly baseUrl: string;
    readonly endpoint: string;
    readonly operation: string;
    readonly variables: Readonly<Record<string, unknown>>;
    readonly guard: Pick<CemGraphqlGuard, 'applicationDocument' | 'renderResponse'> & {
        validateQueryVariables(operation: string, bytes: Uint8Array, uri: string): void;
    };
    readonly signal?: AbortSignal;
}

export interface CemGraphqlActionOptions {
    readonly root: HTMLElement;
    /** Trusted service origin for a parent-owned script-free frame. */
    readonly baseUrl?: string;
    /** Same-origin endpoint supplied by the host's deployed application profile. */
    readonly endpoint: string;
    readonly actions: Readonly<Record<string, CemGraphqlActionRoute>>;
    readonly bindings: Readonly<Record<string, string>>;
    readonly guard: CemGraphqlGuard;
    readonly owner: Pick<CemCheckpointPublicationOwner, 'beginAction'>;
    /** Original native source owner's replay fence, claimed before transport. */
    readonly admitMutation?: (operationId: string) => void;
    /** Synchronously accept and publish the guarded response for this owner. */
    readonly publish: (response: CemGraphqlResponse) => boolean;
    readonly reportError: (error: unknown) => void;
}

const policy = Object.freeze({
    allowCrossOrigin: false, maxRequestBytes: 1_048_576, maxResponseBytes: 1_048_576,
    timeoutMs: 15_000, redirect: 'error' as const,
});
const responseTypes = ['application/graphql-response+json', 'application/json'];
const headers = Object.freeze({ accept: responseTypes.join(', ') });
const encoder = new TextEncoder();

function graphqlTarget(endpoint: string, baseUrl: string): URL {
    const target = new URL(endpoint, baseUrl);
    if (!['http:', 'https:'].includes(target.protocol) || target.origin !== new URL(baseUrl).origin
        || target.username || target.password || target.hash) {
        throw new TypeError('GraphQL requires a same-origin HTTP endpoint');
    }
    return target;
}

function requestSignal(lifetime?: AbortSignal): AbortSignal {
    const timeout = AbortSignal.timeout(policy.timeoutMs);
    return lifetime ? AbortSignal.any([lifetime, timeout]) : timeout;
}

async function responseBytes(loaded: CemHttpResourceLoadResult, signal: AbortSignal, current: () => boolean): Promise<Uint8Array> {
    const type = loaded.response.contentType?.split(';', 1)[0]?.trim().toLowerCase();
    if (!type || !responseTypes.includes(type) || signal.aborted || !current()) {
        await loaded.body[Symbol.asyncIterator]().return?.();
        throw new Error('GraphQL response was refused');
    }
    const bytes = await readByteStream(loaded.body, policy.maxResponseBytes, signal);
    signal.throwIfAborted();
    return bytes;
}

/** Read one explicit query from the selected document. Only the native guard
 * can admit the operation as a query before it is sent using HTTP GET. */
export async function readCemGraphqlQuery(options: CemGraphqlQueryOptions): Promise<CemGraphqlResponse> {
    const { baseUrl, endpoint, operation, guard } = options;
    const signal = requestSignal(options.signal);
    signal.throwIfAborted();
    const target = graphqlTarget(endpoint, baseUrl);
    const uri = target.href;
    const variables = JSON.stringify(options.variables);
    guard.validateQueryVariables(operation, encoder.encode(variables), `${uri}#variables`);
    target.searchParams.set('query', guard.applicationDocument());
    target.searchParams.set('operationName', operation);
    target.searchParams.set('variables', variables);
    if (target.href.length > 16 * 1024) throw new Error('GraphQL query exceeds the URL byte limit');
    const loaded = await openCemHttpResource({
        authoredUrl: endpoint, baseUrl, resolvedUrl: target.href,
        resolverIdentity: 'selected-graphql', resourcePolicyStamp: 'same-origin',
        contextIdentity: operation, method: 'GET', policy, signal,
        credentials: 'same-origin', cache: 'no-store', headers,
    });
    const bytes = await responseBytes(loaded, signal, () => true);
    return { operation, bytes, uri, html: guard.renderResponse(operation, bytes, uri) };
}

/** Bind trusted user actions to the selected application. Restoring a view
 * binds listeners only. There is no API for dispatching a saved operation. */
export function bindCemGraphqlActions(options: CemGraphqlActionOptions): () => void {
    const { root, endpoint, actions, guard, owner, publish, reportError } = options;
    const baseUrl = options.baseUrl ?? root.ownerDocument.URL;
    const resolvedUrl = graphqlTarget(endpoint, baseUrl).href;
    const document = JSON.stringify(guard.applicationDocument());
    const bindings = new Map(Object.entries(options.bindings));
    const lifetime = new AbortController();
    const current = () => !lifetime.signal.aborted && root.isConnected;
    let pending = false;
    const commands: Record<string, (request: CemHostCommandRequest) => Promise<void>> = Object.create(null);
    for (const [operation, route] of Object.entries(actions)) {
        const { event } = route;
        if (event !== 'change' && event !== 'click') throw new TypeError('GraphQL actions require a click or change binding');
        const sources = Object.entries(route.variables);
        commands[operation] = async request => {
            if (!request.isTrusted || request.eventType !== event || pending) return;
            const variables: Record<string, string | number> = Object.create(null);
            for (const [name, source] of sources) {
                if (source === 'positive-number') {
                    const value = Number(request.value);
                    if (!Number.isFinite(value) || value <= 0) throw new TypeError('Action requires a positive control value');
                    variables[name] = value;
                } else {
                    const value = source === 'key' ? request.commandKey : source === 'binding' ? bindings.get(name) : undefined;
                    if (value === undefined || value === '') throw new TypeError(`Missing action variable ${name}`);
                    variables[name] = value;
                }
            }
            const serialized = JSON.stringify(variables);
            guard.validateVariables(operation, encoder.encode(serialized), `${resolvedUrl}#variables`);
            const body = encoder.encode(`{"query":${document},"operationName":${JSON.stringify(operation)},"variables":${serialized}}`);
            if (body.byteLength > policy.maxRequestBytes) throw new Error('GraphQL action exceeds the request byte limit');
            if (!current()) return;

            const action = owner.beginAction();
            pending = true;
            try {
                options.admitMutation?.(action.operationId);
                const signal = requestSignal(lifetime.signal);
                const loaded = await openCemHttpResource({
                    authoredUrl: endpoint, baseUrl, resolvedUrl,
                    resolverIdentity: 'selected-graphql', resourcePolicyStamp: 'same-origin',
                    contextIdentity: action.operationId, method: 'POST', body, policy, signal,
                    credentials: 'same-origin',
                    headers: { ...headers, 'content-type': 'application/json' },
                });
                const bytes = await responseBytes(loaded, signal, () => current() && action.current());
                const uri = loaded.response.url || resolvedUrl;
                if (!current() || !action.current()) return;
                const html = guard.renderResponse(operation, bytes, uri);
                if (!current() || !action.current()) return;
                if (!publish({ operation, bytes, uri, html })) throw new Error('GraphQL response publication was refused');
                if (current()) action.acknowledge();
            } finally {
                pending = false;
            }
        };
    }
    const unbind = bindCemHostCommands(root, { commands, current, report: reportError });
    return () => {
        unbind();
        lifetime.abort();
    };
}
