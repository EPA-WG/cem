import { CEM_COMPONENT_PRIMITIVES } from '@epa-wg/cem-components';
import '@epa-wg/custom-element';

import { mountCemStudio } from '@epa-wg/cem-studio';

const mounted = mountCemStudio();

Object.defineProperty(globalThis, '__cemStudioBootstrap', {
    configurable: false,
    enumerable: false,
    writable: false,
    value: Object.freeze({
        mounted: true,
        componentCount: CEM_COMPONENT_PRIMITIVES.length,
        baseUrl: mounted.baseUrl.href,
    }),
});
