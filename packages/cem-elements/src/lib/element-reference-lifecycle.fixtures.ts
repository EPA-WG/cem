import type { CemElementReferenceInputs } from './element-reference-inputs.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- fixtures use the explicit native reload export boundary.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';

export const ELEMENT_REFERENCE_TEMPLATE = '{attribute @name=tone | initial}{slice @name=relation}{slice @name=destination}{button @type=button @commandfor={#relation} @command=show-modal | Open}{$destination}{span | {$tone}}';
export function elementReferenceFixtureInputs(target = '{dialog | Native target}'): CemElementReferenceInputs {
    const bundle = (source: string, uri: string) => {
        const id = wasm.parseReferenceSource(new TextEncoder().encode(source), 'text/cem-ml', uri, '');
        try { return wasm.exportReferenceReloadBundle(id, '').slice().buffer as ArrayBuffer; }
        finally { wasm.disposeReferenceSource(id); }
    };
    return { kind: 'cem-element-reference-inputs-v1', requesting: 0,
        sources: [{ bundle: bundle('{#datadom.slices.destination}', 'memory:relationship.cem'), primarySourceId: 1, context: true },
            { bundle: bundle(target, 'memory:target.cem'), primarySourceId: 1, context: true }],
        bindings: [{ source: 0, name: 'relation', select: 'input.children' }, { source: 1, name: 'destination', select: 'input.children' }],
        grants: [[0, 1]] };
}
