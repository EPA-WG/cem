/** Bounded name-presence checks for native CEM control owners, not an AccName string API. */
const controls = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>(':scope > :is(button,a)[part~="control"]')];
const tokens = (value: string | null) => value?.trim().split(/\s+/).filter(Boolean) ?? [];
const nonempty = (value: string | null | undefined) => !!value?.trim();
type Root = Document | ShadowRoot;
type Verdict = boolean | undefined;
interface Reference { root: Root; ids: string[]; targets: (Element | null)[]; }
interface Entry { host: HTMLElement; request(): void; references: Reference[]; }
const roots = new WeakMap<Root, { entries: Set<Entry>; observer: MutationObserver }>();

function hidden(node: Element): boolean {
    if (node.matches('[hidden],[inert],[aria-hidden="true"]')) return true;
    const style = node.ownerDocument.defaultView?.getComputedStyle(node);
    return style?.display === 'none' || style?.visibility === 'hidden' || style?.visibility === 'collapse' || style?.contentVisibility === 'hidden';
}
function reference(node: Element): Reference {
    const root = node.getRootNode() as Root;
    const ids = tokens(node.getAttribute('aria-labelledby')).slice(0, 65);
    return { root, ids, targets: ids.map(id => root.getElementById?.(id) ?? null) };
}

/** Unknown/oversize structures never become a false missing-name warning. */
export function nativeControlHasName(control: HTMLElement): Verdict {
    let remaining = 1024;
    const visited = new Map<Node, number>();
    const walk = (node: Node, referenced = false, includeHidden = false, depth = 0): Verdict => {
        if (--remaining < 0 || depth > 128) return undefined;
        if (node.nodeType === 3) return nonempty(node.textContent);
        if (node.nodeType !== 1) return false;
        const element = node as HTMLElement;
        if (element.matches('template,script,style')) return false;
        if (element !== control && !includeHidden && hidden(element)) return false;
        const mode = referenced ? 2 : 1;
        if ((visited.get(node) ?? 0) & mode) return false;
        visited.set(node, (visited.get(node) ?? 0) | mode);
        if (!referenced) {
            const refs = reference(element);
            if (refs.ids.length > 64) return undefined;
            const targets = refs.targets.filter((target): target is Element => !!target);
            if (targets.length) return any(targets.map(target => walk(target, true, hidden(target), depth + 1)));
        }
        if (nonempty(element.getAttribute('aria-label'))) return true;
        if (element.localName === 'img' && element.hasAttribute('alt')) return nonempty(element.getAttribute('alt'));
        const labels = (element as HTMLButtonElement).labels;
        if (labels?.length) return any([...labels].map(label => walk(label, true, hidden(label), depth + 1)));
        // These need a general accessibility-tree engine; don't misdiagnose them.
        if (element.hasAttribute('aria-owns') || element.shadowRoot
            || element !== control && element.matches('input,select,textarea,[contenteditable]')) return undefined;
        const children = any([...element.childNodes].map(child => walk(child, referenced, includeHidden, depth + 1)));
        if (children !== false) return children;
        // Generated text/alt syntax needs the browser's full name computation.
        for (const pseudo of ['::before', '::after']) {
            const content = element.ownerDocument.defaultView?.getComputedStyle(element, pseudo).content;
            if (content && !['none', 'normal', '""', "''"].includes(content) && !/\/\s*(?:""|'')\s*$/.test(content)) return undefined;
        }
        return nonempty(element.getAttribute('title'));
    };
    return walk(control);
}
function any(values: Verdict[]): Verdict {
    return values.includes(true) ? true : values.includes(undefined) ? undefined : false;
}

function bindReferences(entry: Entry): void {
    const previous = new Set(entry.references.map(ref => ref.root));
    entry.references = controls(entry.host).map(reference).filter(ref => ref.ids.length > 0 && ref.ids.length <= 64);
    const next = new Set(entry.references.map(ref => ref.root));
    for (const root of previous) if (!next.has(root)) releaseReferences(root, entry);
    for (const root of next) {
        let group = roots.get(root);
        if (!group) {
            const entries = new Set<Entry>();
            const observer = new MutationObserver(records => {
                for (const member of entries) {
                    const changed = member.references.some(ref => ref.root === root && ref.ids.some((id, index) => {
                        const target = root.getElementById(id);
                        return target !== ref.targets[index] || !!target && records.some(record =>
                            target.contains(record.target) || record.type === 'attributes' && record.target.contains(target));
                    }));
                    if (changed) member.request();
                }
            });
            observer.observe(root, { subtree: true, childList: true, characterData: true, attributes: true,
                attributeFilter: ['id', 'aria-label', 'aria-labelledby', 'aria-hidden', 'alt', 'title', 'hidden', 'inert', 'class', 'style'] });
            group = { entries, observer }; roots.set(root, group);
        }
        group.entries.add(entry);
    }
}
function releaseReferences(root: Root, entry: Entry): void {
    const group = roots.get(root);
    group?.entries.delete(entry);
    if (group?.entries.size === 0) { group.observer.disconnect(); roots.delete(root); }
}

/** No DOM writes or activation listeners: diagnostics never change control behavior. */
export function watchCemControlNames(host: HTMLElement, settle: () => Promise<void>, report: (missing: boolean) => void): () => void {
    let active = true, pending = false, revision = 0;
    const entry: Entry = { host, references: [], request: () => {
        revision++;
        if (pending || !active) return;
        pending = true;
        void Promise.resolve().then(async () => {
            while (active && host.isConnected) {
                const current = revision;
                await settle();
                if (!active || !host.isConnected) break;
                if (current !== revision) continue;
                bindReferences(entry);
                report(controls(host).some(control => nativeControlHasName(control) === false));
                break;
            }
        }).catch(() => undefined).finally(() => { pending = false; });
    } };
    const observer = new MutationObserver(entry.request);
    observer.observe(host, { subtree: true, childList: true, characterData: true, attributes: true,
        attributeFilter: ['aria-label', 'aria-labelledby', 'aria-hidden', 'alt', 'title', 'id', 'hidden', 'inert', 'class', 'style', 'href', 'part'] });
    entry.request();
    return () => {
        active = false; observer.disconnect();
        for (const root of new Set(entry.references.map(ref => ref.root))) releaseReferences(root, entry);
        report(false);
    };
}
