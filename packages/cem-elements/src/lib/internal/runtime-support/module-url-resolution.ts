import { resolveCemModuleUrl } from './cem-ql-render.js';

export interface CemModuleUrlResolutionContext {
    readonly identity: string;
    readonly baseUrl: string;
    readonly resolverIdentity: string;
    readonly resourcePolicyStamp: string;
}

export type CemModuleUrlResolutionReferrer =
    | { readonly kind: 'url'; readonly value: string }
    | { readonly kind: 'context'; readonly context: CemModuleUrlResolutionContext };

export interface CemModuleUrlResolutionRequest {
    readonly purpose: 'template-slice';
    readonly authoredSpecifier: string;
    readonly currentContext: CemModuleUrlResolutionContext;
    readonly referrer?: CemModuleUrlResolutionReferrer;
}

export interface CemModuleUrlResolution {
    readonly authoredSpecifier: string;
    readonly normalizedSpecifier: string;
    readonly resolvedUrl: string;
    readonly contextIdentity: string;
    readonly resolverIdentity: string;
    readonly resourcePolicyStamp: string;
    readonly referrerKind?: 'url' | 'context';
    readonly authoredReferrer?: string;
    readonly resolvedReferrerUrl?: string;
    readonly currentContextIdentity: string;
    readonly selectedContextIdentity: string;
    readonly matchedFrameId?: string;
    readonly matchedScopePrefix?: string;
    readonly matchedCollection?: 'imports' | 'resources';
    readonly matchedKey?: string;
    readonly contentTypeHint?: string;
    readonly integrity?: string;
}

export interface CemModuleUrlResolutionFailure {
    readonly authoredSpecifier: string;
    readonly normalizedSpecifier?: string;
    readonly contextIdentity: string;
    readonly resolverIdentity: string;
    readonly resourcePolicyStamp: string;
    readonly referrerKind?: 'url' | 'context';
    readonly authoredReferrer?: string;
    readonly resolvedReferrerUrl?: string;
    readonly currentContextIdentity: string;
    readonly selectedContextIdentity?: string;
    readonly reason:
        | 'invalid'
        | 'unresolved'
        | 'blocked'
        | 'policy-denied'
        | 'unavailable'
        | 'referrer-invalid'
        | 'referrer-unresolved'
        | 'referrer-unavailable'
        | 'referrer-scope-denied';
    readonly message: string;
    readonly matchedFrameId?: string;
    readonly matchedKey?: string;
}

export class CemModuleUrlResolutionError extends Error {
    constructor(readonly failure: CemModuleUrlResolutionFailure) {
        super(failure.message);
        this.name = 'CemModuleUrlResolutionError';
    }
}

export interface CemBrowserImportMap {
    imports?: Readonly<Record<string, string | null>>;
    scopes?: Readonly<Record<string, Readonly<Record<string, string | null>>>>;
}

export interface CemBrowserModuleUrlRootOptions {
    /** Frozen root base. Defaults to `document.baseURI`. */
    baseUrl?: string;
    /** Explicit input replaces automatic page import-map capture. */
    importMap?: CemBrowserImportMap;
    resolverIdentity?: string;
}

export interface CemBrowserModuleUrlMapping {
    target: string | null;
    contentTypeHint?: string | null;
    integrity?: string | null;
}

export interface CemBrowserModuleUrlSpecifierMap {
    imports: Record<string, CemBrowserModuleUrlMapping>;
    resources: Record<string, CemBrowserModuleUrlMapping>;
}

export interface CemBrowserModuleUrlScopedMap {
    prefix: string;
    specifiers: CemBrowserModuleUrlSpecifierMap;
}

/** Static CEM-ML `module-map` metadata for one template resolution context. */
export interface CemBrowserModuleUrlMap {
    scopes: CemBrowserModuleUrlScopedMap[];
    specifiers: CemBrowserModuleUrlSpecifierMap;
}

interface CemModuleUrlFrameWire {
    frameId: string;
    baseUrl: string;
    moduleMapBaseUrl?: string;
    scopes: CemBrowserModuleUrlScopedMap[];
    specifiers: CemBrowserModuleUrlSpecifierMap;
    allowedSchemes?: string[];
}

interface CemModuleUrlContextWire {
    identity: string;
    resolverIdentity: string;
    resourcePolicyStamp: string;
    frames: CemModuleUrlFrameWire[];
}

export interface CemBrowserModuleUrlContext extends CemModuleUrlResolutionContext {
    readonly handle: string;
    readonly parent: CemBrowserModuleUrlContext | null;
    readonly wire: CemModuleUrlContextWire;
}

export interface CemBrowserModuleUrlRoot {
    context: CemBrowserModuleUrlContext;
    diagnostics: string[];
}

export function createBrowserModuleUrlRoot(
    document: Document,
    resourcePolicyStamp: string,
    options: CemBrowserModuleUrlRootOptions = {},
): CemBrowserModuleUrlRoot {
    const baseUrl = absoluteBaseUrl(options.baseUrl ?? document.baseURI);
    const diagnostics: string[] = [];
    const importMaps = options.importMap === undefined
        ? readDocumentImportMaps(document, diagnostics)
        : [options.importMap];
    const specifiers = emptySpecifierMap();
    const scopes = new Map<string, CemBrowserModuleUrlSpecifierMap>();
    for (const importMap of importMaps) {
        mergeSpecifierEntries(specifiers.imports, importMap.imports, baseUrl, diagnostics, 'imports');
        for (const [authoredScope, entries] of Object.entries(importMap.scopes ?? {})) {
            let scope: string;
            try {
                scope = new URL(authoredScope, baseUrl).href;
            } catch {
                diagnostics.push(`import-map scope \`${authoredScope}\` is not a valid URL prefix`);
                continue;
            }
            const scoped = scopes.get(scope) ?? emptySpecifierMap();
            mergeSpecifierEntries(scoped.imports, entries, baseUrl, diagnostics, `scopes[${authoredScope}]`);
            scopes.set(scope, scoped);
        }
    }
    const frame: CemModuleUrlFrameWire = {
        frameId: 'browser-document-root',
        baseUrl,
        scopes: Array.from(scopes, ([prefix, scoped]) => ({ prefix, specifiers: scoped })),
        specifiers,
    };
    const mapIdentity = stableHash(JSON.stringify({ baseUrl, scopes: frame.scopes, specifiers }));
    const resolverIdentity = options.resolverIdentity?.trim() || `browser-import-map:${mapIdentity}`;
    const identity = `browser-root:${stableHash(`${baseUrl}\n${resolverIdentity}\n${resourcePolicyStamp}`)}`;
    const context = Object.freeze({
        handle: identity,
        parent: null,
        identity,
        baseUrl,
        resolverIdentity,
        resourcePolicyStamp,
        wire: {
            identity,
            resolverIdentity,
            resourcePolicyStamp,
            frames: [frame],
        },
    } satisfies CemBrowserModuleUrlContext);
    return { context, diagnostics };
}

export function createBrowserModuleUrlContext(
    parent: CemBrowserModuleUrlContext,
    handleSeed: string,
    baseUrl: string,
    resolverIdentity: string,
    resourcePolicyStamp: string,
    moduleMap?: CemBrowserModuleUrlMap | null,
): CemBrowserModuleUrlContext {
    const normalizedBaseUrl = absoluteBaseUrl(baseUrl);
    const normalizedModuleMap = cloneModuleMap(moduleMap);
    const mapIdentity = stableHash(JSON.stringify(normalizedModuleMap));
    const handle = `browser-context:${stableHash(`${parent.handle}\n${handleSeed}\n${normalizedBaseUrl}\n${mapIdentity}`)}`;
    const identity = `${handle}:${stableHash(`${resolverIdentity}\n${resourcePolicyStamp}\n${mapIdentity}`)}`;
    const wireResolverIdentity = `${parent.resolverIdentity}+${resolverIdentity}:${mapIdentity}`;
    return Object.freeze({
        handle,
        parent,
        identity,
        baseUrl: normalizedBaseUrl,
        resolverIdentity: wireResolverIdentity,
        resourcePolicyStamp,
        wire: {
            identity,
            resolverIdentity: wireResolverIdentity,
            resourcePolicyStamp,
            frames: [
                ...parent.wire.frames,
                {
                    frameId: handle,
                    baseUrl: normalizedBaseUrl,
                    scopes: normalizedModuleMap.scopes,
                    specifiers: normalizedModuleMap.specifiers,
                },
            ],
        },
    } satisfies CemBrowserModuleUrlContext);
}

export async function resolveBrowserModuleUrl(
    currentContext: CemBrowserModuleUrlContext,
    authoredSpecifier: string,
    referrer?: { kind: 'url'; value: string } | { kind: 'context'; context: CemBrowserModuleUrlContext },
): Promise<CemModuleUrlResolution> {
    const contexts = new Map<string, CemBrowserModuleUrlContext>();
    collectContextLineage(contexts, currentContext);
    if (referrer?.kind === 'context') {
        collectContextLineage(contexts, referrer.context);
    }
    const result = await resolveCemModuleUrl({
        purpose: 'template-slice',
        authoredSpecifier,
        currentContext: currentContext.handle,
        ...(referrer === undefined
            ? {}
            : referrer.kind === 'url'
              ? { referrer: { kind: 'url', value: referrer.value } }
              : { referrer: { kind: 'context', context: referrer.context.handle } }),
        contexts: Array.from(contexts.values(), (context) => ({
            handle: context.handle,
            ...(context.parent === null ? {} : { parent: context.parent.handle }),
            context: context.wire,
        })),
    }) as {
        status?: 'resolved' | 'error';
        resolution?: CemModuleUrlResolution;
        error?: CemModuleUrlResolutionFailure | { code?: string; message?: string };
    };
    if (result.status === 'resolved' && result.resolution) {
        return result.resolution;
    }
    if (result.status === 'error' && result.error && 'reason' in result.error) {
        throw new CemModuleUrlResolutionError(result.error);
    }
    throw new Error(result.error?.message ?? 'module URL resolution returned an invalid host response');
}

function collectContextLineage(
    contexts: Map<string, CemBrowserModuleUrlContext>,
    context: CemBrowserModuleUrlContext,
): void {
    if (context.parent) {
        collectContextLineage(contexts, context.parent);
    }
    contexts.set(context.handle, context);
}

function readDocumentImportMaps(document: Document, diagnostics: string[]): CemBrowserImportMap[] {
    const maps: CemBrowserImportMap[] = [];
    for (const script of Array.from(document.querySelectorAll('script[type="importmap"]'))) {
        try {
            const parsed = JSON.parse(script.textContent ?? '') as unknown;
            if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
                diagnostics.push('page import map must contain a JSON object');
                continue;
            }
            const record = parsed as Record<string, unknown>;
            maps.push({
                imports: readImportEntries(record.imports, diagnostics, 'imports'),
                scopes: readImportScopes(record.scopes, diagnostics),
            });
        } catch (error) {
            diagnostics.push(`page import map is invalid JSON: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    return maps;
}

function readImportEntries(
    value: unknown,
    diagnostics: string[],
    label: string,
): Record<string, string | null> | undefined {
    if (value === undefined) {
        return undefined;
    }
    if (!value || typeof value !== 'object' || Array.isArray(value)) {
        diagnostics.push(`import-map ${label} must be an object`);
        return undefined;
    }
    const entries: Record<string, string | null> = {};
    for (const [specifier, target] of Object.entries(value)) {
        if (typeof target === 'string' || target === null) {
            entries[specifier] = target;
        } else {
            diagnostics.push(`import-map ${label} entry \`${specifier}\` must be a string or null`);
        }
    }
    return entries;
}

function readImportScopes(
    value: unknown,
    diagnostics: string[],
): Record<string, Record<string, string | null>> | undefined {
    if (value === undefined) {
        return undefined;
    }
    if (!value || typeof value !== 'object' || Array.isArray(value)) {
        diagnostics.push('import-map scopes must be an object');
        return undefined;
    }
    const scopes: Record<string, Record<string, string | null>> = {};
    for (const [scope, entries] of Object.entries(value)) {
        const parsed = readImportEntries(entries, diagnostics, `scopes[${scope}]`);
        if (parsed) {
            scopes[scope] = parsed;
        }
    }
    return scopes;
}

function mergeSpecifierEntries(
    destination: Record<string, CemBrowserModuleUrlMapping>,
    entries: Readonly<Record<string, string | null>> | undefined,
    baseUrl: string,
    diagnostics: string[],
    label: string,
): void {
    for (const [authoredKey, target] of Object.entries(entries ?? {})) {
        const key = normalizeImportMapKey(authoredKey, baseUrl);
        if (!key) {
            diagnostics.push(`import-map ${label} has an empty specifier key`);
            continue;
        }
        if (Object.hasOwn(destination, key)) {
            continue;
        }
        destination[key] = { target };
    }
}

function normalizeImportMapKey(value: string, baseUrl: string): string {
    const trimmed = value.trim();
    if (!isUrlLikeSpecifier(trimmed)) {
        return trimmed;
    }
    try {
        return new URL(trimmed, baseUrl).href;
    } catch {
        return trimmed;
    }
}

function isUrlLikeSpecifier(value: string): boolean {
    return /^[A-Za-z][A-Za-z0-9+.-]*:/.test(value)
        || value.startsWith('/')
        || value.startsWith('./')
        || value.startsWith('../')
        || value.startsWith('?')
        || value.startsWith('#');
}

function emptySpecifierMap(): CemBrowserModuleUrlSpecifierMap {
    return { imports: {}, resources: {} };
}

function cloneModuleMap(moduleMap?: CemBrowserModuleUrlMap | null): CemBrowserModuleUrlMap {
    const cloneSpecifiers = (
        specifiers?: Partial<CemBrowserModuleUrlSpecifierMap>,
    ): CemBrowserModuleUrlSpecifierMap => ({
        imports: Object.fromEntries(Object.entries(specifiers?.imports ?? {}).map(([key, mapping]) => [
            key,
            { ...mapping },
        ])),
        resources: Object.fromEntries(Object.entries(specifiers?.resources ?? {}).map(([key, mapping]) => [
            key,
            { ...mapping },
        ])),
    });
    return {
        scopes: (moduleMap?.scopes ?? []).map((scope) => ({
            prefix: scope.prefix,
            specifiers: cloneSpecifiers(scope.specifiers),
        })),
        specifiers: cloneSpecifiers(moduleMap?.specifiers),
    };
}

function absoluteBaseUrl(value: string): string {
    try {
        return new URL(value).href;
    } catch (error) {
        throw new TypeError(
            `module URL base \`${value}\` is not absolute: ${error instanceof Error ? error.message : String(error)}`,
            { cause: error },
        );
    }
}

function stableHash(value: string): string {
    let hash = 2_166_136_261;
    for (let index = 0; index < value.length; index += 1) {
        hash ^= value.charCodeAt(index);
        hash = Math.imul(hash, 16_777_619);
    }
    return (hash >>> 0).toString(36);
}
