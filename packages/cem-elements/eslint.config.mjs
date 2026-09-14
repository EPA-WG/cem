// For more info, see https://github.com/storybookjs/eslint-plugin-storybook#configuration-flat-config-format
import storybook from "eslint-plugin-storybook";

import baseConfig from '../../eslint.config.mjs';

export default [...baseConfig, {
    files: ['**/*.json'],
    rules: {
        '@nx/dependency-checks': [
            'error',
            {
                ignoredFiles: [
                    '{projectRoot}/eslint.config.{js,cjs,mjs,ts,cts,mts}',
                    '{projectRoot}/vite.config.{js,ts,mjs,mts}',
                ],
            },
        ],
    },
    languageOptions: {
        parser: await import('jsonc-eslint-parser'),
    },
}, {
    ignores: [
        '**/out-tsc', '**/dist', '**/storybook-static',
        // Deliberately malformed HTTP response; the source-contract test
        // requires JSON.parse to fail so the demo can exercise recovery.
        'demo/http-data-invalid.json',
    ],
}, ...storybook.configs["flat/recommended"]];
