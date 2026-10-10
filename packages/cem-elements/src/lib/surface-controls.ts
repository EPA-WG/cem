import { interactionControl, interactionReference, nativeInteractionReference, reportInteractionReference } from './interaction-reference.js';

interface Claim { before: string | null; value: string }
type Attributes = Map<HTMLElement, Map<string, string>>;
let identifiers = 0;

/** Native command routes and their reversible accessibility projection. */
export class CemSurfaceControls {
    private readonly claims = new Map<HTMLElement, Map<string, Claim>>();
    private trigger?: HTMLButtonElement;
    private close?: HTMLButtonElement;
    private triggerSource?: HTMLElement;
    private closeSource?: HTMLElement;
    private refreshing = false;
    constructor(private readonly owner: HTMLElement, private readonly host: HTMLElement) {}
    owns(node: Node): boolean { return node === this.trigger || node === this.close; }
    ownsAttribute(node: Element, name: string): boolean {
        const claim = node instanceof HTMLElement && this.claims.get(node)?.get(name);
        return !!claim && node.getAttribute(name) === claim.value;
    }
    chrome(node: Node): boolean { return this.owns(node) || node instanceof HTMLElement && (node.getAttribute('slot') === 'close' || node.matches('[part~=heading]')); }
    isClose(source: Element | null): boolean { return !!source && source === this.closeSource; }
    popoverAction(source: HTMLElement): string | undefined {
        if ((source === this.triggerSource || source === this.closeSource) && source.hasAttribute('popovertarget')) return source.getAttribute('popovertargetaction') ?? 'toggle';
        return;
    }
    private original(node: HTMLElement, name: string): string | null {
        const claim = this.claims.get(node)?.get(name), current = node.getAttribute(name);
        return claim && current === claim.value ? claim.before : current;
    }
    private reconcile(desired: Attributes): void {
        for (const [node, claims] of this.claims) {
            for (const [name, claim] of claims) if (!desired.get(node)?.has(name)) {
                if (node.getAttribute(name) === claim.value) {
                    if (claim.before === null) node.removeAttribute(name); else node.setAttribute(name, claim.before);
                }
                claims.delete(name);
            }
            if (!claims.size) this.claims.delete(node);
        }
        for (const [node, attributes] of desired) for (const [name, value] of attributes) {
            let claims = this.claims.get(node);
            if (!claims) { claims = new Map(); this.claims.set(node, claims); }
            const prior = claims.get(name), current = node.getAttribute(name);
            claims.set(name, { before: prior && current === prior.value ? prior.before : current, value });
            if (current !== value) node.setAttribute(name, value);
        }
    }
    refresh(enabled: boolean, open: boolean): void {
        if (this.refreshing) return;
        this.refreshing = true;
        try { this.update(enabled, open); } finally { this.refreshing = false; }
    }
    private update(enabled: boolean, open: boolean): void {
        const desired: Attributes = new Map();
        const set = (node: HTMLElement, name: string, value: string) => {
            let attributes = desired.get(node); if (!attributes) { attributes = new Map(); desired.set(node, attributes); }
            attributes.set(name, value);
        };
        const heading = this.owner.querySelector<HTMLElement>(':scope > [part~=heading]');
        if (enabled && heading && !this.original(this.owner, 'aria-labelledby') && !this.original(this.owner, 'aria-label')) {
            let id = heading.id;
            if (!id) do { id = `cem-native-heading-${++identifiers}`; } while (nativeInteractionReference(heading, id).code !== 'interaction-reference-missing');
            if (!heading.id || this.ownsAttribute(heading, 'id')) set(heading, 'id', id);
            set(this.owner, 'aria-labelledby', id);
        }
        const local = [...this.host.children].filter((node): node is HTMLElement => node instanceof HTMLElement && node !== this.owner && node !== this.trigger
            && (node.getAttribute('slot') === 'trigger' || node.matches('[part~=trigger]')));
        const slots = [...this.owner.querySelectorAll<HTMLElement>('[slot=close]')].filter(node => node.closest('dialog,[popover],[part~=surface]') === this.owner);
        const external = this.host.getAttribute('trigger-for'), label = this.host.getAttribute('trigger');
        const ambiguous = external !== null && (local.length > 0 || label !== null) || local.length > 1;
        let source: HTMLElement | undefined, triggerCode: string | undefined, closeCode: string | undefined;
        if (enabled && ambiguous) triggerCode = 'interaction-trigger-ambiguous';
        else if (enabled && external !== null) {
            const reference = interactionReference(this.host, external, 'trigger-for'); source = interactionControl(reference.target);
            triggerCode = reference.code ?? (!source ? 'interaction-control-unsupported' : undefined);
        } else if (enabled && local.length) {
            source = interactionControl(local[0]); if (!source) triggerCode = 'interaction-control-unsupported';
        }
        const generate = enabled && !ambiguous && external === null && !local.length && label !== null;
        const ariaLabel = this.host.getAttribute('trigger-aria-label');
        if (generate && !label?.trim() && !ariaLabel?.trim()) triggerCode = 'interaction-control-name-missing';
        if (!generate || triggerCode) { this.trigger?.remove(); this.trigger = undefined; }
        else {
            if (!this.trigger) {
                this.trigger = this.owner.ownerDocument.createElement('button'); this.trigger.type = 'button'; this.trigger.setAttribute('part', 'trigger');
                this.owner.before(this.trigger);
            }
            if (this.trigger.textContent !== label) this.trigger.textContent = label;
            set(this.trigger, 'type', 'button');
            if (ariaLabel !== null) set(this.trigger, 'aria-label', ariaLabel);
            if (this.host.hasAttribute('trigger-disabled')) set(this.trigger, 'disabled', '');
            source = this.trigger;
        }
        const closeLabel = this.host.getAttribute('close-label');
        if (!enabled || slots.length || closeLabel === null || !closeLabel.trim()) { this.close?.remove(); this.close = undefined; }
        else {
            if (!this.close) {
                this.close = this.owner.ownerDocument.createElement('button'); this.close.type = 'button'; this.close.setAttribute('part', 'close'); this.owner.append(this.close);
            }
            if (this.close.textContent !== closeLabel) this.close.textContent = closeLabel;
            set(this.close, 'type', 'button');
        }
        const closeSource = slots.length === 1 ? interactionControl(slots[0]) : this.close;
        if (enabled && slots.length > 1) closeCode = 'interaction-close-ambiguous';
        else if (enabled && slots.length && !closeSource) closeCode = 'interaction-control-unsupported';
        else if (enabled && !slots.length && closeLabel !== null && !closeLabel.trim()) closeCode = 'interaction-control-name-missing';
        // IDs are routing tokens only, never retained native-value evidence.
        if (enabled && (source || closeSource || this.host.hasAttribute('surface-id'))) {
            const forwarded = this.host.getAttribute('surface-id');
            if (forwarded && !this.original(this.owner, 'id')) set(this.owner, 'id', forwarded);
            else if (this.ownsAttribute(this.owner, 'id')) set(this.owner, 'id', this.owner.id);
            else if (!this.owner.id) {
                let id: string;
                do { id = `cem-native-surface-${++identifiers}`; } while (nativeInteractionReference(this.owner, id).code !== 'interaction-reference-missing');
                set(this.owner, 'id', id);
            }
            // Routes below must resolve against the identifier being published now.
            const id = desired.get(this.owner)?.get('id') ?? this.owner.id;
            const conflict = forwarded !== null && (!forwarded || /\s/.test(forwarded))
                || !!forwarded && !!this.original(this.owner, 'id') && forwarded !== this.owner.id
                || [...(this.owner.getRootNode() as ParentNode).querySelectorAll('[id]')].some(node => node !== this.owner && node.id === id);
            if (conflict) { desired.get(this.owner)?.delete('id'); triggerCode ??= 'interaction-name-duplicate'; closeCode ??= 'interaction-name-duplicate'; }
            else {
                if (source && !triggerCode) triggerCode = this.route(source, id, false, set);
                if (closeSource && !closeCode) closeCode = this.route(closeSource, id, true, set);
                if (source && !triggerCode) {
                    set(source, 'aria-controls', id); set(source, 'aria-haspopup', 'dialog'); set(source, 'aria-expanded', String(open));
                }
            }
        }
        this.triggerSource = enabled && !triggerCode ? source : undefined;
        this.closeSource = enabled && !closeCode ? closeSource : undefined;
        this.reconcile(desired);
        reportInteractionReference(this.host, enabled ? triggerCode : undefined, 'surface-trigger');
        reportInteractionReference(this.host, enabled ? closeCode : undefined, 'surface-close');
    }
    private route(source: HTMLElement, id: string, close: boolean, set: (node: HTMLElement, name: string, value: string) => void): string | undefined {
        const commandFor = this.original(source, 'commandfor'), popover = this.original(source, 'popovertarget');
        if (commandFor !== null && popover !== null) return 'interaction-dual-route';
        // Do not consume form submission/reset or navigation as a side effect of wiring.
        if (!(source instanceof HTMLButtonElement) || source.type !== 'button') return 'interaction-control-unsupported';
        if (commandFor !== null || popover !== null) {
            const target = commandFor ?? popover;
            const command = commandFor !== null ? this.original(source, 'command') : `${this.original(source, 'popovertargetaction') ?? 'toggle'}-popover`;
            const commands = close ? ['--cem-hide', 'request-close', 'hide-popover', 'toggle-popover'] : ['--cem-show', 'show-modal', 'show-popover', 'toggle-popover'];
            const customHost = command?.startsWith('--cem-') && target === this.host.id && this.host.id;
            if (target !== id && !customHost || !commands.includes(command ?? '')
                || command === 'show-modal' && (this.owner.hasAttribute('popover') || this.host.getAttribute('mode') === 'nonmodal' || this.host.getAttribute('presentation') === 'local')
                || command === 'request-close' && this.owner.hasAttribute('popover')
                || command?.endsWith('-popover') && !this.owner.hasAttribute('popover')) return 'interaction-command-incompatible';
            return;
        }
        const authored = this.original(source, 'command');
        if (authored !== null && authored !== (close ? '--cem-hide' : '--cem-show')) return 'interaction-command-incompatible';
        set(source, 'commandfor', id); set(source, 'command', close ? '--cem-hide' : '--cem-show');
        return;
    }
    disconnect(): void {
        this.reconcile(new Map()); this.trigger?.remove(); this.close?.remove(); this.trigger = this.close = undefined; this.triggerSource = this.closeSource = undefined;
    }
}
