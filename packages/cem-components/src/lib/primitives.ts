import type {
    CemElementDiagnostic,
    CemElementRuntime,
    CemProducedElementBehavior,
} from '@epa-wg/cem-elements';
import { CEM_AUTOCOMPLETE_BEHAVIOR } from './autocomplete-behavior.js';
import { CEM_DATEPICKER_BEHAVIOR } from './datepicker-behavior.js';
import { CEM_EXPANSION_BEHAVIOR } from './expansion-behavior.js';
import { CEM_FEEDBACK_DIALOG_BEHAVIOR } from './feedback-behavior.js';
import { CEM_NAVIGATION_BEHAVIOR } from './navigation-behavior.js';
import { CEM_PAGINATOR_BEHAVIOR } from './paginator-behavior.js';
import { CEM_PROGRESS_SPINNER_BEHAVIOR } from './progress-spinner-behavior.js';
import { CEM_SELECT_BEHAVIOR } from './select-behavior.js';
import { CEM_SLIDER_BEHAVIOR } from './slider-behavior.js';
import { CEM_SORT_HEADER_BEHAVIOR } from './sort-header-behavior.js';
import { CEM_STEPPER_BEHAVIOR } from './stepper-behavior.js';
import { CEM_TIMEPICKER_BEHAVIOR } from './timepicker-behavior.js';
import { CEM_TOOLTIP_BEHAVIOR } from './tooltip-behavior.js';

export interface CemComponentPrimitiveDeclaration {
    readonly tag: string;
    readonly description: string;
    readonly cemMl: string;
    readonly behavior?: CemProducedElementBehavior;
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
            '{attribute @name=indicator | underline}' +
            '{div @class=cem-field |' +
            ' {label @class=cem-field__label | {span | {slot @name=label | {$label}}} {input @class=cem-field__control @type="{$type}" @name="{$datadom.attributes.name}" @value={datadom.slices.value ?? datadom.attributes.value} @placeholder="{$datadom.attributes.placeholder}" @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @readonly={datadom.attributes.readonly} @data-state={if datadom.attributes.busy { "loading" } else { null }} @aria-busy={if datadom.attributes.busy { true } else { null }} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} @slice=value @slice-event=input @slice-value="{$target.value}" | }}' +
            ' {span @class=cem-field__help | {slot @name=help}}}',
    },
    {
        tag: 'cem-text-field',
        description: 'MVP single-line text field with label and help slots.',
        cemMl:
            '{attribute @name=label | Text field}' +
            '{attribute @name=type | text}' +
            '{attribute @name=indicator | underline}' +
            '{div @class=cem-text-field |' +
            ' {label @class=cem-text-field__label | {span | {slot @name=label | {$label}}} {input @class=cem-text-field__control @type="{$type}" @name="{$datadom.attributes.name}" @value={datadom.slices.value ?? datadom.attributes.value} @placeholder="{$datadom.attributes.placeholder}" @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @readonly={datadom.attributes.readonly} @data-state={if datadom.attributes.busy { "loading" } else { null }} @aria-busy={if datadom.attributes.busy { true } else { null }} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} @slice=value @slice-event=input @slice-value="{$target.value}" | }}' +
            ' {span @class=cem-text-field__help | {slot @name=help}}}',
    },
    {
        tag: 'cem-textarea',
        description: 'MVP multi-line text field with label and help slots.',
        cemMl:
            '{attribute @name=label | Textarea}' +
            '{attribute @name=indicator | underline}' +
            '{div @class=cem-textarea |' +
            ' {label @class=cem-textarea__label | {span | {slot @name=label | {$label}}} {textarea @class=cem-textarea__control @name="{$datadom.attributes.name}" @placeholder="{$datadom.attributes.placeholder}" @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @readonly={datadom.attributes.readonly} @data-state={if datadom.attributes.busy { "loading" } else { null }} @aria-busy={if datadom.attributes.busy { true } else { null }} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} @slice=value @slice-event=input @slice-value="{$target.value}" | {$datadom.slices.value ?? datadom.attributes.value}}}' +
            ' {span @class=cem-textarea__help | {slot @name=help}}}',
    },
    {
        tag: 'cem-autocomplete',
        description: 'Form-associated editable combobox with declarative suggestions.',
        behavior: CEM_AUTOCOMPLETE_BEHAVIOR,
        cemMl:
            '{module |' +
            ' {attribute @name=label | Autocomplete}' +
            ' {attribute @name=indicator | underline}' +
            ' {slice @name=groups}' +
            ' {slice @name=displayValue | }' +
            ' {slice @name=expanded | false}' +
            ' {template @name=autocomplete-option |' +
            '  {param @name=option}' +
            '  {body |' +
            '   {div @id="{$option.id}" @class=cem-autocomplete__option @role=option @data-option-index="{$option.index}" @data-active="{$option.active}" @aria-selected="{$option.selected}" @aria-disabled="{$option.disabled}" |' +
            '    {cem:choose |' +
            '     {cem:when @test="option.hasChildren" | {cem:project-payload @select="option.children" | }}' +
            '     {cem:otherwise | {$option.label}}}}}}}' +
            ' {template @name=autocomplete-options |' +
            '  {param @name=groups}' +
            '  {body |' +
            '   {cem:for-each @select="groups" @as=group |' +
            '    {cem:choose |' +
            '     {cem:when @test="group.label" |' +
            '      {div @class=cem-autocomplete__group @role=group @aria-label="{$group.label}" @aria-disabled="{$group.disabled}" |' +
            '       {div @class=cem-autocomplete__group-label @aria-hidden=true | {$group.label}}' +
            '       {cem:for-each @select="group.options" @as=option | {call @template=autocomplete-option @with:option="{$option}"}}}}' +
            '     {cem:otherwise |' +
            '      {cem:for-each @select="group.options" @as=option | {call @template=autocomplete-option @with:option="{$option}"}}}}}}}' +
            ' {body |' +
            '  {div @class=cem-autocomplete |' +
            '   {span @id="{$datadom.slices.labelId}" @class=cem-autocomplete__label | {slot @name=label | {$label}}}' +
            '   {input @type=text @class=cem-autocomplete__control @role=combobox @value="{$datadom.slices.displayValue}" @placeholder="{$datadom.attributes.placeholder}" @autocomplete="{$datadom.attributes.autocomplete}" @aria-labelledby="{$datadom.slices.labelId}" @aria-autocomplete=list @aria-haspopup=listbox @aria-expanded="{$datadom.slices.expanded}" @aria-controls={if datadom.slices.expanded { datadom.slices.listboxId } else { null }} @aria-activedescendant={if datadom.slices.expanded && datadom.slices.activeOptionId { datadom.slices.activeOptionId } else { null }} @disabled={if datadom.attributes.disabled || datadom.slices.behaviorDisabled { true } else { null }} @readonly={datadom.attributes.readonly} @required={datadom.attributes.required} @data-state={if datadom.attributes.busy { "loading" } else { null }} @aria-busy={if datadom.attributes.busy { true } else { null }} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} | }' +
            '   {cem:if @test="datadom.slices.expanded" |' +
            '    {div @id="{$datadom.slices.listboxId}" @class=cem-autocomplete__popup @role=listbox @aria-labelledby="{$datadom.slices.labelId}" |' +
            '     {call @template=autocomplete-options @with:groups="{$datadom.slices.groups}"}}}' +
            '   {span @class=cem-autocomplete__help | {slot @name=help}}}}}',
    },
    {
        tag: 'cem-timepicker',
        description: 'Time-of-day picker retaining one authored native text-input form owner.',
        behavior: CEM_TIMEPICKER_BEHAVIOR,
        cemMl:
            '{module |' +
            ' {slice @name=options}' +
            ' {slice @name=expanded | false}' +
            ' {template @name=timepicker-option |' +
            '  {param @name=option}' +
            '  {body |' +
            '   {div @id="{$option.id}" @class=cem-timepicker__option @role=option @data-option-index="{$option.index}" @data-value="{$option.value}" @data-active="{$option.active}" @aria-selected="{$option.selected}" @aria-disabled="{$option.disabled}" | {$option.label}}}}' +
            ' {body |' +
            '  {span @class=cem-timepicker @data-mode="{$datadom.slices.mode}" |' +
            '   {slot @name=input}' +
            '   {slot @name=toggle}' +
            '   {div @id="{$datadom.slices.listboxId}" @class=cem-timepicker__popup @role=listbox @popover=manual |' +
            '    {cem:for-each @select=datadom.slices.options @as=option | {call @template=timepicker-option @with:option="{$option}"}}}}}}',
    },
    {
        tag: 'cem-datepicker',
        description: 'Single-date calendar retaining one authored native text-input form owner.',
        behavior: CEM_DATEPICKER_BEHAVIOR,
        cemMl:
            '{module |' +
            ' {slice @name=weekdays}' +
            ' {slice @name=weeks}' +
            ' {slice @name=expanded | false}' +
            ' {template @name=datepicker-day |' +
            '  {param @name=day}' +
            '  {body |' +
            '   {cem:choose |' +
            '    {cem:when @test="day.value" |' +
            '     {button @id="{$day.id}" @type=button @class=cem-datepicker__day @role=gridcell @data-date="{$day.value}" @data-active="{$day.active}" @data-outside="{$day.outside}" @tabindex="{$day.tabIndex}" @aria-label="{$day.fullLabel}" @aria-selected="{$day.selected}" @aria-current={if day.current { "date" } else { null }} @aria-disabled="{$day.disabled}" @disabled={if day.disabled { true } else { null }} | {$day.number}}}' +
            '    {cem:otherwise | {span @id="{$day.id}" @class=cem-datepicker__day @role=gridcell @aria-disabled=true | }}}}}}' +
            ' {template @name=datepicker-week |' +
            '  {param @name=week}' +
            '  {body | {div @class=cem-datepicker__week @role=row |' +
            '   {cem:for-each @select=week.days @as=day | {call @template=datepicker-day @with:day="{$day}"}}}}}' +
            ' {body |' +
            '  {span @class=cem-datepicker @data-mode="{$datadom.slices.mode}" |' +
            '   {slot @name=input}' +
            '   {slot @name=toggle}' +
            '   {dialog @id="{$datadom.slices.dialogId}" @class=cem-datepicker__dialog @aria-labelledby="{$datadom.slices.headingId}" |' +
            '    {div @class=cem-datepicker__header |' +
            '     {button @type=button @class=cem-datepicker__action @data-datepicker-action=previous @aria-label="Previous month" @disabled={if datadom.slices.previousDisabled { true } else { null }} | Previous}' +
            '     {h2 @id="{$datadom.slices.headingId}" @class=cem-datepicker__heading @aria-live=polite | {$datadom.slices.heading}}' +
            '     {button @type=button @class=cem-datepicker__action @data-datepicker-action=next @aria-label="Next month" @disabled={if datadom.slices.nextDisabled { true } else { null }} | Next}}' +
            '    {div @class=cem-datepicker__grid @role=grid @aria-labelledby="{$datadom.slices.headingId}" |' +
            '     {div @class=cem-datepicker__weekdays @role=row |' +
            '      {cem:for-each @select=datadom.slices.weekdays @as=weekday |' +
            '       {span @class=cem-datepicker__weekday @role=columnheader @aria-label="{$weekday.full}" | {$weekday.short}}}}' +
            '     {cem:for-each @select=datadom.slices.weeks @as=week | {call @template=datepicker-week @with:week="{$week}"}}}' +
            '    {div @class=cem-datepicker__actions |' +
            '     {button @type=button @class=cem-datepicker__action @data-datepicker-action=cancel | Cancel}' +
            '     {button @type=button @class=cem-datepicker__action @data-datepicker-action=apply @disabled={if datadom.slices.applyDisabled { true } else { null }} | Apply}}}}}}',
    },
    {
        tag: 'cem-select',
        description: 'Form-associated custom select with rich cem-option content.',
        behavior: CEM_SELECT_BEHAVIOR,
        cemMl:
            '{module |' +
            ' {attribute @name=label | Select}' +
            ' {attribute @name=indicator | underline}' +
            ' {slice @name=groups}' +
            ' {slice @name=value | }' +
            ' {slice @name=selectedValues}' +
            ' {slice @name=expanded | false}' +
            ' {slice @name=mode | dropdown}' +
            ' {template @name=select-option |' +
            '  {param @name=option}' +
            '  {body |' +
            '   {div @id="{$option.id}" @class=cem-select__option @role=option @data-option-index="{$option.index}" @data-active="{$option.active}" @aria-selected="{$option.selected}" @aria-disabled="{$option.disabled}" |' +
            '    {cem:choose |' +
            '     {cem:when @test="option.rich" | {cem:project-payload @select="option.children" | }}' +
            '     {cem:otherwise | {$option.label}}}}}}}' +
            ' {template @name=select-options |' +
            '  {param @name=groups}' +
            '  {body |' +
            '   {cem:for-each @select="groups" @as=group |' +
            '    {cem:choose |' +
            '     {cem:when @test="group.label" |' +
            '      {div @class=cem-select__group @role=group @aria-label="{$group.label}" @aria-disabled="{$group.disabled}" |' +
            '       {div @class=cem-select__group-label @aria-hidden=true | {$group.label}}' +
            '       {cem:for-each @select="group.options" @as=option | {call @template=select-option @with:option="{$option}"}}}}' +
            '     {cem:otherwise |' +
            '      {cem:for-each @select="group.options" @as=option | {call @template=select-option @with:option="{$option}"}}}}}}}' +
            ' {body |' +
            '  {div @class=cem-select |' +
            '   {span @id="{$datadom.slices.labelId}" @class=cem-select__label | {slot @name=label | {$label}}}' +
            '   {cem:choose |' +
            '    {cem:when @test=\'datadom.slices.mode == "dropdown"\' |' +
            '     {button @type=button @class=cem-select__control @role=combobox @value="{$datadom.slices.value}" @aria-labelledby="{$datadom.slices.labelId}" @aria-haspopup=listbox @aria-expanded="{$datadom.slices.expanded}" @aria-controls={if datadom.slices.expanded { datadom.slices.popupId } else { null }} @aria-activedescendant={if datadom.slices.expanded { datadom.slices.activeOptionId } else { null }} @disabled={if datadom.attributes.disabled || datadom.slices.behaviorDisabled { true } else { null }} @data-state={if datadom.attributes.busy { "loading" } else { null }} @aria-busy={if datadom.attributes.busy { true } else { null }} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} |' +
            '      {span @class=cem-select__value |' +
            '       {cem:choose |' +
            '        {cem:when @test="datadom.slices.selectedRich" | {cem:project-payload @select="datadom.slices.selectedChildren" | }}' +
            '        {cem:otherwise | {$datadom.slices.displayLabel}}}}' +
            '      {span @class=cem-select__marker @aria-hidden=true | ▾}}' +
            '     {cem:if @test="datadom.slices.expanded" |' +
            '      {div @id="{$datadom.slices.popupId}" @class=cem-select__popup @role=listbox @aria-labelledby="{$datadom.slices.labelId}" |' +
            '       {call @template=select-options @with:groups="{$datadom.slices.groups}"}}}}' +
            '    {cem:otherwise |' +
            '     {div @id="{$datadom.slices.listboxId}" @class="cem-select__control cem-select__listbox" @role=listbox @tabindex={if datadom.attributes.disabled || datadom.slices.behaviorDisabled { -1 } else { 0 }} @aria-labelledby="{$datadom.slices.labelId}" @aria-multiselectable={if datadom.slices.mode == "multiple-listbox" { true } else { null }} @aria-activedescendant="{$datadom.slices.activeOptionId}" @aria-disabled="{$datadom.attributes.disabled || datadom.slices.behaviorDisabled}" @data-visible-rows="{$datadom.slices.visibleRows}" @data-state={if datadom.attributes.busy { "loading" } else { null }} @aria-busy={if datadom.attributes.busy { true } else { null }} @aria-invalid={datadom.attributes.invalid} @aria-describedby={datadom.attributes.describedby} @aria-errormessage={datadom.attributes.error} |' +
            '      {call @template=select-options @with:groups="{$datadom.slices.groups}"}}}}' +
            '   {span @class=cem-select__help | {slot @name=help}}}}}',
    },
    {
        tag: 'cem-option',
        description: 'Canonical rich-content option consumed by cem-select, cem-autocomplete, and cem-timepicker.',
        cemMl: '{span @class=cem-option | {slot}}',
    },
    {
        tag: 'cem-option-group',
        description: 'Labeled canonical option group consumed by cem-select and cem-autocomplete.',
        cemMl: '{div @class=cem-option-group | {slot}}',
    },
    {
        tag: 'cem-checkbox',
        description: 'MVP checkbox field with slotted label content.',
        cemMl:
            '{attribute @name=label | Checkbox}' +
            '{attribute @name=value | on}' +
            '{attribute @name=indicator | outline}' +
            '{label @class=cem-checkbox |' +
            ' {input @class=cem-checkbox__control @type=checkbox @name="{$datadom.attributes.name}" @value="{$value}" @checked={datadom.slices.checked ?? datadom.attributes.checked} @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @data-state={if datadom.attributes.busy { "loading" } else { null }} @aria-busy={if datadom.attributes.busy { true } else { null }} @aria-invalid={datadom.attributes.invalid} @aria-checked={datadom.attributes.indeterminate} @slice=checked @slice-event=change @slice-value="$target.checked" | }' +
            ' {span @class=cem-checkbox__label | {slot | {$label}}}}',
    },
    {
        tag: 'cem-radio',
        description: 'MVP radio field with slotted label content.',
        cemMl:
            '{attribute @name=label | Radio}' +
            '{attribute @name=value | on}' +
            '{attribute @name=indicator | outline}' +
            '{label @class=cem-radio |' +
            ' {input @class=cem-radio__control @type=radio @name="{$datadom.attributes.name}" @value="{$value}" @checked={datadom.slices.checked ?? datadom.attributes.checked} @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @data-state={if datadom.attributes.busy { "loading" } else { null }} @aria-busy={if datadom.attributes.busy { true } else { null }} @aria-invalid={datadom.attributes.invalid} @slice=checked @slice-event=change @slice-value="$target.checked" | }' +
            ' {span @class=cem-radio__label | {slot | {$label}}}}',
    },
    {
        tag: 'cem-switch',
        description: 'MVP switch field backed by a native checkbox control.',
        cemMl:
            '{attribute @name=label | Switch}' +
            '{attribute @name=value | on}' +
            '{attribute @name=indicator | outline}' +
            '{label @class=cem-switch |' +
            ' {input @class=cem-switch__control @type=checkbox @role=switch @name="{$datadom.attributes.name}" @value="{$value}" @checked={datadom.slices.checked ?? datadom.attributes.checked} @disabled={datadom.attributes.disabled} @required={datadom.attributes.required} @data-state={if datadom.attributes.busy { "loading" } else { null }} @aria-busy={if datadom.attributes.busy { true } else { null }} @aria-invalid={datadom.attributes.invalid} @slice=checked @slice-event=change @slice-value="$target.checked" | }' +
            ' {span @class=cem-switch__label | {slot | {$label}}}}',
    },
    {
        tag: 'cem-slider',
        description: 'Single-value or range slider retaining native range-input and form ownership.',
        behavior: CEM_SLIDER_BEHAVIOR,
        cemMl:
            '{attribute @name=min | 0}' +
            '{attribute @name=max | 100}' +
            '{attribute @name=step | 1}' +
            '{div @class=cem-slider @data-mode="{$datadom.slices.mode}" |' +
            ' {div @class=cem-slider__visual @aria-hidden=true |' +
            '  {span @class=cem-slider__track | }' +
            '  {span @class=cem-slider__active-track | }' +
            '  {span @class=cem-slider__ticks | }' +
            '  {span @class=cem-slider__value @data-cem-slider-value=single | }' +
            '  {span @class=cem-slider__value @data-cem-slider-value=start | }' +
            '  {span @class=cem-slider__value @data-cem-slider-value=end | }}' +
            ' {div @class=cem-slider__inputs | {slot}}}',
    },
    {
        tag: 'cem-surface',
        description: 'Section surface for grouped content with explicit busy and empty workflow states.',
        cemMl:
            '{attribute @name=tone | default}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.busy" | {section @class="cem-surface cem-surface--{$tone}" @aria-label="{$datadom.attributes.label}" @data-state=loading @aria-busy=true | {slot}}}' +
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
        tag: 'cem-divider',
        description: 'Semantic or decorative sibling-separation track with horizontal, vertical, and inset forms.',
        cemMl:
            '{attribute @name=orientation | horizontal}' +
            '{attribute @name=spacing | group}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.decorative" | {cem:choose |' +
            '  {cem:when @test=\'datadom.attributes.orientation == "vertical"\' | {div @class=cem-divider @data-orientation=vertical @aria-hidden=true | }}' +
            '  {cem:otherwise | {div @class=cem-divider @data-orientation=horizontal @aria-hidden=true | }}}}' +
            ' {cem:otherwise | {cem:choose |' +
            '  {cem:when @test=\'datadom.attributes.orientation == "vertical"\' | {div @class=cem-divider @data-orientation=vertical @role=separator @aria-orientation=vertical | }}' +
            '  {cem:otherwise | {div @class=cem-divider @data-orientation=horizontal @role=separator @aria-orientation=horizontal | }}}}}',
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
        description: 'MVP card surface for profile, asset, and message summaries with explicit busy content state.',
        cemMl:
            '{attribute @name=label | Card}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.busy" | {section @class=cem-card @aria-label="{$label}" @data-state=loading @aria-busy=true |' +
            '  {header @class=cem-card__header | {slot @name=title | {$label}}}' +
            '  {div @class=cem-card__body | {slot}}}}' +
            ' {cem:otherwise | {section @class=cem-card @aria-label="{$label}" |' +
            '  {header @class=cem-card__header | {slot @name=title | {$label}}}' +
            '  {div @class=cem-card__body | {slot}}}}}',
    },
    {
        tag: 'cem-expansion',
        description: 'Independent general-purpose disclosure panel with a native header button.',
        behavior: CEM_EXPANSION_BEHAVIOR,
        cemMl:
            '{attribute @name=label | Expansion}' +
            '{div @class=cem-expansion |' +
            ' {div @class=cem-expansion__heading @role=heading @aria-level="{$datadom.slices.headingLevel}" |' +
            '  {button @id="{$datadom.slices.headingId}" @type=button @class=cem-expansion__header @disabled={datadom.attributes.disabled} @aria-labelledby="{$datadom.slices.summaryId}" @aria-expanded={if datadom.attributes.expanded { true } else { false }} @aria-controls="{$datadom.slices.panelId}" |' +
            '   {span @id="{$datadom.slices.summaryId}" @class=cem-expansion__summary | {slot @name=summary | {$label}}}' +
            '   {span @class=cem-expansion__indicator @aria-hidden=true | {cem:choose |' +
            '    {cem:when @test="datadom.attributes.expanded" | ▾}' +
            '    {cem:otherwise | ▸}}}}}' +
            ' {div @id="{$datadom.slices.panelId}" @class=cem-expansion__panel @role={if datadom.attributes.region { "region" } else { null }} @aria-labelledby="{$datadom.slices.headingId}" @hidden={if datadom.attributes.expanded { null } else { true }} | {slot}}}',
    },
    {
        tag: 'cem-table',
        description: 'MVP table wrapper for structured data comparison.',
        cemMl:
            '{attribute @name=label | Table}' +
            '{div @class=cem-table @role=table @aria-label="{$label}" | {slot | {div @role=row | {span @role=cell | No rows}}}}',
    },
    {
        tag: 'cem-sort-header',
        description: 'Sortable column header with native button activation and external data ownership.',
        behavior: CEM_SORT_HEADER_BEHAVIOR,
        cemMl:
            '{attribute @name=label | Column}' +
            '{div @class=cem-sort-header @role=columnheader @aria-sort={if datadom.slices.direction == "none" { null } else { datadom.slices.direction }} |' +
            ' {button @type=button @class=cem-sort-header__button @disabled={datadom.attributes.disabled} @aria-label="{$datadom.slices.actionLabel}" |' +
            '  {span @class=cem-sort-header__label | {$label}}' +
            '  {span @class=cem-sort-header__indicator @aria-hidden=true | {$datadom.slices.indicator}}}}',
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
        behavior: CEM_NAVIGATION_BEHAVIOR,
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
        behavior: CEM_NAVIGATION_BEHAVIOR,
        cemMl:
            '{attribute @name=label | Tabs}' +
            '{div @class=cem-tabs @role=tablist @aria-label="{$label}" | {slot | {button @type=button @role=tab @aria-selected=true | Tab}}}',
    },
    {
        tag: 'cem-stepper',
        description: 'Labeled linear or nonlinear workflow navigation with persistent panels.',
        behavior: CEM_STEPPER_BEHAVIOR,
        cemMl:
            '{module |' +
            ' {attribute @name=label | Steps}' +
            ' {slice @name=steps}' +
            ' {template @name=stepper-header |' +
            '  {param @name=step}' +
            '  {body |' +
            '   {li @class=cem-stepper__item @data-completed="{$step.connectorCompleted}" |' +
            '    {button @id="{$step.buttonId}" @type=button @class=cem-stepper__header @data-step-index="{$step.index}" @data-marker-state="{$step.markerState}" @tabindex="{$step.tabIndex}" @disabled={if step.disabled { true } else { null }} @aria-disabled={if step.unavailable { true } else { null }} @aria-current={if step.current { "step" } else { null }} @aria-invalid={if step.invalid { true } else { null }} @aria-controls="{$step.panelId}" |' +
            '     {span @class=cem-stepper__marker @aria-hidden=true | {$step.marker}}' +
            '     {span @class=cem-stepper__label | {$step.label}}' +
            '     {cem:if @test=step.status | {span @class=cem-stepper__status | {$step.status}}}}}}}' +
            ' {template @name=stepper-panel |' +
            '  {param @name=step}' +
            '  {body |' +
            '   {div @id="{$step.panelId}" @class=cem-stepper__panel @role=region @aria-labelledby="{$step.buttonId}" @hidden={if step.current { null } else { true }} |' +
            '    {cem:project-payload @select="step.children" | }}}}' +
            ' {body |' +
            '  {cem:choose |' +
            '   {cem:when @test=datadom.slices.authoringValid |' +
            '    {section @class=cem-stepper @data-orientation="{$datadom.slices.orientation}" @aria-label="{$label}" |' +
            '     {ol @class=cem-stepper__steps |' +
            '      {cem:for-each @select=datadom.slices.steps @as=step | {call @template=stepper-header @with:step="{$step}"}}}' +
            '     {div @class=cem-stepper__panels |' +
            '      {cem:for-each @select=datadom.slices.steps @as=step | {call @template=stepper-panel @with:step="{$step}"}}}}}' +
            '   {cem:otherwise | {span @class="cem-stepper cem-stepper--invalid" @hidden=true | }}}}}',
    },
    {
        tag: 'cem-step',
        description: 'Inert labeled workflow-step payload consumed by cem-stepper.',
        cemMl: '{div @class=cem-step | {slot}}',
    },
    {
        tag: 'cem-paginator',
        description: 'Labeled paged-content navigation with external data ownership.',
        behavior: CEM_PAGINATOR_BEHAVIOR,
        cemMl:
            '{attribute @name=label | Pagination}' +
            '{attribute @name=items-per-page-label | Items per page}' +
            '{attribute @name=first-page-label | First page}' +
            '{attribute @name=previous-page-label | Previous page}' +
            '{attribute @name=next-page-label | Next page}' +
            '{attribute @name=last-page-label | Last page}' +
            '{attribute @name=of-label | of}' +
            '{nav @class=cem-paginator @aria-label="{$label}" |' +
            ' {cem:if @test="!datadom.attributes.hide-page-size" |' +
            '  {label @class=cem-paginator__page-size |' +
            '   {span @class=cem-paginator__page-size-label | {$datadom.slices.itemsPerPageLabel}}' +
            '   {select @class=cem-paginator__page-size-control @disabled={datadom.attributes.disabled} |' +
            '    {cem:for-each @select=datadom.slices.pageSizeOptions @as=option |' +
            '     {option @value="{$option.value}" @selected={if option.selected { true } else { null }} | {$option.label}}}}}}' +
            ' {div @class=cem-paginator__range-actions |' +
            '  {span @class=cem-paginator__range @role=status @aria-live=polite @aria-atomic=true | {$datadom.slices.rangeLabel}}' +
            '  {cem:if @test="datadom.attributes.show-first-last" |' +
            '   {button @type=button @class=cem-paginator__action @data-page-action=first @disabled={datadom.attributes.disabled} @aria-disabled={if datadom.slices.previousDisabled { true } else { null }} @tabindex={if datadom.slices.previousDisabled { -1 } else { null }} @aria-label="{$datadom.slices.firstPageLabel}" | {span @class=cem-paginator__icon @aria-hidden=true | «}}}' +
            '  {button @type=button @class=cem-paginator__action @data-page-action=previous @disabled={datadom.attributes.disabled} @aria-disabled={if datadom.slices.previousDisabled { true } else { null }} @tabindex={if datadom.slices.previousDisabled { -1 } else { null }} @aria-label="{$datadom.slices.previousPageLabel}" | {span @class=cem-paginator__icon @aria-hidden=true | ‹}}' +
            '  {button @type=button @class=cem-paginator__action @data-page-action=next @disabled={datadom.attributes.disabled} @aria-disabled={if datadom.slices.nextDisabled { true } else { null }} @tabindex={if datadom.slices.nextDisabled { -1 } else { null }} @aria-label="{$datadom.slices.nextPageLabel}" | {span @class=cem-paginator__icon @aria-hidden=true | ›}}' +
            '  {cem:if @test="datadom.attributes.show-first-last" |' +
            '   {button @type=button @class=cem-paginator__action @data-page-action=last @disabled={datadom.attributes.disabled} @aria-disabled={if datadom.slices.nextDisabled { true } else { null }} @tabindex={if datadom.slices.nextDisabled { -1 } else { null }} @aria-label="{$datadom.slices.lastPageLabel}" | {span @class=cem-paginator__icon @aria-hidden=true | »}}}}}',
    },
    {
        tag: 'cem-tooltip',
        description: 'Supplemental plain-text tooltip retaining one authored native trigger owner.',
        behavior: CEM_TOOLTIP_BEHAVIOR,
        cemMl:
            '{attribute @name=message | }' +
            '{attribute @name=position | below}' +
            '{attribute @name=show-delay | 0}' +
            '{attribute @name=hide-delay | 0}' +
            '{span @class=cem-tooltip @data-mode="{$datadom.slices.mode}" @data-position="{$datadom.slices.position}" |' +
            ' {slot @name=trigger}' +
            ' {span @id="{$datadom.slices.descriptionId}" @class=cem-tooltip__description | {$datadom.slices.message}}' +
            ' {span @id="{$datadom.slices.surfaceId}" @class=cem-tooltip__surface @role=tooltip @popover=manual | {$datadom.slices.message}}}',
    },
    {
        tag: 'cem-dialog',
        description: 'MVP modal decision or focused task surface.',
        behavior: CEM_FEEDBACK_DIALOG_BEHAVIOR,
        cemMl:
            '{attribute @name=label | Dialog}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.transient" | {dialog @class=cem-dialog @aria-label="{$label}" | {slot}}}' +
            ' {cem:otherwise | {div @class=cem-dialog @role=dialog @aria-modal=true @aria-label="{$label}" | {slot}}}}',
    },
    {
        tag: 'cem-dialog-shell',
        description: 'Dialog shell with labeled light-DOM content.',
        behavior: CEM_FEEDBACK_DIALOG_BEHAVIOR,
        cemMl:
            '{attribute @name=label | Dialog}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.transient" | {dialog @class=cem-dialog-shell @aria-label="{$label}" | {slot}}}' +
            ' {cem:otherwise | {div @class=cem-dialog-shell @role=dialog @aria-modal=true @aria-label="{$label}" | {slot}}}}',
    },
    {
        tag: 'cem-sheet',
        description: 'MVP non-modal or edge-attached task surface.',
        cemMl:
            '{attribute @name=label | Sheet}' +
            '{cem:choose |' +
            ' {cem:when @test="datadom.attributes.transient" | {aside @class=cem-sheet @role=region @aria-label="{$label}" @hidden={if datadom.attributes.expanded { null } else { true }} | {slot}}}' +
            ' {cem:otherwise | {aside @class=cem-sheet @role=region @aria-label="{$label}" | {slot}}}}',
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
        tag: 'cem-progress-spinner',
        description: 'Non-interactive circular determinate or indeterminate progress indicator.',
        behavior: CEM_PROGRESS_SPINNER_BEHAVIOR,
        cemMl:
            '{attribute @name=label | Progress}' +
            '{span @class=cem-progress-spinner @role=progressbar @data-mode="{$datadom.slices.mode}" @aria-label="{$label}" @aria-describedby={datadom.attributes.describedby} @aria-valuemin={if datadom.slices.indeterminate { null } else { 0 }} @aria-valuemax={if datadom.slices.indeterminate { null } else { datadom.slices.max }} @aria-valuenow={if datadom.slices.indeterminate { null } else { datadom.slices.value }} |' +
            ' {element @name=svg @namespace="http://www.w3.org/2000/svg" |' +
            '  {attribute @name=class @value=cem-progress-spinner__svg}' +
            '  {attribute @name=viewBox @value="0 0 100 100"}' +
            '  {attribute @name=aria-hidden @value=true}' +
            '  {attribute @name=focusable @value=false}' +
            '  {element @name=circle @namespace="http://www.w3.org/2000/svg" |' +
            '   {attribute @name=class @value=cem-progress-spinner__track}' +
            '   {attribute @name=cx @value=50}' +
            '   {attribute @name=cy @value=50}' +
            '   {attribute @name=r @value=42}' +
            '   {attribute @name=pathLength @value=100}}' +
            '  {element @name=circle @namespace="http://www.w3.org/2000/svg" |' +
            '   {attribute @name=class @value=cem-progress-spinner__indicator}' +
            '   {attribute @name=cx @value=50}' +
            '   {attribute @name=cy @value=50}' +
            '   {attribute @name=r @value=42}' +
            '   {attribute @name=pathLength @value=100}' +
            '   {attribute @name=stroke-dasharray @value="{$datadom.slices.dashArray}"}}}}',
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

        const behavior = 'behavior' in primitive ? primitive.behavior : undefined;
        if (runtime.registerDeclaration(declaration, { behavior })) {
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
