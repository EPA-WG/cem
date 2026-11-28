import { getCemApplicationStorage, grantCemApplicationBrowserServices, installCemElementRuntime } from '../../../packages/cem-elements/dist/index.js';

const workbench = document.querySelector('cem-application');
const framed = new URL(location.href).searchParams.get('host') === 'iframe';
const lifetime = workbench.querySelector('select');
const template = document.querySelector('#route-declaration');
const apps = new Map();
const controls = document.querySelector('#storage-controls');
const writable = { location: 'read-write', 'local-storage': 'read-write', 'session-storage': 'read-write' };
const readable = { location: 'read', 'local-storage': 'read', 'session-storage': 'read' };

const shellRuntime = installCemElementRuntime(window);
const initialRoutes = [];
if (!framed) document.body.append(template.content.cloneNode(true));

async function addRoute(app) {
    const host = app.ownerDocument.defaultView;
    await host.customElements.whenDefined('workspace-route');
    const route = app.ownerDocument.createElement('workspace-route');
    route.setAttribute('destination', '#' + app.id);
    app.append(route);
    await (framed ? host.routeRuntime : shellRuntime).whenRenderSettled(route);
}

for (let app of workbench.querySelectorAll('cem-application')) {
    const section = app.parentElement;
    const details = document.createElement('details');
    const summary = document.createElement('summary');
    const source = document.createElement('pre');
    summary.textContent = 'Application declaration';
    source.textContent = app.outerHTML;
    details.append(summary, source);
    section.append(details);
    if (framed) {
        const frame = document.createElement('iframe');
        frame.title = section.querySelector('h2').textContent;
        frame.src = './frame.html';
        const loaded = new Promise(resolve => frame.addEventListener('load', resolve, { once: true }));
        app.replaceWith(frame);
        await loaded;
        const doc = frame.contentDocument;
        app = doc.importNode(app, true);
        doc.body.append(app, doc.importNode(template.content, true));
    }
    apps.set(app.id, app);
    if (app.id === 'editor' || app.id === 'notes') {
        const scope = app.id === 'notes' ? 'workspace' : undefined;
        grantCemApplicationBrowserServices(workbench, app, { scope, groupId: 'project-1', ...writable });
    }
    if (app.id !== 'review') initialRoutes.push(addRoute(app));
    section.append(controls.content.cloneNode(true));
    const form = section.querySelector('form');
    const output = form.querySelector('output');
    const store = () => getCemApplicationStorage(app, lifetime.value);
    form.onsubmit = event => {
        event.preventDefault();
        try {
            store().setItem('draft', form.elements.draft.value);
            output.value = 'Saved';
        } catch { output.value = 'Write unavailable'; }
    };
    form.querySelector('[type=button]').onclick = () => {
        try { output.value = store().getItem('draft') ?? 'Empty'; }
        catch { output.value = 'Read unavailable'; }
    };
}
const review = apps.get('review');
const issue = groupId => grantCemApplicationBrowserServices(workbench, review, { scope: 'workspace', groupId, ...readable });
let grant = issue('project-1');
initialRoutes.push(addRoute(review));
const original = getCemApplicationStorage(review);
const status = document.querySelector('#grant-status');
document.querySelector('#withdraw').onclick = () => {
    grant.revoke();
    review.querySelector('workspace-route')?.remove();
    status.value = 'Reviewer access withdrawn';
};
document.querySelector('#probe').onclick = () => {
    try { status.value = original.getItem('draft') ?? 'Empty'; }
    catch { status.value = 'Retained handle refused'; }
};
document.querySelector('#replace').onclick = async () => {
    grant = issue('project-2');
    review.querySelector('workspace-route')?.remove();
    await addRoute(review);
    status.value = 'Reviewer bound to a separate group';
};

const editor = apps.get('editor');
const preview = editor.ownerDocument.createElement('cem-application');
preview.innerHTML = '<browser-policy scope="workspace" local-storage="shared"></browser-policy>';
editor.append(preview);
document.querySelector('#nested-read').onclick = () => {
    const result = document.querySelector('#nested-status');
    try { result.value = getCemApplicationStorage(preview).getItem('draft') ?? 'Empty'; }
    catch { result.value = 'Nested preview has no export'; }
};
await Promise.all(initialRoutes);
workbench.inert = false;
