import { installCemElementRuntime, mountCemView } from '@epa-wg/cem-elements';

export const componentRuntime = installCemElementRuntime();
export const componentRuntimeErrors = [];

// Declaration registration is bootstrap metadata; all visible UI lives in XHTML.
export async function loadDeclaration(tag, source, capability) {
    const declaration = document.createElement('cem-element');
    declaration.setAttribute('tag', tag);
    declaration.setAttribute('src', source);
    if (capability) declaration.setAttribute('capability', capability);
    document.head.append(declaration);
    componentRuntime.registerDeclaration(declaration);
    await componentRuntime.whenDeclarationSettled(declaration);
    const errors = componentRuntime.diagnosticsFor(declaration)
        .filter(({ severity }) => severity === 'error' || severity === 'fatal')
        .map(({ code, message }) => `${code}: ${message}`);
    if (errors.length) throw new Error(errors.join('\n'));
}

let installation;
async function installComponents() {
    installation ??= (async () => {
        try {
            for (const [tag, capability] of [
                ['cem-action', 'action-command'], ['cem-field', 'form-control'],
                ['cem-surface'], ['cem-media-preview'],
            ]) {
                await loadDeclaration(tag, `./assets/cem-components/${tag}.xhtml#${tag}`, capability);
            }
        } catch (error) {
            componentRuntimeErrors.push(error instanceof Error ? error.message : String(error));
        }
        globalThis.__cemSiteComponents = { done: true, errors: componentRuntimeErrors };
    })();
    await installation;
}

export async function mountSiteView(root, tag, source, state, commands) {
    await installComponents();
    if (componentRuntimeErrors.length) throw new Error(componentRuntimeErrors.join('\n'));
    await loadDeclaration(tag, source);
    const view = mountCemView(componentRuntime, { root, tag, state: { state }, commands });
    await view.whenSettled();
    return view;
}

export function viewErrors(view) {
    return [...view.errors.map(error => error.message), ...[view.element, ...view.element.querySelectorAll('*')]
        .flatMap(element => componentRuntime.diagnosticsFor(element))
        .filter(({ severity }) => severity === 'error' || severity === 'fatal')
        .map(({ code, message }) => `${code}: ${message}`)];
}
