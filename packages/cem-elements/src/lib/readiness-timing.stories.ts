import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { httpReadinessMetadata } from '../../.storybook/tree-processing-timing.js';

const meta: Meta = { title: 'CEM Elements/Readiness Trace', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const HttpMetadataExcludesDocumentData: Story = {
    render: () => document.createElement('section'),
    play: () => {
        const forbidden = () => { throw new Error('readiness tracing must not read document or private response data'); };
        const response = { status: 200, get headers() { return forbidden(); } };
        const resource = {
            kind: 'http-request', state: 'loaded', resourceRevision: 4, response,
            get data() { return forbidden(); },
            get request() { return forbidden(); },
            get sourceId() { return forbidden(); },
        };
        const slices = {
            registry: resource,
            pending: { kind: 'http-request', state: 'in-progress', resourceRevision: 2 },
            selection: '0.1.0', absent: null,
            native: { kind: 'cem-value', get data() { return forbidden(); } },
        };
        expect(httpReadinessMetadata(slices)).toEqual([
            { slice: 'registry', state: 'loaded', resourceRevision: 4, status: 200 },
            { slice: 'pending', state: 'in-progress', resourceRevision: 2, status: undefined },
        ]);
        expect(httpReadinessMetadata({})).toEqual([]);
        // Keep Storybook's assertion recorder from serializing the guarded inputs.
        expect(slices.registry === resource).toBe(true);
        expect(resource.response === response).toBe(true);
    },
};
