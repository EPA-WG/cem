import { dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import { defineMain } from '@storybook/web-components-vite/node';

const config = defineMain({
    stories: ['../src/**/*.stories.ts'],
    addons: [getAbsolutePath('@storybook/addon-vitest')],
    framework: {
        name: getAbsolutePath('@storybook/web-components-vite'),
        options: {},
    },
    core: {
        disableTelemetry: true,
    },
});

export default config;

function getAbsolutePath(value: string): string {
    return dirname(fileURLToPath(import.meta.resolve(`${value}/package.json`)));
}
