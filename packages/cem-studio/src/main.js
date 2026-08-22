import '@epa-wg/custom-element';

import {
    mountCemStudio,
    mountCemStudioApplicationShell,
    registerCemStudioServiceWorker,
} from '@epa-wg/cem-studio';

const mounted = mountCemStudio();
const registration = await registerCemStudioServiceWorker().catch(() => undefined);
const shell = await mountCemStudioApplicationShell({
    root: mounted.root,
    registration,
});

Object.defineProperty(globalThis, '__cemStudioBootstrap', {
    configurable: false,
    enumerable: false,
    writable: false,
    value: Object.freeze({
        mounted: true,
        baseUrl: mounted.baseUrl.href,
    }),
});

Object.defineProperty(globalThis, '__cemStudioApplication', {
    configurable: false,
    enumerable: false,
    writable: false,
    value: Object.freeze({ shell }),
});
