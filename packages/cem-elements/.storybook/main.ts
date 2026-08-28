import { fileURLToPath } from "node:url";
import { dirname } from "node:path";
import { defineMain } from '@storybook/web-components-vite/node';

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
});

export default config;

function getAbsolutePath(value: string): any {
    return dirname(fileURLToPath(import.meta.resolve(`${value}/package.json`)));
}
