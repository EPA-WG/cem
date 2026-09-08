import storybook from 'eslint-plugin-storybook';

import baseConfig from '../../eslint.config.mjs';

export default [
    ...baseConfig,
    {
        ignores: ['**/dist', '**/storybook-static'],
    },
    ...storybook.configs['flat/recommended'],
];
