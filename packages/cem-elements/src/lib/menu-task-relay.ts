/** Validated menu relationships supply transient focus relay; DOM IDs grant no authority here. */
interface Menu {
    host: HTMLElement;
    owns(node: Node): boolean;
    parent(): HTMLElement | undefined;
    launcher(): HTMLElement | undefined;
    active(): boolean;
    revision(): number;
    dismiss(): void;
}
interface Popup {
    host: HTMLElement;
    panel(): HTMLElement | undefined;
    launcher(): HTMLElement | undefined;
    active(): boolean;
    revision(): number;
    dismiss(): void;
}
const menus = new Map<HTMLElement, Menu>(), popups = new Map<HTMLElement, Popup>();
const activations = new WeakSet<HTMLElement>();
export function registerCemMenuRelay(menu: Menu): () => void {
    menus.set(menu.host, menu); return () => { if (menus.get(menu.host) === menu) menus.delete(menu.host); };
}
export function registerCemPopupRelay(popup: Popup): () => void {
    popups.set(popup.host, popup); return () => { if (popups.get(popup.host) === popup) popups.delete(popup.host); };
}
function chainFor(node: Node): Menu[] {
    const found = [...menus.values()].filter(menu => menu.owns(node));
    if (found.length !== 1) return [];
    const chain: Menu[] = []; let menu: Menu | undefined = found[0];
    while (menu) {
        if (chain.includes(menu)) return [];
        chain.push(menu);
        const parent = menu.parent(); menu = parent ? menus.get(parent) : undefined;
        if (parent && !menu) return [];
    }
    return chain;
}
/** Includes independently placed submenus linked through admitted parent-item relationships. */
export function containsCemMenuChain(panel: HTMLElement, node: Node | null): boolean {
    return !!node && (panel.contains(node) || chainFor(node).some(menu => panel.contains(menu.host)));
}
export function resetCemTaskActivation(source: HTMLElement): void { activations.delete(source); }
export function consumeCemTaskActivation(source: HTMLElement): boolean {
    const handled = activations.has(source); activations.delete(source); return handled;
}
export interface CemMenuTaskRelay {
    readonly logicalParent: HTMLElement;
    readonly returnDestination: HTMLElement;
    current(): boolean;
    /** Dismiss without menu focus restoration, immediately before task native entry. */
    commit(): boolean;
}
export function captureCemMenuTaskRelay(source?: HTMLElement): CemMenuTaskRelay | undefined {
    if (!source) return;
    const chain = chainFor(source);
    if (!chain.length || chain.some(menu => !menu.active())) return;
    // Even rejected task commands are handled activations: leaf dismissal must not race them.
    activations.add(source);
    const root = chain[chain.length - 1].host;
    const ancestors = [...popups.values()].filter(popup => popup.active() && popup.panel()?.contains(root));
    ancestors.sort((a, b) => a.host.contains(b.host) ? 1 : b.host.contains(a.host) ? -1 : 0);
    const parents = chain.map(menu => menu.parent()), triggers = chain.map(menu => menu.launcher());
    const panels = ancestors.map(popup => popup.panel()), launchers = ancestors.map(popup => popup.launcher());
    const menuRevisions = chain.map(menu => menu.revision()), popupRevisions = ancestors.map(popup => popup.revision());
    const destination = launchers.at(-1) ?? triggers.filter((node): node is HTMLElement => !!node).at(-1) ?? source;
    let committed = false;
    const current = () => !committed && source.isConnected && chain[0].owns(source) && chain.every((menu, i) => menus.get(menu.host) === menu
        && menu.active() && menu.revision() === menuRevisions[i] && menu.parent() === parents[i] && menu.launcher() === triggers[i])
        && ancestors.every((popup, i) => popups.get(popup.host) === popup && popup.active()
            && popup.revision() === popupRevisions[i] && popup.panel() === panels[i] && popup.launcher() === launchers[i]);
    return Object.freeze({ logicalParent: chain[0].host, returnDestination: destination, current,
        commit() {
            if (!current()) return false;
            committed = true;
            for (const menu of chain) menu.dismiss();
            for (const popup of ancestors) popup.dismiss();
            return true;
        },
    });
}
