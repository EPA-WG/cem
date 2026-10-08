/** Transient DOM geometry ownership, independent of visibility and focus policy. */
type StyleValue = { value: string; priority: string };
type StyleClaim = { saved: StyleValue; applied: StyleValue };
interface GeometryObserver {
    host: HTMLElement;
    panel?: HTMLElement;
    update: () => void;
    resize: ResizeObserver;
    mutations: MutationObserver;
    targets: Set<Element>;
    generation: number;
    frame?: number;
    disposed: boolean;
    view: Window;
}
interface GeometryState {
    styles: WeakMap<HTMLElement, Map<string, StyleClaim>>;
    placements: WeakMap<HTMLElement, { saved: string | null; applied: string }>;
    owners: WeakMap<HTMLElement, HTMLElement>;
    targets: WeakMap<HTMLElement, Map<HTMLElement, Element[]>>;
    observers: WeakMap<HTMLElement, Set<GeometryObserver>>;
    leases: WeakMap<HTMLElement, object>;
}
// Source and packaged runtimes share transient ownership in this realm only.
const GEOMETRY = Symbol.for('cem.surface-geometry.v1');
const environment = globalThis as typeof globalThis & { [GEOMETRY]?: GeometryState };
const state = environment[GEOMETRY] ??= {
    styles: new WeakMap(), placements: new WeakMap(), owners: new WeakMap(),
    targets: new WeakMap(), observers: new WeakMap(), leases: new WeakMap(),
};
function readStyle(panel: HTMLElement, name: string): StyleValue {
    return { value: panel.style.getPropertyValue(name), priority: panel.style.getPropertyPriority(name) };
}
function same(a: StyleValue, b: StyleValue): boolean { return a.value === b.value && a.priority === b.priority; }
function properties(name: string): string[] {
    if (name === 'margin') return ['margin-top', 'margin-right', 'margin-bottom', 'margin-left'];
    if (name === 'overflow') return ['overflow-x', 'overflow-y'];
    return [name];
}
export function writeGeometryStyle(panel: HTMLElement, name: string, value: string): void {
    let claims = state.styles.get(panel);
    if (!claims) { claims = new Map(); state.styles.set(panel, claims); }
    const saved = properties(name).map(property => {
        const current = readStyle(panel, property), previous = claims.get(property);
        return { property, saved: previous && same(current, previous.applied) ? previous.saved : current };
    });
    const current = readStyle(panel, name);
    if (current.value !== value || current.priority) panel.style.setProperty(name, value);
    for (const entry of saved) claims.set(entry.property, { saved: entry.saved, applied: readStyle(panel, entry.property) });
}
export function restoreGeometryStyle(panel: HTMLElement, name: string): void {
    const claims = state.styles.get(panel);
    for (const property of properties(name)) {
        const claim = claims?.get(property);
        if (!claim) continue;
        if (same(readStyle(panel, property), claim.applied)) {
            if (claim.saved.value) panel.style.setProperty(property, claim.saved.value, claim.saved.priority);
            else panel.style.removeProperty(property);
        }
        claims?.delete(property);
    }
    if (!claims?.size) state.styles.delete(panel);
}
export function writeGeometryPlacement(panel: HTMLElement, placement: string): void {
    const current = panel.getAttribute('data-placement'), previous = state.placements.get(panel);
    const saved = previous && current === previous.applied ? previous.saved : current;
    if (current !== placement) panel.setAttribute('data-placement', placement);
    state.placements.set(panel, { saved, applied: placement });
}
export function ownsSurfaceGeometry(panel: HTMLElement, token?: object): boolean {
    const owner = state.leases.get(panel);
    return token ? owner === token : !owner;
}
export function claimSurfaceGeometry(panel: HTMLElement): object {
    if (state.leases.has(panel) || state.owners.has(panel) || state.styles.has(panel)) throw new TypeError('Surface already has a geometry owner');
    const token = {}; state.leases.set(panel, token); return token;
}
export function releaseSurfaceGeometryClaim(panel: HTMLElement, token: object): void {
    if (state.leases.get(panel) === token) state.leases.delete(panel);
}
function cancelFrame(observer: GeometryObserver): void {
    observer.generation++;
    if (observer.frame !== undefined) observer.view.cancelAnimationFrame(observer.frame);
    observer.frame = undefined;
}
function schedule(observer: GeometryObserver): void {
    if (observer.disposed || !observer.targets.size || observer.frame !== undefined) return;
    const generation = observer.generation;
    observer.frame = observer.view.requestAnimationFrame(() => {
        observer.frame = undefined;
        if (!observer.disposed && observer.targets.size && observer.generation === generation) observer.update();
    });
}
function refresh(observer: GeometryObserver): void {
    const surfaces = state.targets.get(observer.host);
    const selected = observer.panel ? [[observer.panel, surfaces?.get(observer.panel) ?? []] as const] : [...surfaces ?? []];
    const next = new Set(selected.flatMap(([, targets]) => targets));
    if (next.size === observer.targets.size && [...next].every(target => observer.targets.has(target))) return;
    cancelFrame(observer);
    for (const target of observer.targets) if (!next.has(target)) observer.resize.unobserve(target);
    for (const target of next) if (!observer.targets.has(target)) observer.resize.observe(target);
    observer.mutations.disconnect();
    const panels = new Set(selected.map(([panel]) => panel));
    // Only this registration's own panels suppress style observation. An anchor
    // that is another surface must still report position/class changes.
    for (const target of next) if (!panels.has(target as HTMLElement)) observer.mutations.observe(target, { attributes: true, attributeFilter: ['style', 'class'] });
    observer.targets = next;
}
export function watchSurfaceGeometry(host: HTMLElement, panel: HTMLElement, targets: Element[]): void {
    const previous = state.owners.get(panel);
    if (previous && previous !== host) forgetTargets(previous, panel);
    state.owners.set(panel, host);
    let surfaces = state.targets.get(host);
    if (!surfaces) { surfaces = new Map(); state.targets.set(host, surfaces); }
    surfaces.set(panel, targets);
    for (const observer of state.observers.get(host) ?? []) refresh(observer);
}
function forgetTargets(host: HTMLElement, panel: HTMLElement): void {
    const surfaces = state.targets.get(host); surfaces?.delete(panel);
    if (!surfaces?.size) state.targets.delete(host);
    for (const observer of state.observers.get(host) ?? []) refresh(observer);
}
export function releaseSurfaceGeometry(panel: HTMLElement, token?: object): void {
    if (!ownsSurfaceGeometry(panel, token)) return;
    const host = state.owners.get(panel);
    if (host) forgetTargets(host, panel);
    state.owners.delete(panel);
    for (const name of [...state.styles.get(panel)?.keys() ?? []]) restoreGeometryStyle(panel, name);
    const placement = state.placements.get(panel);
    if (placement && panel.getAttribute('data-placement') === placement.applied) {
        if (placement.saved === null) panel.removeAttribute('data-placement'); else panel.setAttribute('data-placement', placement.saved);
    }
    state.placements.delete(panel);
}
export function observePopupGeometry(host: HTMLElement, update: () => void, panel?: HTMLElement): () => void {
    const view = host.ownerDocument.defaultView;
    if (!view) return () => undefined;
    const observer: GeometryObserver = { host, panel, update, view, targets: new Set(), generation: 0, disposed: false,
        resize: new ResizeObserver(() => schedule(observer)), mutations: new MutationObserver(() => schedule(observer)) };
    let observers = state.observers.get(host);
    if (!observers) { observers = new Set(); state.observers.set(host, observers); }
    observers.add(observer); refresh(observer);
    return () => {
        if (observer.disposed) return;
        observer.disposed = true; cancelFrame(observer); observer.resize.disconnect(); observer.mutations.disconnect();
        observers?.delete(observer);
        if (!observers?.size) state.observers.delete(host);
    };
}
export function queueSurfaceGeometry(host: HTMLElement, panel: HTMLElement): void {
    for (const observer of state.observers.get(host) ?? []) if (observer.panel === panel) schedule(observer);
}
