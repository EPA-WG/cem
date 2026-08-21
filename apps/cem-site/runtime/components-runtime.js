import { installCustomElementRuntime } from '@epa-wg/custom-element';
import { installCemComponentPrimitives } from '@epa-wg/cem-components/primitives';

export const componentRuntime = installCustomElementRuntime();
const installed = await installCemComponentPrimitives(componentRuntime);
export const componentRuntimeErrors = installed.diagnostics
    .filter(({ severity }) => severity === 'error' || severity === 'fatal')
    .map(({ code, message }) => `${code}: ${message}`);

globalThis.__cemSiteComponents = {
    done: true,
    errors: componentRuntimeErrors,
};
