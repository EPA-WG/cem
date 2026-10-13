import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { mergeRenderedFragmentIntoRange } from './projection.js';

export default { title: 'CEM Elements/Projection Focus Ownership', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

export const RemovedOwnerCannotBorrowSiblingControl: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('Missing fixture root');
        const start = document.createComment('start'), end = document.createComment('end');
        root.append(start, end);
        const owner = (name: string, tag = 'button') => `<div data-cem-render-node-id="${name}"><template data-cem-island="instance"></template><${tag} data-cem-render-node-id="shared-control">${name}</${tag}></div>`;
        const merge = (html: string) => {
            const template = document.createElement('template'); template.innerHTML = html;
            mergeRenderedFragmentIntoRange({ start, end }, template.content);
        };
        merge(owner('first') + owner('second'));
        const sibling = root.querySelector<HTMLButtonElement>('button');
        const removed = root.querySelector<HTMLButtonElement>('div:nth-of-type(2) button');
        if (!sibling || !removed) throw new Error('Missing fixture controls');
        removed.focus();
        merge(owner('first'));
        await expect(document.activeElement).not.toBe(sibling);
        await expect(document.activeElement).toBe(document.body);

        // A replacement within the same retained produced owner still restores focus.
        sibling.focus(); merge(owner('first', 'input'));
        await expect(document.activeElement).toBe(root.querySelector('input'));
    },
};
