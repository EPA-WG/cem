// Explicit executable dependency transport; never part of render data.
// eslint-disable-next-line @nx/enforce-module-boundaries -- generated shared WASM control plane.
import { retainCemtXPathFunctions, disposeCemtXPathFunctions } from '../../../../../cem_ql/dist/wasm/cem_ql.js';
import { cemMlTemplateArtifactPayloadKey, ensureRuntimeReady } from './cem-ql-render.js';

export interface CemXPathFunctionLibrarySource {
    kind: 'cemt-xpath-function-library';
    uri: string;
    source: string;
    sourceHash: string;
    resolverPolicyStamp: string;
    cemMlVersion: string;
    cemQlVersion: string;
}

export interface CemXPathFunctionLibraryLease {
    companionId: number;
    release(): void;
}

export const XPATH_LIBRARY_MAX_SOURCE_BYTES = 32 * 1024;

export async function identifyXPathFunctionLibrary(
    source: string, uri: string, resolverPolicyStamp: string,
): Promise<CemXPathFunctionLibrarySource> {
    if (typeof source !== 'string' || typeof uri !== 'string' || typeof resolverPolicyStamp !== 'string'
        || !resolverPolicyStamp || new URL(uri).href !== uri) {
        throw new Error('XPath library requires an absolute URL and resolver policy identity');
    }
    for (const value of [source, uri]) {
        if (new TextEncoder().encode(value).byteLength > XPATH_LIBRARY_MAX_SOURCE_BYTES) {
            throw new Error('XPath function library exceeds source limit');
        }
    }
    const identity = await cemMlTemplateArtifactPayloadKey(source, 'dev');
    return { kind: 'cemt-xpath-function-library', uri, source, resolverPolicyStamp,
        sourceHash: identity.sourceHash, cemMlVersion: identity.cemMlVersion, cemQlVersion: identity.cemQlVersion };
}

interface LibraryEntry {
    pending: Promise<number>;
    references: number;
    companionId?: number;
}

/** Independent, reference-counted library cache owned by one processing host. */
export class CemXPathFunctionLibraries {
    private readonly entries = new Map<string, LibraryEntry>();
    private disposed = false;

    async acquire(source: CemXPathFunctionLibrarySource): Promise<CemXPathFunctionLibraryLease> {
        if (this.disposed) throw new Error('XPath library cache is disposed');
        // Include bytes as well as the claimed hash: a forged reuse request must
        // pass validation rather than borrowing another request's capability.
        const key = JSON.stringify([source.kind, source.uri, source.sourceHash,
            source.resolverPolicyStamp, source.cemMlVersion, source.cemQlVersion, source.source]);
        let entry = this.entries.get(key);
        if (!entry) {
            if (this.entries.size >= 64) throw new Error('XPath function library count exceeds limit');
            const created: LibraryEntry = { references: 0, pending: Promise.resolve(0) };
            created.pending = this.retain(source).then((id) => {
                if (this.disposed) {
                    disposeCemtXPathFunctions(id);
                    throw new Error('XPath library cache is disposed');
                }
                created.companionId = id;
                return id;
            }).catch((error: unknown) => {
                this.entries.delete(key);
                throw error;
            });
            this.entries.set(key, created);
            entry = created;
        }
        entry.references++;
        const companionId = await entry.pending;
        let released = false;
        return { companionId, release: () => {
            if (released) return;
            released = true;
            if (--entry.references === 0 && this.entries.get(key) === entry) {
                this.entries.delete(key);
                disposeCemtXPathFunctions(companionId);
            }
        } };
    }

    dispose(): void {
        this.disposed = true;
        for (const entry of this.entries.values()) {
            if (entry.companionId !== undefined) disposeCemtXPathFunctions(entry.companionId);
        }
        this.entries.clear();
    }

    private async retain(source: CemXPathFunctionLibrarySource): Promise<number> {
        const actual = await identifyXPathFunctionLibrary(source.source, source.uri, source.resolverPolicyStamp);
        if (source.kind !== actual.kind || source.sourceHash !== actual.sourceHash
            || source.cemMlVersion !== actual.cemMlVersion || source.cemQlVersion !== actual.cemQlVersion) {
            throw new Error('XPath function library identity mismatch');
        }
        await ensureRuntimeReady();
        let metadata: string;
        try {
            metadata = retainCemtXPathFunctions(source.source, source.uri);
        } catch (error) {
            throw new Error(`${source.uri}: ${error instanceof Error ? error.message : String(error)}`, { cause: error });
        }
        const result = JSON.parse(metadata) as {
            companionId: number; contentType: string; formatVersion: string; sourceHash: string;
        };
        if (!Number.isSafeInteger(result.companionId) || result.companionId < 1) {
            throw new Error('XPath function library did not retain a companion');
        }
        if (result.contentType !== 'application/vnd.cem.cemt-xpath-functions+cem-bin'
            || result.formatVersion !== 'cemt-xpath-functions/1' || result.sourceHash !== source.sourceHash) {
            disposeCemtXPathFunctions(result.companionId);
            throw new Error('XPath function companion identity mismatch');
        }
        return result.companionId;
    }
}
