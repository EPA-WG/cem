import type { CemElementDiagnostic, CemElementRuntime } from '@epa-wg/cem-elements';

export interface CemComponentPrimitiveDeclaration {
    readonly tag: string;
    readonly description: string;
    readonly cemMl: string;
}

export interface CemComponentPrimitiveInstallResult {
    readonly registered: string[];
    readonly skipped: string[];
    readonly diagnostics: CemElementDiagnostic[];
}

export const CEM_COMPONENT_PRIMITIVES = [
    {
        tag: 'cem-action',
        description: 'Native button action with slotted label content.',
        cemMl:
            '{attribute @name=label | Action}' +
            '{attribute @name=variant | primary}' +
            '{button @type=button @class="cem-action cem-action--{$variant}" | {slot | {$label}}}',
    },
    {
        tag: 'cem-field',
        description: 'Labeled text input field with named label/help slots.',
        cemMl:
            '{attribute @name=label | Field}' +
            '{attribute @name=type | text}' +
            '{div @class=cem-field |' +
            ' {label @class=cem-field__label | {span | {slot @name=label | {$label}}} {input @class=cem-field__control @type="{$type}" @name="{$datadom.attributes.name}" @value="{$datadom.attributes.value}" @placeholder="{$datadom.attributes.placeholder}" | }}' +
            ' {span @class=cem-field__help | {slot @name=help}}}',
    },
    {
        tag: 'cem-surface',
        description: 'Section surface for grouped content.',
        cemMl:
            '{attribute @name=tone | default}' +
            '{section @class="cem-surface cem-surface--{$tone}" @aria-label="{$datadom.attributes.label}" | {slot}}',
    },
    {
        tag: 'cem-text',
        description: 'Inline text primitive for token-scoped typography.',
        cemMl:
            '{attribute @name=variant | body}' +
            '{attribute @name=text | }' +
            '{span @class="cem-text cem-text--{$variant}" | {slot | {$text}}}',
    },
    {
        tag: 'cem-icon',
        description: 'Decorative or labeled icon text primitive.',
        cemMl:
            '{attribute @name=name | circle}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.label" | {span @class="cem-icon cem-icon--{$name}" @role=img @aria-label="{$datadom.attributes.label}" | {$name}}}' +
            ' {cem:otherwise | {span @class="cem-icon cem-icon--{$name}" @aria-hidden=true | {$name}}}}',
    },
    {
        tag: 'cem-stack',
        description: 'Single-axis layout primitive.',
        cemMl:
            '{attribute @name=gap | md}' +
            '{div @class="cem-stack cem-stack--{$gap}" @data-gap="{$gap}" | {slot}}',
    },
    {
        tag: 'cem-grid',
        description: 'Grid layout primitive.',
        cemMl:
            '{attribute @name=columns | auto}' +
            '{attribute @name=gap | md}' +
            '{div @class="cem-grid cem-grid--{$columns} cem-grid--gap-{$gap}" @data-columns="{$columns}" @data-gap="{$gap}" | {slot}}',
    },
    {
        tag: 'cem-list',
        description: 'List container with default empty-state fallback.',
        cemMl:
            '{attribute @name=label | Items}' +
            '{ul @class=cem-list @aria-label="{$label}" | {slot | {li @class=cem-list__empty | No items}}}',
    },
    {
        tag: 'cem-nav',
        description: 'Labeled navigation landmark.',
        cemMl:
            '{attribute @name=label | Navigation}' +
            '{nav @class=cem-nav @aria-label="{$label}" | {slot}}',
    },
    {
        tag: 'cem-dialog-shell',
        description: 'Dialog shell with labeled light-DOM content.',
        cemMl:
            '{attribute @name=label | Dialog}' +
            '{div @class=cem-dialog-shell @role=dialog @aria-modal=true @aria-label="{$label}" | {slot}}',
    },
] as const satisfies readonly CemComponentPrimitiveDeclaration[];

export function installCemComponentPrimitives(runtime: CemElementRuntime): CemComponentPrimitiveInstallResult {
    const registered: string[] = [];
    const skipped: string[] = [];
    const diagnostics: CemElementDiagnostic[] = [];

    for (const primitive of CEM_COMPONENT_PRIMITIVES) {
        const declaration = createPrimitiveDeclaration(primitive);
        const registry = declaration.ownerDocument.defaultView?.customElements;

        if (registry?.get(primitive.tag)) {
            skipped.push(primitive.tag);
            continue;
        }

        if (runtime.registerDeclaration(declaration)) {
            registered.push(primitive.tag);
        } else {
            diagnostics.push(...runtime.diagnosticsFor(declaration));
        }
    }

    return { registered, skipped, diagnostics };
}

export function createPrimitiveDeclaration(primitive: CemComponentPrimitiveDeclaration): HTMLElement {
    if (typeof document === 'undefined') {
        throw new Error('CEM component primitive declarations require a browser document');
    }

    const declaration = document.createElement('div');
    declaration.setAttribute('tag', primitive.tag);

    const template = document.createElement('template');
    template.setAttribute('type', 'text/cem-ml');
    template.textContent = primitive.cemMl;
    declaration.append(template);

    return declaration;
}
