import { SNAPSHOT_SCHEMA_VERSION, type DataIslandSnapshot } from './cem-elements.js';
import type { TemplateSourceNode } from './projection.js';

class HostClassInstance {
    constructor(readonly value: string) {}
}

export const PROCESSING_BOUNDARY_TEMPLATE_SOURCE: TemplateSourceNode[] = [
    {
        kind: 'element',
        namespace: null,
        tag: 'article',
        attributes: [{ name: 'data-label', value: '{$label}' }],
        sourceMapRef: { fidelity: 'dom-canonical', frame: 'dom:0' },
        children: [
            {
                kind: 'element',
                namespace: null,
                tag: 'slot',
                attributes: [{ name: 'name', value: 'detail' }],
                sourceMapRef: { fidelity: 'dom-canonical', frame: 'dom:0/0' },
                children: [
                    {
                        kind: 'text',
                        text: 'fallback',
                        sourceMapRef: { fidelity: 'dom-canonical', frame: 'dom:0/0/0' },
                    },
                ],
            },
        ],
    },
];

export function processingBoundarySnapshotFixture(): DataIslandSnapshot {
    return {
        version: SNAPSHOT_SCHEMA_VERSION,
        instanceId: 'boundary-instance-1',
        producedTag: 'boundary-card',
        declarationTag: 'cem-element-boundary',
        templateArtifactId: 'boundary-template-1',
        dataRevision: '1',
        renderAttempt: 2,
        outputTarget: 'light-dom',
        sourceMapMode: 'dev',
        scopePolicyStamp: 'boundary-scope',
        privacyPolicyStamp: 'boundary-privacy',
        hostAttributes: { label: 'Projected' },
        dataset: { flavor: 'plain' },
        payload: {
            text: 'Detail',
            childCount: 1,
            nodes: [{
                kind: 'element',
                key: 'payload-0',
                tag: 'span',
                namespace: null,
                attributes: { slot: 'detail' },
                slot: 'detail',
                children: [{ kind: 'text', key: 'payload-0/0', text: 'Detail' }],
            }],
            slots: {
                detail: [{
                    kind: 'element',
                    key: 'payload-0',
                    tag: 'span',
                    namespace: null,
                    attributes: { slot: 'detail' },
                    slot: 'detail',
                    children: [{ kind: 'text', key: 'payload-0/0', text: 'Detail' }],
                }],
            },
            elementsByAttribute: {},
            data: [],
            options: [],
            dataByValue: {},
            optionsByValue: {},
        },
        slices: {
            date: new Date('2026-06-17T00:00:00.000Z') as unknown,
            klass: new HostClassInstance('class-value') as unknown,
            primitive: 'ok',
        },
        formData: { signin: { username: 'ada' } },
        validationState: {},
        eventPayloads: {
            fn: (() => 'dropped') as unknown,
            detail: { ok: true },
        },
    };
}
