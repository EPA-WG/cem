/** Transient DOM geometry ownership, independent of visibility and focus policy. */
type StyleValue = { value: string; priority: string };
type StyleClaim = { saved: StyleValue; applied: StyleValue };
interface AnchorBinding { anchor: HTMLElement; name: string; point?: { left: number; top: number; placement: string } }
interface AnchorNames { saved: StyleValue; applied: StyleValue; names: Set<string>; authored: string[] }
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
    anchorBindings: WeakMap<HTMLElement, AnchorBinding>;
    anchorNames: WeakMap<HTMLElement, AnchorNames>;

}
// Source and packaged runtimes share transient ownership in this realm only.
const GEOMETRY = Symbol.for('cem.surface-geometry.v1');
const environment = globalThis as typeof globalThis & { [GEOMETRY]?: GeometryState };
const state = environment[GEOMETRY] ??= {
    styles: new WeakMap(), placements: new WeakMap(), owners: new WeakMap(),
    targets: new WeakMap(), observers: new WeakMap(), leases: new WeakMap(), anchorBindings: new WeakMap(), anchorNames: new WeakMap(),
};
state.anchorBindings ??= new WeakMap(); state.anchorNames ??= new WeakMap();
function readStyle(panel: HTMLElement, name: string): StyleValue {
    return { value: panel.style.getPropertyValue(name), priority: panel.style.getPropertyPriority(name) };
}
function same(a: StyleValue, b: StyleValue): boolean { return a.value === b.value && a.priority === b.priority; }
function properties(name: string): string[] {
    if (name === 'inset') return ['top', 'right', 'bottom', 'left'];
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
    clearCssSurfaceAnchor(panel);
    const host = state.owners.get(panel);
    if (host) forgetTargets(host, panel);
    state.owners.delete(panel);
    for (const name of [...state.styles.get(panel)?.keys() ?? []]) restoreGeometryStyle(panel, name);
    restoreGeometryPlacement(panel);
}
function restoreGeometryPlacement(panel: HTMLElement): void {
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

/** Read-only checkpoint fence for the legacy popup owner's settled, unmodified claims. */
export function checkpointSurfaceGeometryCurrent(host: HTMLElement, panel: HTMLElement): boolean {
    const claims = state.styles.get(panel);
    const binding = state.anchorBindings.get(panel), names = binding && state.anchorNames.get(binding.anchor);
    return (!binding || !!names && same(readStyle(binding.anchor, 'anchor-name'), names.applied))
        && ownsSurfaceGeometry(panel) && state.owners.get(panel) === host && !!claims?.size
        && [...claims].every(([name, claim]) => same(readStyle(panel, name), claim.applied))
        && ![...state.observers.get(host) ?? []].some(observer => observer.targets.has(panel) && observer.frame !== undefined);
}


const anchorProperties = ['position-anchor', '--_cem-surface-area', '--_cem-surface-tries', '--_cem-surface-position-mode', 'inset', 'margin'];
/** Named anchors are shared by panels and never replace an author's names. */
export function bindCssSurfaceAnchor(panel: HTMLElement, anchor: HTMLElement): string {
    const previous = state.anchorBindings.get(panel);
    if (previous?.anchor !== anchor) clearCssSurfaceAnchor(panel);
    let binding = state.anchorBindings.get(panel);
    let names = state.anchorNames.get(anchor);
    const current = readStyle(anchor, 'anchor-name');
    if (!names) {
        names = { saved: current, applied: current, names: new Set(), authored: [] }; state.anchorNames.set(anchor, names);
        names.authored = (current.value || getComputedStyle(anchor).getPropertyValue('anchor-name')).split(',').map(value => value.trim()).filter(value => value && value !== 'none');
    } else if (!same(current, names.applied)) {
        names.saved = current;
        names.authored = (current.value || getComputedStyle(anchor).getPropertyValue('anchor-name')).split(',').map(value => value.trim()).filter(value => value && value !== 'none' && !names!.names.has(value));
    }
    if (!binding) {
        binding = { anchor, name: `--cem-surface-${crypto.randomUUID()}` };
        names.names.add(binding.name); state.anchorBindings.set(panel, binding);
    }
    const value = [...new Set([...names.authored, ...names.names])].join(', ');
    if (current.value !== value || current.priority !== names.saved.priority) anchor.style.setProperty('anchor-name', value, names.saved.priority);
    names.applied = readStyle(anchor, 'anchor-name');
    return binding.name;
}
export function clearCssSurfaceAnchor(panel: HTMLElement): void {
    const binding = state.anchorBindings.get(panel);
    if (!binding) return;
    const names = state.anchorNames.get(binding.anchor);
    if (names) {
        const current = readStyle(binding.anchor, 'anchor-name'), managed = new Set(names.names);
        names.names.delete(binding.name);
        if (!same(current, names.applied) && names.names.size) {
            names.saved = current;
            names.authored = (current.value || getComputedStyle(binding.anchor).getPropertyValue('anchor-name')).split(',').map(value => value.trim()).filter(value => value && value !== 'none' && !managed.has(value));
            binding.anchor.style.setProperty('anchor-name', [...names.authored, ...names.names].join(', '), names.saved.priority);
            names.applied = readStyle(binding.anchor, 'anchor-name');
        } else if (same(current, names.applied)) {
            if (!names.names.size) {
                if (names.saved.value) binding.anchor.style.setProperty('anchor-name', names.saved.value, names.saved.priority);
                else binding.anchor.style.removeProperty('anchor-name');
            } else {
                binding.anchor.style.setProperty('anchor-name', [...names.authored, ...names.names].join(', '), names.saved.priority);
                names.applied = readStyle(binding.anchor, 'anchor-name');
            }
        }
        if (!names.names.size) state.anchorNames.delete(binding.anchor);
    }
    state.anchorBindings.delete(panel);
    for (const property of anchorProperties) restoreGeometryStyle(panel, property);
    restoreGeometryPlacement(panel);
}

export function rememberCssSurfaceAnchor(panel: HTMLElement, point: { left: number; top: number; placement: string }): void {
    const binding = state.anchorBindings.get(panel);
    if (binding) binding.point = point;
}
/** A frozen surface must stop depending on an anchor that is no longer laid out. */
export function freezeCssSurfaceAnchor(panel: HTMLElement): void {
    const point = state.anchorBindings.get(panel)?.point;
    if (!point) return;
    clearCssSurfaceAnchor(panel);
    writeGeometryStyle(panel, '--_cem-surface-position-mode', 'fixed');
    writeGeometryStyle(panel, 'position', 'fixed'); writeGeometryStyle(panel, 'margin', '0');
    writeGeometryStyle(panel, 'right', 'auto'); writeGeometryStyle(panel, 'bottom', 'auto');
    writeGeometryStyle(panel, 'left', `${point.left}px`); writeGeometryStyle(panel, 'top', `${point.top}px`);
    writeGeometryPlacement(panel, point.placement);
}
