import '@epa-wg/custom-element';

import {
    createCemStudioBrowserValidator,
    createCemStudioFeatureTourWorkbench,
    createCemStudioProjectRepository,
    installCemStudioFeatureTour,
    loadCemStudioFeatureTour,
    mountCemStudio,
    mountCemStudioApplicationShell,
    mountCemStudioFeatureTourWorkbench,
    registerCemStudioServiceWorker,
} from '@epa-wg/cem-studio';

const mounted = mountCemStudio();
const registration = await registerCemStudioServiceWorker().catch(() => undefined);
const shell = await mountCemStudioApplicationShell({
    root: mounted.root,
    registration,
});
const validator = await createCemStudioBrowserValidator();
const repository = createCemStudioProjectRepository({
    validateProject: validator.validateProject,
});
const seed = await loadCemStudioFeatureTour({
    baseUrl: new URL('./samples/feature-tour/', mounted.baseUrl),
    validator,
});
const featureTour = await installCemStudioFeatureTour(repository, seed);
const workbench = await createCemStudioFeatureTourWorkbench({
    repository,
    validator,
    seed,
    projectId: featureTour.projectId,
});
const workbenchView = await mountCemStudioFeatureTourWorkbench({
    root: mounted.root,
    workbench,
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
    value: Object.freeze({ shell, repository, validator, seed, featureTour, workbench, workbenchView }),
});
