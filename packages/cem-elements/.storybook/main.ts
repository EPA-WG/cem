import type { StorybookConfig } from '@storybook/web-components-vite';

const config: StorybookConfig = {
    stories: ['../src/**/*.stories.@(js|jsx|mjs|ts|tsx)'],
    addons: ['@storybook/addon-vitest'],
    framework: {
        name: '@storybook/web-components-vite',
        options: {},
    },
    core: {
        disableTelemetry: true,
    },
};

export default config;
