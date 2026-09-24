import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { verifyExternalFilePreviews } from '../../.storybook/external-file-previews.js';

export default { title: 'CEM Elements/Compatibility File Previews', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

function previewStory(page: string, tag: string, files: readonly string[]): Story {
    const url = new URL(`../../../custom-element/demo/${page}`, import.meta.url);
    return {
        render: () => {
            const root = document.createElement('section');
            const declaration = document.createElement('cem-element');
            declaration.hidden = true;
            declaration.setAttribute('tag', tag);
            declaration.setAttribute('src', url.href);
            root.append(declaration, document.createElement(tag));
            return root;
        },
        play: async ({ canvasElement }) => {
            await verifyExternalFilePreviews(canvasElement, url, files);
        },
    };
}

export const HttpFiles: Story = previewStory('http-request.html', 'story-compat-http-previews',
    ['http-data.json', 'http-data-compact.json', 'http-data-invalid.json', 'http-pokemon.json']);
export const VersionFile: Story = previewStory('npm-versions-demo.html', 'story-compat-version-previews',
    ['npm-versions.json']);
