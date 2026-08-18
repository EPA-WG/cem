/** Browser assertions shared by the file-backed legacy and material inventories. */

const NATIVE_INTERACTIVE_SELECTOR = [
    'a[href]',
    'button',
    'input:not([type="hidden"])',
    'select',
    'textarea',
    'summary',
].join(',');

const REFERENCE_ATTRIBUTES = [
    'aria-labelledby',
    'aria-describedby',
    'aria-controls',
    'aria-owns',
    'aria-activedescendant',
    'for',
] as const;

const BOOLEAN_ARIA_ATTRIBUTES = [
    'aria-busy',
    'aria-disabled',
    'aria-expanded',
    'aria-hidden',
    'aria-invalid',
    'aria-pressed',
    'aria-selected',
] as const;

export interface Phase3AccessibilityAudit {
    interactiveCount: number;
    namedInteractiveCount: number;
    imageCount: number;
    referenceCount: number;
}

/**
 * Enforce the runtime subset of the binding Phase 3 accessibility contract on
 * rendered light DOM. This intentionally uses element capabilities/local names
 * instead of global constructors so it works for same-origin iframe fixtures.
 */
export function assertPhase3Accessibility(
    roots: readonly ParentNode[],
    label: string
): Phase3AccessibilityAudit {
    const audit: Phase3AccessibilityAudit = {
        interactiveCount: 0,
        namedInteractiveCount: 0,
        imageCount: 0,
        referenceCount: 0,
    };
    const seenIds = new Set<string>();

    for (const root of roots) {
        assertUniqueIds(root, label, seenIds);
        assertReferenceIntegrity(root, label, audit);
        assertAriaValues(root, label);
        assertImages(root, label, audit);

        const interactive = matchingElements(root, NATIVE_INTERACTIVE_SELECTOR) as HTMLElement[];
        audit.interactiveCount += interactive.length;
        for (const element of interactive) {
            assert(
                !element.hasAttribute('role'),
                `${label}: native <${element.localName}> must retain its implicit role`
            );
            const name = accessibleName(element);
            assert(name.length > 0, `${label}: interactive <${element.localName}> requires an accessible name`);
            audit.namedInteractiveCount += 1;
            if (!isDisabled(element)) {
                assert(element.tabIndex >= 0, `${label}: enabled <${element.localName}> must remain keyboard focusable`);
                element.focus();
                assert(
                    element.ownerDocument.activeElement === element,
                    `${label}: enabled <${element.localName}> must accept document focus`
                );
            }
        }

        const customHosts = [
            ...(root.nodeType === 1 ? [root as Element] : []),
            ...Array.from(root.querySelectorAll('*')),
        ].filter((element) => element.localName.includes('-'));
        for (const host of customHosts) {
            if (!host.querySelector(NATIVE_INTERACTIVE_SELECTOR)) continue;
            assert(
                !host.hasAttribute('tabindex'),
                `${label}: <${host.localName}> with native interactive descendants must not add a duplicate tab stop`
            );
        }
    }

    assertEqual(
        audit.namedInteractiveCount,
        audit.interactiveCount,
        `${label}: every native interactive element is named`
    );
    return audit;
}

export function accessibleName(element: Element): string {
    const labelledBy = element.getAttribute('aria-labelledby')?.trim();
    if (labelledBy) {
        return labelledBy
            .split(/\s+/)
            .map((id) => visibleText(element.ownerDocument.getElementById(id)))
            .filter(Boolean)
            .join(' ')
            .trim();
    }

    const ariaLabel = element.getAttribute('aria-label')?.trim();
    if (ariaLabel) return ariaLabel;

    const labels = 'labels' in element
        ? Array.from((element as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement).labels ?? [])
        : [];
    if (labels.length > 0) {
        return labels.map((label) => labelText(label, element)).filter(Boolean).join(' ').trim();
    }
    const wrappingLabel = element.closest('label');
    if (wrappingLabel) return labelText(wrappingLabel, element);

    if (element.localName === 'img' || inputType(element) === 'image') {
        return element.getAttribute('alt')?.trim() ?? '';
    }
    if (element.localName === 'input') {
        const type = inputType(element);
        if (type === 'button' || type === 'submit' || type === 'reset') {
            return element.getAttribute('value')?.trim() ?? '';
        }
        return element.getAttribute('placeholder')?.trim() ?? '';
    }

    return visibleText(element) || element.getAttribute('title')?.trim() || '';
}

function assertUniqueIds(root: ParentNode, label: string, seen: Set<string>): void {
    for (const element of matchingElements(root, '[id]')) {
        const id = element.id;
        assert(id.length > 0, `${label}: rendered id must not be empty`);
        assert(!seen.has(id), `${label}: rendered id ${id} must be unique within the fixture side`);
        seen.add(id);
    }
}

function assertReferenceIntegrity(
    root: ParentNode,
    label: string,
    audit: Phase3AccessibilityAudit
): void {
    const selector = REFERENCE_ATTRIBUTES.map((attribute) => `[${attribute}]`).join(',');
    for (const element of matchingElements(root, selector)) {
        for (const attribute of REFERENCE_ATTRIBUTES) {
            const value = element.getAttribute(attribute)?.trim();
            if (!value) continue;
            for (const id of value.split(/\s+/)) {
                assert(
                    element.ownerDocument.getElementById(id) !== null,
                    `${label}: ${attribute} reference ${id} must resolve in the owning document`
                );
                audit.referenceCount += 1;
            }
        }
    }
}

function assertAriaValues(root: ParentNode, label: string): void {
    for (const attribute of BOOLEAN_ARIA_ATTRIBUTES) {
        for (const element of matchingElements(root, `[${attribute}]`)) {
            const value = element.getAttribute(attribute);
            assert(
                value === 'true' || value === 'false',
                `${label}: ${attribute} on <${element.localName}> must be "true" or "false"`
            );
        }
    }
    for (const element of matchingElements(root, '[aria-current]')) {
        const value = element.getAttribute('aria-current');
        assert(
            value === 'page' ||
                value === 'step' ||
                value === 'location' ||
                value === 'date' ||
                value === 'time' ||
                value === 'true' ||
                value === 'false',
            `${label}: aria-current on <${element.localName}> has an unsupported value`
        );
    }
}

function assertImages(root: ParentNode, label: string, audit: Phase3AccessibilityAudit): void {
    for (const image of matchingElements(root, 'img')) {
        audit.imageCount += 1;
        assert(image.hasAttribute('alt'), `${label}: rendered images require an explicit alt attribute`);
        if (image.getAttribute('alt') === '') {
            assert(image.getAttribute('tabindex') !== '0', `${label}: decorative images must not enter the tab order`);
            assert(!image.hasAttribute('role'), `${label}: decorative images must not add a competing role`);
        }
    }
}

function matchingElements(root: ParentNode, selector: string): Element[] {
    const descendants = Array.from(root.querySelectorAll(selector));
    return root.nodeType === 1 && (root as Element).matches(selector)
        ? [root as Element, ...descendants]
        : descendants;
}

function labelText(label: HTMLLabelElement, control: Element): string {
    const clone = label.cloneNode(true) as HTMLLabelElement;
    for (const nested of Array.from(clone.querySelectorAll('input,select,textarea,button'))) {
        nested.remove();
    }
    const text = visibleText(clone);
    if (text) return text;
    return control.getAttribute('placeholder')?.trim() ?? '';
}

function visibleText(element: Element | null): string {
    return (element?.textContent ?? '').replace(/\s+/g, ' ').trim();
}

function inputType(element: Element): string {
    return element.localName === 'input'
        ? (element.getAttribute('type') ?? 'text').toLowerCase()
        : '';
}

function isDisabled(element: HTMLElement): boolean {
    return element.hasAttribute('disabled') || element.getAttribute('aria-disabled') === 'true';
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, message: string): void {
    if (actual !== expected) {
        throw new Error(`${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
}
