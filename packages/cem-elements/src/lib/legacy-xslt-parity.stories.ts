import type { Meta, StoryObj } from '@storybook/web-components-vite';
import attributeInvalidationCemFixture from '../../tests/parity/legacy/attribute-invalidation.cem.html?raw';
import attributeInvalidationLegacyFixture from '../../tests/parity/legacy/attribute-invalidation.legacy.html?raw';
import attributesCemFixture from '../../tests/parity/legacy/attributes.cem.html?raw';
import attributesLegacyFixture from '../../tests/parity/legacy/attributes.legacy.html?raw';
import conditionalsCemFixture from '../../tests/parity/legacy/conditionals.cem.html?raw';
import conditionalsLegacyFixture from '../../tests/parity/legacy/conditionals.legacy.html?raw';
import datadomAccessCemFixture from '../../tests/parity/legacy/datadom-access.cem.html?raw';
import datadomAccessLegacyFixture from '../../tests/parity/legacy/datadom-access.legacy.html?raw';
import declarationRegistrationCemFixture from '../../tests/parity/legacy/declaration-registration.cem.html?raw';
import declarationRegistrationLegacyFixture from '../../tests/parity/legacy/declaration-registration.legacy.html?raw';
import externalSrcCemFixture from '../../tests/parity/legacy/external-src.cem.html?raw';
import externalSrcLegacyFixture from '../../tests/parity/legacy/external-src.legacy.html?raw';
import externalSrcTemplatesFixture from '../../tests/parity/legacy/external-src.templates.html?raw';
import inlineTemplateShapeCemFixture from '../../tests/parity/legacy/inline-template-shape.cem.html?raw';
import inlineTemplateShapeLegacyFixture from '../../tests/parity/legacy/inline-template-shape.legacy.html?raw';
import legacyBridgeCemFixture from '../../tests/parity/legacy/legacy-bridge.cem.html?raw';
import legacyBridgeLegacyFixture from '../../tests/parity/legacy/legacy-bridge.legacy.html?raw';
import localSrcCemFixture from '../../tests/parity/legacy/local-src.cem.html?raw';
import localSrcLegacyFixture from '../../tests/parity/legacy/local-src.legacy.html?raw';
import payloadCaptureCemFixture from '../../tests/parity/legacy/payload-capture.cem.html?raw';
import payloadCaptureLegacyFixture from '../../tests/parity/legacy/payload-capture.legacy.html?raw';
import sliceEventsCemFixture from '../../tests/parity/legacy/slice-events.cem.html?raw';
import sliceEventsLegacyFixture from '../../tests/parity/legacy/slice-events.legacy.html?raw';
import slotsCemFixture from '../../tests/parity/legacy/slots.cem.html?raw';
import slotsLegacyFixture from '../../tests/parity/legacy/slots.legacy.html?raw';
import { assertPhase3Accessibility } from '../../.storybook/accessibility-contract.js';
import { CemElementRuntime, type CemElementRuntimeOptions } from './cem-elements.js';

/**
 * Legacy HTML+XSLT ⇆ CEM-ML parity stories.
 *
 * Each story registers a **legacy** `<custom-element>` template authored as declarative HTML+XSLT
 * (bare `for-each`/`if`/`choose`/`when`, `{$x}` AVT, XPath functions) and its **hand-written CEM-ML
 * twin**, instantiates both, and asserts they render **identical** light DOM. This proves the
 * runtime `legacy-xslt` mode lowers legacy markup to CEM-ML through the CEM-owned engine
 * (`cem_ml::legacy_custom_element` via the cem_ql WASM module) and renders it on the same engine as
 * migrated templates — one compiler, no browser XSLT engine involved.
 */

const meta: Meta = {
    title: 'CEM Elements/Legacy XSLT Parity',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const LegacyIconLinkXsltParity: Story = {
    render: () => {
        const root = section('legacy icon-link parity');
        // Legacy HTML+XSLT (verbatim material `icon-link` shape: choose/when + contains() + AVT + slot).
        defineLegacy(
            makeRuntime('legacy-iconlink'),
            'legacy-iconlink',
            '<a href="{$href}">' +
                '<choose>' +
                "<when test=\"contains($icon,'/')\"><img class=\"icon\" src=\"{$icon}\"/></when>" +
                '<when test="$icon"><span class="icon material-icons">{$icon}</span></when>' +
                '</choose>' +
                '<slot></slot>' +
                '</a>'
        );
        // Hand-written CEM-ML twin.
        defineCemMl(
            makeRuntime('cem-iconlink'),
            'cem-iconlink',
            '{a @href="{$href}" |' +
                ' {cem:choose |' +
                ' {cem:when @test=\'str:contains(icon, "/")\' | {img @class="icon" @src="{$icon}"}}' +
                ' {cem:when @test="icon" | {span @class="icon material-icons" | {$icon}}}}' +
                ' {slot}}'
        );
        root.append(
            instance('legacy-iconlink', { href: '#go', icon: 'recycling' }, 'Recycle'),
            instance('cem-iconlink', { href: '#go', icon: 'recycling' }, 'Recycle')
        );
        return root;
    },
    play: async ({ canvasElement }) => {
        const legacy = requiredElement(canvasElement, 'legacy-iconlink') as HTMLElement;
        const twin = requiredElement(canvasElement, 'cem-iconlink') as HTMLElement;

        // The legacy instance renders the expected structure through conversion → WASM.
        const link = (await waitForElement(legacy, 'a.icon, a')) as HTMLAnchorElement;
        assertEqual(link.getAttribute('href'), '#go', 'AVT href resolves');
        const icon = await waitForElement(legacy, 'span.material-icons');
        assertEqual(icon.textContent, 'recycling', 'cem:when ($icon truthy, not a path) renders the icon name');
        assert(legacy.textContent?.includes('Recycle'), 'the default slot projects the payload');

        // Legacy output equals its hand-written CEM-ML twin (ignoring cem-internal frame attrs).
        await waitForElement(twin, 'span.material-icons');
        assertEqual(renderedHtml(legacy), renderedHtml(twin), 'legacy HTML+XSLT renders identically to its CEM-ML twin');
    },
};

export const LegacyIfNotXsltParity: Story = {
    render: () => {
        const root = section('legacy if/not parity');
        defineLegacy(
            makeRuntime('legacy-field'),
            'legacy-field',
            '<attribute name="disabled"></attribute>' +
                '<label>{$label}' +
                '<if test="not($disabled)"><input type="text"/></if>' +
                '</label>'
        );
        defineCemMl(
            makeRuntime('cem-field'),
            'cem-field',
            '{attribute @name=disabled}' +
                '{label | {$label}{cem:if @test="!disabled" | {input @type="text"}}}'
        );
        root.append(
            instance('legacy-field', { label: 'Name' }, ''),
            instance('cem-field', { label: 'Name' }, '')
        );
        return root;
    },
    play: async ({ canvasElement }) => {
        const legacy = requiredElement(canvasElement, 'legacy-field') as HTMLElement;
        const twin = requiredElement(canvasElement, 'cem-field') as HTMLElement;
        await waitForElement(legacy, 'label');
        const input = await waitForElement(legacy, 'input');
        assertEqual(input.getAttribute('type'), 'text', 'cem:if with not() renders the input when not disabled');
        await waitForElement(twin, 'input');
        assertEqual(renderedHtml(legacy), renderedHtml(twin), 'legacy if/not renders identically to its CEM-ML twin');
    },
};

export const LegacyForEachXsltParity: Story = {
    render: () => {
        const root = section('legacy for-each parity');
        // Legacy: inline node-set variable + for-each with position() and the context item `.`.
        defineLegacy(
            makeRuntime('legacy-fruits'),
            'legacy-fruits',
            '<xsl:variable name="fruits">' +
                '<item>Apple</item><item>Banana</item><item>Cherry</item>' +
                '</xsl:variable>' +
                '<ul><for-each select="exsl:node-set($fruits)/*"><li>{position()}. {.}</li></for-each></ul>'
        );
        // Twin: the unrolled CEM-ML the converter produces for a static node-set.
        defineCemMl(
            makeRuntime('cem-fruits'),
            'cem-fruits',
            '{ul | {li | 1. Apple}{li | 2. Banana}{li | 3. Cherry}}'
        );
        root.append(instance('legacy-fruits', {}, ''), instance('cem-fruits', {}, ''));
        return root;
    },
    play: async ({ canvasElement }) => {
        const legacy = requiredElement(canvasElement, 'legacy-fruits') as HTMLElement;
        const twin = requiredElement(canvasElement, 'cem-fruits') as HTMLElement;
        await waitForElement(legacy, 'ul li');
        const items = Array.from(legacy.querySelectorAll('li')).map((li) => li.textContent?.trim());
        assertEqual(items.join('|'), '1. Apple|2. Banana|3. Cherry', 'for-each unrolls the node-set with position()');
        await waitForElement(twin, 'ul li');
        assertEqual(renderedHtml(legacy), renderedHtml(twin), 'legacy for-each renders identically to the unrolled CEM-ML twin');
    },
};

export const LegacyChooseOtherwiseParity: Story = {
    render: () => {
        const root = section('legacy choose/otherwise parity');
        // Legacy: choose with a datadom-attribute when + an otherwise fallback (material badge shape).
        defineLegacy(
            makeRuntime('legacy-badge'),
            'legacy-badge',
            '<attribute name="kind"></attribute>' +
                '<span class="badge {$kind}">' +
                '<choose>' +
                '<when test="$kind"><b>{$kind}</b></when>' +
                '<otherwise><i>none</i></otherwise>' +
                '</choose>' +
                '</span>'
        );
        defineCemMl(
            makeRuntime('cem-badge'),
            'cem-badge',
            '{attribute @name=kind}' +
                '{span @class="badge {$kind}" |' +
                ' {cem:choose |' +
                ' {cem:when @test="kind" | {b | {$kind}}}' +
                ' {cem:otherwise | {i | none}}}}'
        );
        root.append(
            instance('legacy-badge', { kind: 'alert' }, ''),
            instance('cem-badge', { kind: 'alert' }, ''),
            instance('legacy-badge', {}, ''),
            instance('cem-badge', {}, '')
        );
        return root;
    },
    play: async ({ canvasElement }) => {
        const [legacySet, twinSet, legacyNone, twinNone] = [
            ...canvasElement.querySelectorAll('legacy-badge, cem-badge'),
        ] as HTMLElement[];
        await waitForElement(legacySet, 'b');
        assertEqual(legacySet.querySelector('b')?.textContent, 'alert', 'when branch renders the kind');
        await waitForElement(legacyNone, 'i');
        assertEqual(legacyNone.querySelector('i')?.textContent, 'none', 'otherwise branch renders the fallback');
        await waitForElement(twinSet, 'b');
        await waitForElement(twinNone, 'i');
        assertEqual(renderedHtml(legacySet), renderedHtml(twinSet), 'when-branch legacy ≡ CEM-ML twin');
        assertEqual(renderedHtml(legacyNone), renderedHtml(twinNone), 'otherwise-branch legacy ≡ CEM-ML twin');
    },
};

export const LegacyForEachAttrParity: Story = {
    render: () => {
        const root = section('legacy for-each @attr parity');
        // Legacy: for-each over an inline node-set with attributes, AVT, and position().
        defineLegacy(
            makeRuntime('legacy-swatches'),
            'legacy-swatches',
            '<xsl:variable name="colors">' +
                '<color hex="#f00">Red</color><color hex="#0f0">Green</color>' +
                '</xsl:variable>' +
                '<for-each select="exsl:node-set($colors)/*">' +
                '<div class="swatch" style="background:{@hex}">{position()}. {.}</div>' +
                '</for-each>'
        );
        defineCemMl(
            makeRuntime('cem-swatches'),
            'cem-swatches',
            '{div @class="swatch" @style="background:#f00" | 1. Red}' +
                '{div @class="swatch" @style="background:#0f0" | 2. Green}'
        );
        root.append(instance('legacy-swatches', {}, ''), instance('cem-swatches', {}, ''));
        return root;
    },
    play: async ({ canvasElement }) => {
        const legacy = requiredElement(canvasElement, 'legacy-swatches') as HTMLElement;
        const twin = requiredElement(canvasElement, 'cem-swatches') as HTMLElement;
        await waitForElement(legacy, 'div.swatch');
        const swatches = Array.from(legacy.querySelectorAll('div.swatch'));
        assertEqual(swatches.length, 2, 'for-each unrolls both members');
        assertEqual(
            (swatches[0] as HTMLElement).style.background,
            'rgb(255, 0, 0)',
            '@hex AVT substitutes the member attribute literal'
        );
        assertEqual(swatches[1]?.textContent?.trim(), '2. Green', 'position() and `.` substitute literals');
        await waitForElement(twin, 'div.swatch');
        assertEqual(renderedHtml(legacy), renderedHtml(twin), 'legacy for-each ≡ unrolled CEM-ML twin');
    },
};

export const LegacySliceIfOrderingParity: Story = {
    render: () => {
        const root = section('legacy slice-driven if ordering parity');
        // Legacy `xslt/if` pattern: a checkbox slice drives two `<if>` blocks; the inline one must
        // render in document order between the ▶ ◀ markers. Authored as legacy HTML+XSLT.
        defineLegacy(
            makeRuntime('legacy-toggle'),
            'legacy-toggle',
            '<slice name="show-a">false</slice>' +
                '<div class="whole">' +
                '<label><input type="checkbox" value="AA" slice="show-a" slice-event="change" slice-value="{$target.value}"/> A</label>' +
                '▶<if test="//show-a = \'AA\'">!A</if>◀' +
                '</div>' +
                '<if test="//show-a = \'AA\'"><div class="t1">T1</div></if>'
        );
        // Hand-written CEM-ML twin.
        defineCemMl(
            makeRuntime('cem-toggle'),
            'cem-toggle',
            '{slice @name=show-a | false}' +
                '{div @class=whole |' +
                ' {label | {input @type=checkbox @value=AA @slice=show-a @slice-event=change @slice-value="{$target.value}"} A}' +
                '▶{cem:if @test=\'datadom.slices.show-a == "AA"\' | !A}◀}' +
                '{cem:if @test=\'datadom.slices.show-a == "AA"\' | {div @class=t1 | T1}}'
        );
        root.append(instance('legacy-toggle', {}, ''), instance('cem-toggle', {}, ''));
        return root;
    },
    play: async ({ canvasElement }) => {
        const legacy = requiredElement(canvasElement, 'legacy-toggle') as HTMLElement;
        const twin = requiredElement(canvasElement, 'cem-toggle') as HTMLElement;

        const whole = await waitForElement(legacy, '.whole');
        assert(legacy.querySelector('.t1') === null, 'the gated block is absent before the slice is set');
        assert(/▶\s*◀/.test(whole.textContent ?? ''), 'inline block absent before toggle (▶ ◀)');

        // Toggle the checkbox → the change-event slice sets show-a; both `<if>` blocks render.
        const checkbox = (await waitForElement(legacy, 'input[type=checkbox]')) as HTMLInputElement;
        checkbox.checked = true;
        checkbox.dispatchEvent(new Event('change', { bubbles: true }));

        await waitForElement(legacy, '.t1');
        const wholeAfter = requiredElement(legacy, '.whole');
        assert(/▶\s*!A\s*◀/.test(wholeAfter.textContent ?? ''), 'inline block renders in order between ▶ and ◀');

        // Drive the twin identically and compare.
        const twinCheckbox = (await waitForElement(twin, 'input[type=checkbox]')) as HTMLInputElement;
        twinCheckbox.checked = true;
        twinCheckbox.dispatchEvent(new Event('change', { bubbles: true }));
        await waitForElement(twin, '.t1');
        assertEqual(renderedHtml(legacy), renderedHtml(twin), 'slice-driven legacy ≡ CEM-ML twin after toggle');
    },
};

// ---------------------------------------------------------------------------
// File-backed migration inventory
// ---------------------------------------------------------------------------

export const FileDeclarationRegistrationParity: Story = fileBackedParityStory(
    'declaration-registration',
    declarationRegistrationLegacyFixture,
    declarationRegistrationCemFixture,
    [{ selector: 'article.legacy-register-card', texts: ['Registered'] }]
);

export const FileInlineTemplateShapeParity: Story = fileBackedParityStory(
    'inline-template-shape',
    inlineTemplateShapeLegacyFixture,
    inlineTemplateShapeCemFixture,
    [{ selector: 'span', texts: ['Shape'] }],
    { instanceAttributes: { label: 'Shape' } }
);

export const FileLocalSrcParity: Story = fileBackedParityStory(
    'local-src',
    localSrcLegacyFixture,
    localSrcCemFixture,
    [{ selector: 'p.local', texts: ['Local'] }]
);

export const FileExternalSrcParity: Story = fileBackedParityStory(
    'external-src',
    externalSrcLegacyFixture,
    externalSrcCemFixture,
    [{ selector: 'section.external', texts: ['External'] }]
);

export const FilePayloadCaptureParity: Story = fileBackedParityStory(
    'payload-capture',
    payloadCaptureLegacyFixture,
    payloadCaptureCemFixture,
    [{ selector: 'article.payload strong', texts: ['Fallback payload'] }]
);

export const FileAttributeDefaultsOverridesParity: Story = fileBackedParityStory(
    'attribute-defaults-overrides',
    attributesLegacyFixture,
    attributesCemFixture,
    [{ selector: 'span.tone', texts: ['Default', 'Override'] }]
);

export const FileAttributeInvalidationParity: Story = fileBackedParityStory(
    'attribute-invalidation',
    attributeInvalidationLegacyFixture,
    attributeInvalidationCemFixture,
    [{ selector: 'span.label', texts: ['Before'] }],
    { mutatedExpectations: [{ selector: 'span.label', texts: ['After'] }] }
);

export const FileSlotsParity: Story = fileBackedParityStory(
    'slots',
    slotsLegacyFixture,
    slotsCemFixture,
    [
        { selector: 'h2', texts: ['Projected title'] },
        { selector: 'article > div', texts: ['Projected body'] },
    ]
);

export const FileSliceEventsParity: Story = fileBackedParityStory(
    'slice-events',
    sliceEventsLegacyFixture,
    sliceEventsCemFixture,
    [{ selector: 'output', texts: ['off'] }],
    { eventExpectations: [{ selector: 'output', texts: ['on'] }] }
);

export const FileDatadomAccessParity: Story = fileBackedParityStory(
    'datadom-access-migration',
    datadomAccessLegacyFixture,
    datadomAccessCemFixture,
    [{ selector: 'p', texts: ['From datadom', 'Fallback'] }]
);

export const FileConditionalsParity: Story = fileBackedParityStory(
    'conditionals',
    conditionalsLegacyFixture,
    conditionalsCemFixture,
    [
        { selector: 'strong', texts: ['alert'] },
        { selector: 'em', texts: ['none'] },
        { selector: 'span.present', texts: ['present'] },
    ]
);

export const FileLegacyBridgeParity: Story = fileBackedParityStory(
    'legacy-bridge',
    legacyBridgeLegacyFixture,
    legacyBridgeCemFixture,
    [{ selector: 'button[type="button"]', texts: ['Bridge'] }]
);

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

interface FileParityExpectation {
    selector: string;
    texts: string[];
}

interface FileParityStoryOptions {
    instanceAttributes?: Record<string, string>;
    mutatedExpectations?: FileParityExpectation[];
    eventExpectations?: FileParityExpectation[];
}

interface FileParitySide {
    root: HTMLElement;
    runtime: CemElementRuntime;
    declarations: HTMLElement[];
    instances: HTMLElement[];
}

interface FileParityState {
    id: string;
    legacy: FileParitySide;
    cemMl: FileParitySide;
}

const fileParityStates = new WeakMap<HTMLElement, FileParityState>();

function fileBackedParityStory(
    id: string,
    legacySource: string,
    cemMlSource: string,
    expectations: FileParityExpectation[],
    options: FileParityStoryOptions = {}
): Story {
    return {
        render: () => {
            const root = section(`file-backed legacy parity: ${id}`);
            root.dataset.legacyFileParity = id;
            const legacy = prepareFileParitySide(id, 'legacy', legacySource, options);
            const cemMl = prepareFileParitySide(id, 'cem-ml', cemMlSource, options);
            root.append(legacy.root, cemMl.root);
            fileParityStates.set(root, { id, legacy, cemMl });
            return root;
        },
        play: async ({ canvasElement }) => {
            const root = requiredElement(
                canvasElement,
                `section[data-legacy-file-parity="${id}"]`
            ) as HTMLElement;
            const state = fileParityStates.get(root);
            assert(state, `file-backed parity state exists for ${id}`);

            await registerFileParitySide(state.legacy, 'legacy');
            await registerFileParitySide(state.cemMl, 'CEM-ML');
            assertFileParity(state, expectations, 'initial render');

            if (options.mutatedExpectations) {
                mutateMarkedAttributes(state.legacy.instances);
                mutateMarkedAttributes(state.cemMl.instances);
                await settleFileParity(state);
                await waitForFileExpectations(state, options.mutatedExpectations, 'attribute invalidation');
                assertFileParity(state, options.mutatedExpectations, 'attribute invalidation');
            }

            if (options.eventExpectations) {
                dispatchMarkedSliceEvents(state.legacy.instances);
                dispatchMarkedSliceEvents(state.cemMl.instances);
                await settleFileParity(state);
                await waitForFileExpectations(state, options.eventExpectations, 'slice event');
                assertFileParity(state, options.eventExpectations, 'slice event');
            }
        },
    };
}

function prepareFileParitySide(
    id: string,
    mode: 'legacy' | 'cem-ml',
    source: string,
    options: FileParityStoryOptions
): FileParitySide {
    const root = document.createElement('div');
    root.dataset.paritySide = mode;
    const fixture = document.createElement('template');
    fixture.innerHTML = source;
    root.appendChild(fixture.content.cloneNode(true));

    const declarations = Array.from(root.querySelectorAll('cem-element')).map(neutralizeFixtureDeclaration);
    if (options.instanceAttributes) {
        const validDeclaration = declarations.find((declaration) => declaration.dataset.case !== 'live-content-invalid'
            && declaration.dataset.case !== 'missing-template-invalid');
        assert(validDeclaration, `${id} ${mode} fixture has a valid declaration`);
        const tag = validDeclaration.getAttribute('tag');
        assert(tag, `${id} ${mode} valid declaration has a tag`);
        root.appendChild(instance(tag, options.instanceAttributes, ''));
    }

    const runtime = new CemElementRuntime({
        declarationTag: `cem-element-file-${mode}-${id}`,
        loadSrcDocument: async (path) => {
            assertEqual(path, './external-src.templates.html', `${id} external source path`);
            return externalSrcTemplatesFixture;
        },
    });
    return { root, runtime, declarations, instances: [] };
}

function neutralizeFixtureDeclaration(declaration: Element): HTMLElement {
    const neutral = document.createElement('div');
    for (const attribute of Array.from(declaration.attributes)) {
        neutral.setAttribute(attribute.name, attribute.value);
    }
    while (declaration.firstChild) {
        neutral.appendChild(declaration.firstChild);
    }
    declaration.replaceWith(neutral);
    return neutral;
}

async function registerFileParitySide(side: FileParitySide, label: string): Promise<void> {
    const producedTags: string[] = [];
    for (const declaration of side.declarations) {
        const invalidCase = declaration.dataset.case?.endsWith('-invalid') ?? false;
        const accepted = side.runtime.registerDeclaration(declaration);
        if (invalidCase) {
            assertEqual(accepted, false, `${label} ${declaration.dataset.case} declaration is rejected`);
            const expectedCode = declaration.dataset.case === 'live-content-invalid'
                ? 'cem-element.declaration_live_content'
                : 'cem-element.inline_template_missing';
            assert(
                side.runtime.diagnosticsFor(declaration).some((diagnostic) => diagnostic.code === expectedCode),
                `${label} ${declaration.dataset.case} reports ${expectedCode}`
            );
            continue;
        }
        assert(accepted, `${label} file-backed declaration is accepted`);
        await side.runtime.whenDeclarationSettled(declaration);
        const tag = declaration.getAttribute('tag');
        assert(tag, `${label} file-backed declaration has a produced tag`);
        assert(window.customElements.get(tag), `${label} file-backed declaration registers <${tag}>`);
        producedTags.push(tag);
    }

    side.instances = producedTags.flatMap((tag) =>
        Array.from(side.root.querySelectorAll(tag)) as HTMLElement[]
    );
    assert(side.instances.length > 0, `${label} file-backed fixture exercises a produced instance`);
    await Promise.all(side.instances.map((producedInstance) => side.runtime.whenRenderSettled(producedInstance)));
}

function assertFileParity(
    state: FileParityState,
    expectations: FileParityExpectation[],
    phase: string
): void {
    assertEqual(
        state.legacy.instances.length,
        state.cemMl.instances.length,
        `${state.id} ${phase} instance count`
    );
    for (let index = 0; index < state.legacy.instances.length; index += 1) {
        assertEqual(
            renderedHtml(state.legacy.instances[index] as HTMLElement),
            renderedHtml(state.cemMl.instances[index] as HTMLElement),
            `${state.id} ${phase} legacy instance ${index + 1} matches its CEM-ML twin`
        );
    }
    assertFileExpectations(state.legacy, expectations, `${state.id} legacy ${phase}`);
    assertFileExpectations(state.cemMl, expectations, `${state.id} CEM-ML ${phase}`);
    assertPhase3Accessibility(state.legacy.instances, `${state.id} legacy ${phase}`);
    assertPhase3Accessibility(state.cemMl.instances, `${state.id} CEM-ML ${phase}`);
}

function assertFileExpectations(
    side: FileParitySide,
    expectations: FileParityExpectation[],
    label: string
): void {
    for (const expectation of expectations) {
        const texts = fileExpectationTexts(side, expectation);
        assertEqual(texts.join('|'), expectation.texts.join('|'), `${label} ${expectation.selector} output`);
    }
}

function fileExpectationTexts(side: FileParitySide, expectation: FileParityExpectation): string[] {
    return side.instances.flatMap((producedInstance) =>
        Array.from(producedInstance.querySelectorAll(expectation.selector), (element) =>
            element.textContent?.trim() ?? ''
        )
    );
}

async function waitForFileExpectations(
    state: FileParityState,
    expectations: FileParityExpectation[],
    phase: string,
    timeout = 2000
): Promise<void> {
    const start = Date.now();
    for (;;) {
        const ready = [state.legacy, state.cemMl].every((side) =>
            expectations.every((expectation) =>
                fileExpectationTexts(side, expectation).join('|') === expectation.texts.join('|')
            )
        );
        if (ready) {
            return;
        }
        if (Date.now() - start > timeout) {
            throw new Error(`${state.id} timed out waiting for ${phase} output`);
        }
        await new Promise((resolve) => setTimeout(resolve, 16));
    }
}

function mutateMarkedAttributes(instances: HTMLElement[]): void {
    for (const producedInstance of instances) {
        const instruction = producedInstance.dataset.mutate;
        if (!instruction) {
            continue;
        }
        const separator = instruction.indexOf(':');
        assert(separator > 0, `valid data-mutate instruction ${instruction}`);
        producedInstance.setAttribute(instruction.slice(0, separator), instruction.slice(separator + 1));
    }
}

function dispatchMarkedSliceEvents(instances: HTMLElement[]): void {
    for (const producedInstance of instances) {
        const input = requiredElement(producedInstance, 'input[type="checkbox"]') as HTMLInputElement;
        input.checked = true;
        input.dispatchEvent(new Event('change', { bubbles: true }));
    }
}

async function settleFileParity(state: FileParityState): Promise<void> {
    await Promise.all([
        ...state.legacy.instances.map((producedInstance) => state.legacy.runtime.whenRenderSettled(producedInstance)),
        ...state.cemMl.instances.map((producedInstance) => state.cemMl.runtime.whenRenderSettled(producedInstance)),
    ]);
}

let runtimeSequence = 0;

function makeRuntime(prefix: string, options: Omit<CemElementRuntimeOptions, 'declarationTag'> = {}): CemElementRuntime {
    runtimeSequence += 1;
    return new CemElementRuntime({ ...options, declarationTag: `legacy-decl-${prefix}-${runtimeSequence}` });
}

/** Register a legacy HTML+XSLT template through the explicit migration annotation. */
function defineLegacy(runtime: CemElementRuntime, tag: string, legacyHtml: string): void {
    const declaration = document.createElement('div');
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.setAttribute('lang', 'custom-element-v0');
    template.innerHTML = legacyHtml;
    declaration.appendChild(template);
    assert(runtime.registerDeclaration(declaration), `registered legacy <${tag}>`);
}

/** Register a canonical CEM-ML twin (`type="text/cem-ml"`). */
function defineCemMl(runtime: CemElementRuntime, tag: string, cemMl: string): void {
    const declaration = document.createElement('div');
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.setAttribute('type', 'text/cem-ml');
    template.textContent = cemMl;
    declaration.appendChild(template);
    assert(runtime.registerDeclaration(declaration), `registered cem-ml <${tag}>`);
}

function instance(tag: string, attributes: Record<string, string>, payload: string): HTMLElement {
    const element = document.createElement(tag);
    for (const [name, value] of Object.entries(attributes)) {
        element.setAttribute(name, value);
    }
    if (payload) {
        element.textContent = payload;
    }
    return element;
}

function section(label: string): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', label);
    return root;
}

/**
 * The rendered light DOM of an instance, normalized for legacy-vs-twin comparison: the inert
 * data-island and cem-internal `data-cem-*` frame attributes are removed, whitespace-only text
 * nodes (insignificant CEM-ML structural spacing) are dropped, and remaining text is collapsed.
 */
function renderedHtml(instance: HTMLElement): string {
    const clone = instance.cloneNode(true) as HTMLElement;
    clone.querySelectorAll('template').forEach((node) => node.remove());
    for (const element of [clone, ...Array.from(clone.querySelectorAll('*'))]) {
        for (const attribute of Array.from(element.attributes)) {
            if (attribute.name.startsWith('data-cem-')) {
                element.removeAttribute(attribute.name);
            }
        }
    }
    normalizeWhitespaceNodes(clone);
    return clone.innerHTML.trim();
}

/** Drop whitespace-only text nodes and collapse whitespace runs in the rest. */
function normalizeWhitespaceNodes(root: Node): void {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const texts: Text[] = [];
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        texts.push(node as Text);
    }
    for (const text of texts) {
        const value = text.textContent ?? '';
        if (value.trim() === '') {
            text.remove();
        } else {
            text.textContent = value.replace(/\s+/g, ' ').trim();
        }
    }
}

async function waitForElement(root: ParentNode, selector: string, timeout = 2000): Promise<Element> {
    const start = Date.now();
    for (;;) {
        const found = root.querySelector(selector);
        if (found) {
            return found;
        }
        if (Date.now() - start > timeout) {
            throw new Error(`timed out waiting for \`${selector}\``);
        }
        await new Promise((resolve) => setTimeout(resolve, 16));
    }
}

function requiredElement(root: ParentNode, selector: string): Element {
    const found = root.querySelector(selector);
    if (!found) {
        throw new Error(`expected element \`${selector}\``);
    }
    return found;
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) {
        throw new Error(message);
    }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
}
