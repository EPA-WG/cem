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
            '{button @type=button @class="cem-action cem-action--{$variant}" @disabled={datadom.attributes.disabled} @aria-busy={datadom.attributes.loading} @aria-expanded={datadom.attributes.expanded} @slice=pressed @slice-event=click @slice-value="$event.type" | {slot | {$label}}}',
    },
    {
        tag: 'cem-icon-button',
        description: 'Native icon-only button with a required accessible label.',
        cemMl:
            '{attribute @name=label | Icon action}' +
            '{attribute @name=name | circle}' +
            '{attribute @name=variant | quiet}' +
            '{button @type=button @class="cem-icon-button cem-icon-button--{$variant}" @aria-label="{$label}" @disabled={datadom.attributes.disabled} @aria-expanded={datadom.attributes.expanded} @slice=pressed @slice-event=click @slice-value="$event.type" |' +
            ' {span @class="cem-icon cem-icon--{$name}" @aria-hidden=true | {$name}}' +
            ' {slot}}',
    },
    {
        tag: 'cem-menu-item',
        description: 'Menu command row rendered as an accessible menuitem button.',
        cemMl:
            '{attribute @name=label | Menu item}' +
            '{button @type=button @role=menuitem @class=cem-menu-item @disabled={datadom.attributes.disabled} @aria-expanded={datadom.attributes.expanded} @slice=selected @slice-event=click @slice-value="$event.type" | {slot | {$label}}}',
    },
    {
        tag: 'cem-field',
        description: 'Labeled text input field with named label/help slots.',
        cemMl:
            '{attribute @name=label | Field}' +
            '{attribute @name=type | text}' +
            '{div @class=cem-field |' +
            ' {label @class=cem-field__label | {span | {slot @name=label | {$label}}} {input @class=cem-field__control @type="{$type}" @name="{$datadom.attributes.name}" @value={datadom.slices.value ?? datadom.attributes.value} @placeholder="{$datadom.attributes.placeholder}" @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @readonly={datadom.attributes.readonly} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} @slice=value @slice-event=input @slice-value="{$target.value}" | }}' +
            ' {span @class=cem-field__help | {slot @name=help}}}',
    },
    {
        tag: 'cem-text-field',
        description: 'MVP single-line text field with label and help slots.',
        cemMl:
            '{attribute @name=label | Text field}' +
            '{attribute @name=type | text}' +
            '{div @class=cem-text-field |' +
            ' {label @class=cem-text-field__label | {span | {slot @name=label | {$label}}} {input @class=cem-text-field__control @type="{$type}" @name="{$datadom.attributes.name}" @value={datadom.slices.value ?? datadom.attributes.value} @placeholder="{$datadom.attributes.placeholder}" @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @readonly={datadom.attributes.readonly} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} @slice=value @slice-event=input @slice-value="{$target.value}" | }}' +
            ' {span @class=cem-text-field__help | {slot @name=help}}}',
    },
    {
        tag: 'cem-textarea',
        description: 'MVP multi-line text field with label and help slots.',
        cemMl:
            '{attribute @name=label | Textarea}' +
            '{div @class=cem-textarea |' +
            ' {label @class=cem-textarea__label | {span | {slot @name=label | {$label}}} {textarea @class=cem-textarea__control @name="{$datadom.attributes.name}" @placeholder="{$datadom.attributes.placeholder}" @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @readonly={datadom.attributes.readonly} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} @slice=value @slice-event=input @slice-value="{$target.value}" | {$datadom.slices.value ?? datadom.attributes.value}}}' +
            ' {span @class=cem-textarea__help | {slot @name=help}}}',
    },
    {
        tag: 'cem-select',
        description: 'MVP native select field with projected option content.',
        cemMl:
            '{attribute @name=label | Select}' +
            '{div @class=cem-select |' +
            ' {label @class=cem-select__label | {span | {slot @name=label | {$label}}} {select @class=cem-select__control @name="{$datadom.attributes.name}" @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} @slice=value @slice-event=change @slice-value="{$target.value}" | {slot | {option @value="" | Choose}}}}' +
            ' {span @class=cem-select__help | {slot @name=help}}}',
    },
    {
        tag: 'cem-checkbox',
        description: 'MVP checkbox field with slotted label content.',
        cemMl:
            '{attribute @name=label | Checkbox}' +
            '{attribute @name=value | on}' +
            '{label @class=cem-checkbox |' +
            ' {input @class=cem-checkbox__control @type=checkbox @name="{$datadom.attributes.name}" @value="{$value}" @checked={datadom.slices.checked ?? datadom.attributes.checked} @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @aria-invalid={datadom.attributes.invalid} @aria-checked={datadom.attributes.indeterminate} @slice=checked @slice-event=change @slice-value="$target.checked" | }' +
            ' {span @class=cem-checkbox__label | {slot | {$label}}}}',
    },
    {
        tag: 'cem-radio',
        description: 'MVP radio field with slotted label content.',
        cemMl:
            '{attribute @name=label | Radio}' +
            '{attribute @name=value | on}' +
            '{label @class=cem-radio |' +
            ' {input @class=cem-radio__control @type=radio @name="{$datadom.attributes.name}" @value="{$value}" @checked={datadom.slices.checked ?? datadom.attributes.checked} @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @aria-invalid={datadom.attributes.invalid} @slice=checked @slice-event=change @slice-value="$target.checked" | }' +
            ' {span @class=cem-radio__label | {slot | {$label}}}}',
    },
    {
        tag: 'cem-switch',
        description: 'MVP switch field backed by a native checkbox control.',
        cemMl:
            '{attribute @name=label | Switch}' +
            '{attribute @name=value | on}' +
            '{label @class=cem-switch |' +
            ' {input @class=cem-switch__control @type=checkbox @role=switch @name="{$datadom.attributes.name}" @value="{$value}" @checked={datadom.slices.checked ?? datadom.attributes.checked} @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @aria-invalid={datadom.attributes.invalid} @slice=checked @slice-event=change @slice-value="$target.checked" | }' +
            ' {span @class=cem-switch__label | {slot | {$label}}}}',
    },
    {
        tag: 'cem-surface',
        description: 'Section surface for grouped content with an explicit empty workflow state.',
        cemMl:
            '{attribute @name=tone | default}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.empty" | {section @class="cem-surface cem-surface--{$tone}" @aria-label="{$datadom.attributes.label}" @data-state=empty | {slot}}}' +
            ' {cem:otherwise | {section @class="cem-surface cem-surface--{$tone}" @aria-label="{$datadom.attributes.label}" | {slot}}}}',
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
        description: 'Passive list container with an opt-in native single-select listbox mode.',
        cemMl:
            '{attribute @name=label | Items}' +
            '{attribute @name=size | 4}' +
            '{attribute @name=value | }' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.selectable" |' +
            '  {select @class="cem-list cem-list--selectable" @aria-label="{$label}" @size={$size} @slice=value @slice-event=change @slice-value="$target.value" |' +
            '   {cem:for-each @select="datadom.payload.elementsByAttribute.value" @as=option |' +
            '    {cem:if @test=\'option.tag == "cem-list-option" && !str:contains(option.key, "/")\' |' +
            '     {cem:choose |' +
            '      {cem:when @test=\'(option.attributes.value == (datadom.slices.value ?? value)) || ((datadom.slices.value ?? value) == null && option.key == datadom.payload.elementsByAttribute.selected.where(fn(candidate) => candidate.tag == "cem-list-option" && !str:contains(candidate.key, "/")).key.last())\' |' +
            '       {option @value="{$option.attributes.value}" @disabled={option.attributes.disabled != null} @selected=true @aria-selected=true | {$option.text}}}' +
            '      {cem:otherwise |' +
            '       {option @value="{$option.attributes.value}" @disabled={option.attributes.disabled != null} @aria-selected=false | {$option.text}}}}}}}}' +
            ' {cem:otherwise | {ul @class=cem-list @aria-label="{$label}" | {slot | {li @class=cem-list__empty | No items}}}}}',
    },
    {
        tag: 'cem-card',
        description: 'MVP card surface for profile, asset, and message summaries.',
        cemMl:
            '{attribute @name=label | Card}' +
            '{section @class=cem-card @aria-label="{$label}" |' +
            ' {header @class=cem-card__header | {slot @name=title | {$label}}}' +
            ' {div @class=cem-card__body | {slot}}}',
    },
    {
        tag: 'cem-table',
        description: 'MVP table wrapper for structured data comparison.',
        cemMl:
            '{attribute @name=label | Table}' +
            '{div @class=cem-table @role=table @aria-label="{$label}" | {slot | {div @role=row | {span @role=cell | No rows}}}}',
    },
    {
        tag: 'cem-chip',
        description: 'MVP compact label with an opt-in checked toggle mode.',
        cemMl:
            '{attribute @name=label | Chip}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.checkable" | {cem:choose |' +
            '  {cem:when @test="datadom.slices.checked ?? datadom.attributes.checked" | {button @type=button @class=cem-chip @aria-label="{$label}" @aria-pressed=true @slice=checked @slice-event=click @slice-value=false | {slot | {$label}}}}' +
            '  {cem:otherwise | {button @type=button @class=cem-chip @aria-label="{$label}" @aria-pressed=false @slice=checked @slice-event=click @slice-value=true | {slot | {$label}}}}}}' +
            ' {cem:otherwise | {span @class=cem-chip @aria-label="{$label}" | {slot | {$label}}}}}',
    },
    {
        tag: 'cem-badge',
        description: 'MVP status, count, priority, and severity label.',
        cemMl:
            '{attribute @name=label | Badge}' +
            '{attribute @name=tone | info}' +
            '{span @class="cem-badge cem-badge--{$tone}" @data-tone="{$tone}" | {slot | {$label}}}',
    },
    {
        tag: 'cem-avatar',
        description: 'MVP person or organization visual identity.',
        cemMl:
            '{attribute @name=label | Avatar}' +
            '{attribute @name=initials | }' +
            '{span @class=cem-avatar @role=img @aria-label="{$label}" | {slot | {$initials}}}',
    },
    {
        tag: 'cem-media-preview',
        description: 'MVP asset thumbnail, file, or object preview.',
        cemMl:
            '{attribute @name=label | Media preview}' +
            '{figure @class=cem-media-preview @aria-label="{$label}" |' +
            ' {div @class=cem-media-preview__media | {slot | {$label}}}' +
            ' {figcaption @class=cem-media-preview__caption | {slot @name=caption | {$label}}}}',
    },
    {
        tag: 'cem-app-bar',
        description: 'MVP app bar for product title, context, and global actions.',
        cemMl:
            '{attribute @name=label | Application}' +
            '{header @class=cem-app-bar @role=banner @aria-label="{$label}" |' +
            ' {div @class=cem-app-bar__title | {slot @name=title | {$label}}}' +
            ' {div @class=cem-app-bar__actions | {slot}}}',
    },
    {
        tag: 'cem-nav',
        description: 'Labeled navigation landmark with an opt-in disclosure mode.',
        cemMl:
            '{attribute @name=label | Navigation}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.collapsible" |' +
            '  {nav @class="cem-nav cem-nav--collapsible" @aria-label="{$label}" | {cem:choose |' +
            '   {cem:when @test="datadom.slices.expanded ?? datadom.attributes.expanded" |' +
            '    {button @type=button @class=cem-nav__disclosure @aria-expanded=true @slice=expanded @slice-event=click @slice-value=false | {$label}}' +
            '    {div @class=cem-nav__content | {slot}}}' +
            '   {cem:otherwise |' +
            '    {button @type=button @class=cem-nav__disclosure @aria-expanded=false @slice=expanded @slice-event=click @slice-value=true | {$label}}' +
            '    {div @class=cem-nav__content @hidden=true | {slot}}}}}}' +
            ' {cem:otherwise | {nav @class=cem-nav @aria-label="{$label}" | {slot}}}}',
    },
    {
        tag: 'cem-tabs',
        description: 'MVP tablist container for local view switching.',
        cemMl:
            '{attribute @name=label | Tabs}' +
            '{div @class=cem-tabs @role=tablist @aria-label="{$label}" | {slot | {button @type=button @role=tab @aria-selected=true | Tab}}}',
    },
    {
        tag: 'cem-dialog',
        description: 'MVP modal decision or focused task surface.',
        cemMl:
            '{attribute @name=label | Dialog}' +
            '{div @class=cem-dialog @role=dialog @aria-modal=true @aria-label="{$label}" | {slot}}',
    },
    {
        tag: 'cem-dialog-shell',
        description: 'Dialog shell with labeled light-DOM content.',
        cemMl:
            '{attribute @name=label | Dialog}' +
            '{div @class=cem-dialog-shell @role=dialog @aria-modal=true @aria-label="{$label}" | {slot}}',
    },
    {
        tag: 'cem-sheet',
        description: 'MVP non-modal or edge-attached task surface.',
        cemMl:
            '{attribute @name=label | Sheet}' +
            '{aside @class=cem-sheet @role=region @aria-label="{$label}" | {slot}}',
    },
    {
        tag: 'cem-toast',
        description: 'MVP transient status message.',
        cemMl:
            '{attribute @name=label | Status}' +
            '{div @class=cem-toast @role=status @aria-live=polite | {slot | {$label}}}',
    },
    {
        tag: 'cem-progress',
        description: 'MVP determinate or indeterminate progress indicator.',
        cemMl:
            '{attribute @name=label | Progress}' +
            '{attribute @name=max | 100}' +
            '{progress @class=cem-progress @aria-label="{$label}" @value={datadom.attributes.value} @max="{$max}" | {$label}}',
    },
    {
        tag: 'cem-skeleton',
        description: 'MVP loading placeholder that preserves layout.',
        cemMl:
            '{attribute @name=label | Loading}' +
            '{span @class=cem-skeleton @aria-hidden=true | {slot | {$label}}}',
    },
    {
        tag: 'cem-alert',
        description: 'MVP inline feedback message.',
        cemMl:
            '{attribute @name=label | Alert}' +
            '{attribute @name=tone | info}' +
            '{attribute @name=role | status}' +
            '{div @class="cem-alert cem-alert--{$tone}" @data-tone="{$tone}" @role="{$role}" | {slot | {$label}}}',
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
