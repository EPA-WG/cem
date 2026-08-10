import type { SerializedPayloadNode } from '@epa-wg/cem-elements';

export interface NormalizedChoiceOption {
    value: string;
    label: string;
    disabled: boolean;
    defaultSelected: boolean;
    children: SerializedPayloadNode[];
    rich: boolean;
    group: number;
}

export interface NormalizedChoiceGroup {
    label: string;
    disabled: boolean;
    options: NormalizedChoiceOption[];
}

export interface NormalizeChoiceOptionsResult {
    groups: NormalizedChoiceGroup[];
    options: NormalizedChoiceOption[];
    issue: string | null;
}

const INTERACTIVE_DESCENDANTS = new Set([
    'a',
    'audio',
    'button',
    'details',
    'embed',
    'iframe',
    'input',
    'object',
    'select',
    'summary',
    'textarea',
    'video',
]);

export function normalizeChoiceOptions(nodes: readonly SerializedPayloadNode[]): NormalizeChoiceOptionsResult {
    const authored = nodes.filter(
        (node): node is Extract<SerializedPayloadNode, { kind: 'element' }> =>
            node.kind === 'element' && node.attributes.slot !== 'label' && node.attributes.slot !== 'help',
    );
    const hasCanonical = authored.some((node) => node.tag === 'cem-option' || node.tag === 'cem-option-group');
    const hasLegacy = authored.some((node) => node.tag === 'option' || node.tag === 'optgroup');
    if (hasCanonical && hasLegacy) {
        return { groups: [], options: [], issue: 'Do not mix cem-option/cem-option-group with option/optgroup.' };
    }

    const canonical = hasCanonical;
    const groups: NormalizedChoiceGroup[] = [];
    let loose: NormalizedChoiceGroup | undefined;
    let issue: string | null = null;
    const seenValues = new Set<string>();

    const appendOption = (
        node: Extract<SerializedPayloadNode, { kind: 'element' }>,
        group: NormalizedChoiceGroup,
        inheritedDisabled: boolean,
    ) => {
        if (canonical && !Object.hasOwn(node.attributes, 'value')) {
            issue ??= 'Every cem-option requires an explicit value attribute; the invalid option was omitted.';
            return;
        }
        if (containsInteractiveContent(node.children)) {
            issue ??= 'Interactive descendants are not allowed inside cem-option; the invalid option was omitted.';
            return;
        }
        const label = node.attributes.label ?? collapseText(node.children);
        const value = node.attributes.value ?? label;
        if (seenValues.has(value)) {
            issue ??= `Duplicate option value "${value}" was omitted; values are component identities.`;
            return;
        }
        seenValues.add(value);
        const option: NormalizedChoiceOption = {
            value,
            label,
            disabled: inheritedDisabled || Object.hasOwn(node.attributes, 'disabled'),
            defaultSelected: Object.hasOwn(node.attributes, 'selected'),
            children: node.children,
            rich: hasRichContent(node.children),
            group: groups.indexOf(group),
        };
        group.options.push(option);
    };

    for (const node of authored) {
        const optionTag = canonical ? 'cem-option' : 'option';
        const groupTag = canonical ? 'cem-option-group' : 'optgroup';
        if (node.tag === optionTag) {
            loose ??= { label: '', disabled: false, options: [] };
            if (!groups.includes(loose)) groups.push(loose);
            appendOption(node, loose, false);
            continue;
        }
        if (node.tag !== groupTag) continue;
        const label = node.attributes.label ?? '';
        if (!label) issue ??= `${groupTag} requires a non-empty label attribute.`;
        const group: NormalizedChoiceGroup = {
            label,
            disabled: Object.hasOwn(node.attributes, 'disabled'),
            options: [],
        };
        groups.push(group);
        for (const child of node.children) {
            if (child.kind === 'element' && child.tag === optionTag) {
                appendOption(child, group, group.disabled);
            }
        }
        loose = undefined;
    }

    const options = groups.flatMap((group, groupIndex) =>
        group.options.map((option) => ({ ...option, group: groupIndex })),
    );
    return { groups, options, issue };
}

function containsInteractiveContent(nodes: readonly SerializedPayloadNode[]): boolean {
    return nodes.some((node) => {
        if (node.kind !== 'element') return false;
        if (
            INTERACTIVE_DESCENDANTS.has(node.tag)
            || Object.hasOwn(node.attributes, 'tabindex')
            || Object.hasOwn(node.attributes, 'contenteditable')
        ) {
            return true;
        }
        return containsInteractiveContent(node.children);
    });
}

function hasRichContent(nodes: readonly SerializedPayloadNode[]): boolean {
    return nodes.some((node) => node.kind === 'element' || node.kind === 'comment');
}

function collapseText(nodes: readonly SerializedPayloadNode[]): string {
    return nodes
        .map((node) => (node.kind === 'text' || node.kind === 'comment' ? node.text : collapseText(node.children)))
        .join(' ')
        .replace(/\s+/g, ' ')
        .trim();
}
