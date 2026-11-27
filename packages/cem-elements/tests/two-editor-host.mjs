// Browser fixture. CEM owns the UI and dispatch. The host supplies documents, grants and a file service.
import { CemElementRuntime, mountCemView, connectCemApplicationCommand, CemCheckpointPublicationCoordinator }
    from '/runtime/index.js';

let runtime;
export async function editor(parent, id) {
    if (!runtime) {
        runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw Error('Main-thread test renderer'); } });
        const declaration = document.createElement('cem-element');
        declaration.setAttribute('tag', 'test-document-editor');
        declaration.innerHTML = '<template><article><form name="draft"><textarea name="body" aria-label="Draft" host-command-input="edit" slice="edited" slice-event="change"></textarea></form></article></template>';
        document.body.prepend(declaration);
        runtime.registerDeclaration(declaration);
        await runtime.whenDeclarationSettled(declaration);
    }
    let content = id + ' initial draft', revision = 1;
    const view = mountCemView(runtime, { root: parent, tag: 'test-document-editor',
        commands: { edit({ value }) { content = value; revision++; } } });
    await view.whenSettled();
    const control = view.element.querySelector('textarea');
    control.value = content;
    return { root: view.element.querySelector('article'), pin: () => ({ id, revision: String(revision), content }), dispose: view.dispose,
        inspect: () => ({ snapshot: runtime.readInstanceSnapshot(view.element), diagnostics: runtime.diagnosticsFor(view.element),
            errors: view.errors.map(error => error.message),
            island: view.element.querySelector('template[data-cem-island]').outerHTML }) };
}

export async function start(mode, xml, useGeneratedForm) {
    const source = new DOMParser().parseFromString(xml, 'application/xml');
    if (useGeneratedForm) source.querySelector('manifest > template').remove();
    const authority = { runtimeId: 'desktop', generation: 1, policyStamp: 'test-grants' };
    const journal = new CemCheckpointPublicationCoordinator({ ...authority, seal: text => text, allowOwner: () => true });
    const menuButton = document.querySelector('#file'), menuItem = document.querySelector('#save');
    const app = connectCemApplicationCommand({ manifest: source.querySelector('manifest'), commandId: 'file.save-as',
        shortcutProfile: 'editor.windows', menuButton, menuItem, journal, journalAuthority: () => authority });
    const editors = new Map(), results = [], writes = [];
    let receiptGate, releaseReceipt, releasePreparation, releaseControls, localeDone, applied = false;
    const generations = { A: 0, B: 0 };
    async function mount(id) {
        const generation = ++generations[id], container = document.createElement(mode === 'iframe' ? 'iframe' : 'section');
        container.id = id; document.body.append(container);
        let view;
        if (mode === 'iframe') {
            container.title = 'Editor ' + id;
            await new Promise(resolve => { container.onload = resolve; container.src = '/editor.html'; });
            view = await container.contentWindow.makeEditor(container.contentDocument.querySelector('main'), id);
        } else view = await editor(container, id);
        const checkpoint = journal.register(id, () => view.inspect().island);
        const lease = app.register({ root: view.root, instanceId: id, pinDocument: view.pin,
            menuLabels: id === 'A' ? { fr: source.querySelector('message-override').textContent } : undefined,
            preference: { parameter: 'contentType', read: () => localStorage.getItem('lastContentType'),
                write: value => localStorage.setItem('lastContentType', value) },
            async write(request) {
                await window.commitTestFile(request);
                writes.push(request.operationId);
                await receiptGate;
                return { operationId: request.operationId, outcome: 'acknowledged' };
            },
            result(value) { results.push({ id, generation, ...value }); },
        });
        lease.grant(true);
        editors.set(id, { view, checkpoint, lease, container });
    }
    await mount('A'); await mount('B');
    return {
        async inspect() {
            const snapshots = {};
            for (const [id, { view, checkpoint }] of editors) {
                snapshots[id] = view.inspect();
                const { dataRevision, templateArtifactId } = snapshots[id].snapshot;
                checkpoint.published({ instanceId: id, dataRevision, declarationIdentity: templateArtifactId });
            }
            await journal.capture();
            const checkpoint = JSON.parse(new TextDecoder().decode(journal.current().bytes));
            return { snapshots, results, writes, checkpoint, store: { ...localStorage }, applied };
        },
        beginFrench() {
            const ready = new Promise(resolve => { releasePreparation = resolve; });
            applied = false;
            localeDone = app.setLocale('fr', { ready, acknowledge: () => {
                applied = true; return new Promise(resolve => { releaseControls = resolve; });
            } });
        },
        apply() { releasePreparation(); },
        confirm() { releaseControls(); return localeDone; },
        grant(id, allowed) { editors.get(id).lease.grant(allowed); },
        holdReceipt() { receiptGate = new Promise(resolve => { releaseReceipt = resolve; }); },
        releaseReceipt() { releaseReceipt(); },
        async replace(id) {
            const { lease, checkpoint, view, container } = editors.get(id);
            lease.close(); checkpoint.remove(); view.dispose(); container.remove();
            await mount(id);
        },
    };
}
