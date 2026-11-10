import type { CemDeclarationScope } from '../../declaration-scope.js';
import type { CemOwnedStylesheet, DeclarationStyleOwnership, DeclarationStylesheetCommit } from '../../declaration-style-ownership.js';
import type { QuerySourceResource } from './query-source-resource.js';

function stylesheetOutputs(receipt: string): CemOwnedStylesheet[] {
    type Occurrence = { index: number; scope: string | null; stylesheet: CemOwnedStylesheet['output'] };
    const entries: Occurrence[] = JSON.parse(receipt);
    if (!Array.isArray(entries) || entries.length > 64) throw new Error('cem.application.stylesheet.receipt');
    return entries.map((entry, index) => {
        const sheet = entry?.stylesheet;
        const identity = sheet?.identity;
        if (entry?.index !== index || (entry.scope !== null && (typeof entry.scope !== 'string' || !entry.scope))
            || sheet?.status !== 'ready' || typeof sheet.loadId !== 'string' || typeof sheet.css !== 'string'
            || typeof identity?.ownerKey !== 'string' || !identity.ownerKey
            || typeof identity.cacheKey !== 'string' || !identity.cacheKey
            || (identity.contextMarker !== null && (typeof identity.contextMarker !== 'string' || !identity.contextMarker))
            || !Array.isArray(sheet.diagnostics)) {
            throw new Error('cem.application.stylesheet.receipt');
        }
        const scope = entry.scope === null ? { kind: 'private' as const } : { kind: 'shared' as const, name: entry.scope };
        return { index, scope, output: sheet };
    });
}

/** Stage one host against its original native application session. The caller
 * publishes this entry through declaration ownership, with a patch if needed. */
export async function prepareApplicationStylesheets(
    resource: QuerySourceResource, ownership: DeclarationStyleOwnership, element: HTMLElement, scope: CemDeclarationScope,
): Promise<DeclarationStylesheetCommit> {
    const lease = ownership.stageConsumer(element, scope);
    let source: AbortSignal | undefined;
    let released = false;
    const release = () => {
        if (released) return;
        released = true;
        source?.removeEventListener('abort', release);
        lease.signal.removeEventListener('abort', release);
        lease.release();
    };
    lease.signal.addEventListener('abort', release, { once: true });
    try {
        // Superseding this DOM candidate does not cancel the shared native endpoint.
        const result = await resource.stylesheets(element.localName);
        source = result.signal;
        const isCurrent = () => !released && !lease.signal.aborted && result.isCurrent();
        if (!isCurrent()) throw new Error('cem.application.stylesheet.stale');
        source.addEventListener('abort', release, { once: true });
        return { lease, outputs: stylesheetOutputs(result.receipt), isCurrent, release };
    } catch (error) {
        release();
        throw error;
    }
}
