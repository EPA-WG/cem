import { definePreview } from '@storybook/web-components-vite';

import '../src/index.js';

export default definePreview({
    parameters: {
        controls: { disable: true },
        options: { storySort: { method: 'alphabetical' } },
    },
});
