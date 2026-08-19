import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { edgeSsrStories } from './cem-elements.stories.js';

const meta: Meta = {
    title: 'CEM Elements/Edge SSR',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const SsrHydrationFromSerializedSnapshot: Story =
    edgeSsrStories.SsrHydrationFromSerializedSnapshot;
export const SsrHydrationRejectsUnsupportedSnapshotVersion: Story =
    edgeSsrStories.SsrHydrationRejectsUnsupportedSnapshotVersion;
export const SsrHydrationRejectsIncompleteMarkup: Story =
    edgeSsrStories.SsrHydrationRejectsIncompleteMarkup;
export const EdgePatchFramesFromSerializedSnapshot: Story =
    edgeSsrStories.EdgePatchFramesFromSerializedSnapshot;
export const BrowserToEdgeSnapshotPrivacyPolicy: Story =
    edgeSsrStories.BrowserToEdgeSnapshotPrivacyPolicy;
export const EdgeRenderStateHybridStorageModel: Story =
    edgeSsrStories.EdgeRenderStateHybridStorageModel;
