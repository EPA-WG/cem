import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
import { defineMain } from '@storybook/web-components-vite/node';
import { mergeConfig } from 'vite';

const config = defineMain({
    stories: [
        '../src/**/!(*.edge-ssr).stories.@(js|jsx|mjs|ts|tsx)',
        '../../cem-components/src/components/**/*.stories.ts',
    ],
    addons: [getAbsolutePath("@storybook/addon-vitest")],
    framework: {
        name: getAbsolutePath("@storybook/web-components-vite"),
        options: {},
    },
    core: {
        disableTelemetry: true,
    },
    viteFinal: async (config) => mergeConfig(config, {
        resolve: {
            // Source-loaded demo documents are rebased from Storybook's
            // project root. Their standalone import remains local while
            // preview.ts owns the actual one-time registration.
            alias: {
                '/cem-demo-element/dist/index.js': fileURLToPath(
                    new URL('../../cem-demo-element/src/index.ts', import.meta.url)
                ),
            },
        },
    }),
});

export default config;

function getAbsolutePath(value: string): string {
    return dirname(fileURLToPath(import.meta.resolve(`${value}/package.json`)));
}
