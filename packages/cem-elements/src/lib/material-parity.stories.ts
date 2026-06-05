import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { CemElementRuntime, type CemElementRuntimeOptions } from './cem-elements.js';

/**
 * Material-component parity stories (`docs/todo.md` §3.1). Each named story reproduces a
 * legacy `~/aWork/custom-element-dist/src/material/` component's *characteristic in-scope
 * behavior* on the `<cem-element>` substrate, using the C2 feature set (attribute defaults,
 * `/datadom` selection, `??`, `cem:if`/`cem:choose` conditionals, declarative slots,
 * `<data>`/`<option>` payloads, slice events) and `src` declaration loading.
 *
 * Recorded migration decisions (intentional CEM-ML/CEM-QL replacements):
 * - Legacy templates author conditionals as bare `<choose>`/`<when>` and use `{$x}` text in
 *   DOM-mode templates; here they are authored as `type="text/cem-ml"` with `cem:if`/`cem:choose`
 *   and `{$datadom.attributes.x}` content (the WASM path), per the C2 substrate.
 * - Legacy XPath tests (`string-length($image)<3`, `contains($image,'/')`, `{//bend}`) become
 *   cem-ql functional selection over `/datadom`; string-format branching is shown here via
 *   attribute-presence `cem:if`/`cem:choose` (cem-ql `strings:*` qualified calls are a follow-up).
 * - Scoped `<style>` renders page-global (no light-DOM scoping). Bare `@scope/pkg`
 *   module-map `src` specifiers require host `loadSrcDocument` / `resolveModuleUrl` hooks.
 * - `<if><attribute>` boolean-attribute forwarding (`hasBoolAttribute`) is not reproduced; declared
 *   attributes + AVT cover the common cases.
 * - `/datadom` selection keys must avoid cem-ql builtin pipeline-step names (`first`, `last`,
 *   `take`, `drop`, `nth`, `where`, `target`), which shadow same-named record fields.
 */

const meta: Meta = {
    title: 'CEM Elements/Material Parity',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const MaterialIconParity: Story = {
    render: () => {
        const root = section('material icon parity');
        const runtime = makeRuntime('mat-icon');
        // icon: conditional icon presentation + `{$image}` interpolation + default slot.
        define(
            runtime,
            'mat-icon',
            '{span @class=cem-icon |' +
                ' {cem:choose |' +
                ' {cem:when @test="datadom.attributes.image" | {span @class="icon material-icons" | {$datadom.attributes.image}}}' +
                ' {cem:otherwise | {span @class="icon placeholder" | ?}}}' +
                ' {slot}}'
        );
        root.append(instance('mat-icon', { image: 'home' }, 'label'), instance('mat-icon', {}, ''));
        return root;
    },
    play: async ({ canvasElement }) => {
        const [withImage, withoutImage] = Array.from(canvasElement.querySelectorAll('mat-icon'));
        const named = await waitForElement(withImage, '.cem-icon');
        assertEqual(named.querySelector('.material-icons')?.textContent, 'home', 'cem:when renders the material icon name');
        assert(named.textContent?.includes('label'), 'the default slot projects the instance payload');

        const fallback = await waitForElement(withoutImage, '.cem-icon');
        assertEqual(fallback.querySelector('.placeholder')?.textContent, '?', 'cem:otherwise renders the placeholder');
    },
};

export const MaterialMenuParity: Story = {
    render: () => {
        const root = section('material menu parity');
        const runtime = makeRuntime('mat-menu');
        // menu: direction/justify attributes (AVT) + unnamed slot projecting menu items.
        define(
            runtime,
            'mat-menu',
            '{attribute @name=direction | row}' +
                '{nav @class="cem-menu {$direction}" @data-justify="{$datadom.attributes.justify}" | {slot}}'
        );
        const filled = document.createElement('mat-menu');
        filled.setAttribute('justify', 'start');
        filled.innerHTML = '<a href="#a">One</a><a href="#b">Two</a>';
        const defaulted = document.createElement('mat-menu');
        root.append(filled, defaulted);
        return root;
    },
    play: async ({ canvasElement }) => {
        const [filled, defaulted] = Array.from(canvasElement.querySelectorAll('mat-menu'));
        const nav = await waitForElement(filled, 'nav.cem-menu');
        assert(nav.classList.contains('row'), 'declared `direction` default flows into the AVT class');
        assertEqual(nav.getAttribute('data-justify'), 'start', 'host attribute resolves through the AVT');
        assertEqual(nav.querySelectorAll('a').length, 2, 'the unnamed slot projects the menu items');

        const defaultedNav = await waitForElement(defaulted, 'nav.cem-menu');
        assert(defaultedNav.classList.contains('row'), 'an unset attribute uses its declared default');
    },
};

export const MaterialBadgeParity: Story = {
    render: () => {
        const root = section('material badge parity');
        const runtime = makeRuntime('mat-badge');
        // badge: color default + cem:if-gated badge value + slotted host content.
        define(
            runtime,
            'mat-badge',
            '{attribute @name=color | primary}' +
                '{span @class="cem-badge {$color}" |' +
                ' {slot}' +
                ' {cem:if @test="datadom.attributes.text" | {span @class=badge-dd | {$datadom.attributes.text}}}}'
        );
        const withText = document.createElement('mat-badge');
        withText.setAttribute('text', '5');
        withText.setAttribute('color', 'alert');
        withText.innerHTML = '<button>Inbox</button>';
        const empty = document.createElement('mat-badge');
        empty.innerHTML = '<button>Empty</button>';
        root.append(withText, empty);
        return root;
    },
    play: async ({ canvasElement }) => {
        const [withText, empty] = Array.from(canvasElement.querySelectorAll('mat-badge'));
        const badge = await waitForElement(withText, '.cem-badge');
        assert(badge.classList.contains('alert'), 'the `color` host attribute overrides its default');
        assertEqual(badge.querySelector('.badge-dd')?.textContent, '5', 'cem:if renders the badge value when `text` is set');
        assert(badge.querySelector('button')?.textContent === 'Inbox', 'host content projects through the slot');

        const emptyBadge = await waitForElement(empty, '.cem-badge');
        assert(emptyBadge.classList.contains('primary'), 'the declared `color` default applies when unset');
        assert(emptyBadge.querySelector('.badge-dd') === null, 'cem:if omits the badge value when `text` is absent');
    },
};

export const MaterialActionParity: Story = {
    render: () => {
        const root = section('material action parity');
        const runtime = makeRuntime('mat-action');
        // A nested icon component composed into the action button (in-document composition).
        define(runtime, 'mat-action-icon', '{i @class="icon {$datadom.attributes.image}" | }');
        // action: `text` default, slot-with-fallback, conditional nested icon, and a click slice.
        define(
            runtime,
            'mat-action',
            '{attribute @name=text | Action}' +
                '{button @type=button @slice=pressed @slice-event=click @slice-value="\'on\'" |' +
                ' {cem:if @test="datadom.attributes.icon" | {mat-action-icon @image="{$datadom.attributes.icon}" | }}' +
                ' {slot | {$text}}}' +
                '{span @class=state | {$datadom.slices.pressed}}'
        );
        const labelled = document.createElement('mat-action');
        labelled.setAttribute('icon', 'send');
        labelled.textContent = 'Submit';
        const defaulted = document.createElement('mat-action');
        root.append(labelled, defaulted);
        return root;
    },
    play: async ({ canvasElement }) => {
        const [labelled, defaulted] = Array.from(canvasElement.querySelectorAll('mat-action'));

        const button = (await waitForElement(labelled, 'button')) as HTMLButtonElement;
        assert(button.textContent?.includes('Submit'), 'slotted host content fills the button slot');
        // The conditional nested icon upgrades and renders its own template.
        const icon = await waitForElement(labelled, 'mat-action-icon i.icon');
        assert(icon.classList.contains('send'), 'a nested component composes and renders from a forwarded attribute');

        const fallbackButton = await waitForElement(defaulted, 'button');
        assert(fallbackButton.textContent?.includes('Action'), 'the slot fallback uses the declared `text` default');

        // A click drives the slice and re-renders the slice display.
        button.dispatchEvent(new Event('click', { bubbles: true }));
        await waitForCondition(
            () => requiredElement(labelled, '.state').textContent?.trim() === 'on',
            'a slice-event click updates the slice state and re-renders'
        );
    },
};

export const MaterialDropdownParity: Story = {
    render: () => {
        const root = section('material dropdown parity');
        const runtime = makeRuntime('mat-dropdown');
        // dropdown: label attribute, a click slice that toggles the panel, and a slotted panel.
        define(
            runtime,
            'mat-dropdown',
            '{attribute @name=label | Menu}' +
                '{div @class=cem-dropdown |' +
                ' {button @type=button @slice=open @slice-event=click @slice-value="\'open\'" | {$label}}' +
                ' {cem:if @test="datadom.slices.open" | {div @class=panel | {slot}}}}'
        );
        const dropdown = document.createElement('mat-dropdown');
        dropdown.setAttribute('label', 'File');
        dropdown.innerHTML = '<a href="#new">New</a>';
        root.append(dropdown);
        return root;
    },
    play: async ({ canvasElement }) => {
        const dropdown = requiredElement(canvasElement, 'mat-dropdown');
        const button = (await waitForElement(dropdown, 'button')) as HTMLButtonElement;
        assertEqual(button.textContent?.trim(), 'File', 'the dropdown label renders from the host attribute');
        assert(dropdown.querySelector('.panel') === null, 'the panel stays hidden until the open slice is set');

        button.dispatchEvent(new Event('click', { bubbles: true }));
        const panel = await waitForElement(dropdown, '.panel');
        assertEqual(panel.querySelector('a')?.textContent, 'New', 'opening the slice reveals the slotted items');
    },
};

export const MaterialInputParity: Story = {
    render: () => {
        const root = section('material input parity');
        const runtime = makeRuntime('mat-input');
        // input: type default, AVT type/value, a named label slot with a placeholder fallback.
        define(
            runtime,
            'mat-input',
            '{attribute @name=type | text}' +
                '{label @class=cem-input |' +
                ' {span @class=label | {slot @name=label | {$datadom.attributes.placeholder}}}' +
                ' {input @type="{$type}" @value="{$datadom.attributes.value}" @slice=value @slice-event=input | }}'
        );
        const email = document.createElement('mat-input');
        email.setAttribute('type', 'email');
        email.setAttribute('value', 'a@b.com');
        email.innerHTML = '<span slot="label">Email</span>';
        const plain = document.createElement('mat-input');
        plain.setAttribute('value', 'x');
        plain.setAttribute('placeholder', 'Search');
        root.append(email, plain);
        return root;
    },
    play: async ({ canvasElement }) => {
        const [email, plain] = Array.from(canvasElement.querySelectorAll('mat-input'));
        const input = (await waitForElement(email, 'input')) as HTMLInputElement;
        assertEqual(input.getAttribute('type'), 'email', 'the host `type` drives the input type');
        assertEqual(input.getAttribute('value'), 'a@b.com', 'the value resolves through AVT');
        assertEqual(requiredElement(email, '.label').textContent, 'Email', 'a named slot projects the label');

        const plainInput = (await waitForElement(plain, 'input')) as HTMLInputElement;
        assertEqual(plainInput.getAttribute('type'), 'text', 'the declared `type` default applies when unset');
        assertEqual(requiredElement(plain, '.label').textContent, 'Search', 'the named slot fallback uses the placeholder');
    },
};

export const MaterialIconLinkParity: Story = {
    render: () => {
        const root = section('material icon-link parity');
        const runtime = makeRuntime('mat-icon-link', {
            resolveModuleUrl: (specifier) => {
                const resolved: Record<string, string> = {
                    '@epa-wg/material': 'https://cdn.example.test/@epa-wg/material',
                    '@epa-wg/custom-element/demo/wc-square.svg':
                        'https://cdn.example.test/@epa-wg/custom-element/demo/wc-square.svg',
                };
                return resolved[specifier] ?? specifier;
            },
        });
        define(runtime, 'mat-iconlink-icon', '{i @class="icon {$datadom.attributes.image}" | }');
        // icon-link: href default + AVT, module-url resource slices, a conditional nested
        // icon, and a default slot label.
        define(
            runtime,
            'mat-icon-link',
            '{module-url @slice=cemurl @src="@epa-wg/material"}' +
                '{module-url @slice=logourl @src="@epa-wg/custom-element/demo/wc-square.svg"}' +
                '{attribute @name=href | #}' +
                '{a @class=cem-icon-link @href="{$datadom.slices.cemurl}" |' +
                ' {img @class=resolved-logo @src="{$datadom.slices.logourl}" @alt="" | }' +
                ' {cem:if @test="datadom.attributes.icon" | {mat-iconlink-icon @image="{$datadom.attributes.icon}" | }}' +
                ' {slot}}'
        );
        const link = document.createElement('mat-icon-link');
        link.setAttribute('href', '/home');
        link.setAttribute('icon', 'home');
        link.textContent = 'Home';
        root.append(link);
        return root;
    },
    play: async ({ canvasElement }) => {
        const link = requiredElement(canvasElement, 'mat-icon-link');
        await waitForElement(link, 'a.cem-icon-link');
        await waitForCondition(
            () =>
                requiredElement(link, 'a.cem-icon-link').getAttribute('href') ===
                'https://cdn.example.test/@epa-wg/material',
            'the module-url `cemurl` slice rerenders the link href'
        );
        const anchor = requiredElement(link, 'a.cem-icon-link') as HTMLAnchorElement;
        const logo = requiredElement(link, 'img.resolved-logo') as HTMLImageElement;
        assertEqual(
            logo.getAttribute('src'),
            'https://cdn.example.test/@epa-wg/custom-element/demo/wc-square.svg',
            'the module-url `logourl` slice resolves the image URL'
        );
        assert(link.querySelector('module-url') === null, 'module-url helper elements stay inert in rendered output');
        const icon = await waitForElement(link, 'mat-iconlink-icon i.icon');
        assert(icon.classList.contains('home'), 'the nested icon composes from the forwarded attribute');
        assert(anchor.textContent?.includes('Home'), 'the default slot projects the link label');
    },
};

export const MaterialAutocompleteParity: Story = {
    render: () => {
        const root = section('material autocomplete parity');
        const runtime = makeRuntime('mat-autocomplete');
        define(runtime, 'mat-ac-input', '{input @type=text @value="{$datadom.attributes.value}" | }');
        // autocomplete: a named input slot (falling back to a nested input) + `<data>` options
        // exposed under /datadom and rendered conditionally.
        // NB cem-ql record fields that collide with builtin pipeline steps (`first`, `last`,
        // `take`, `drop`, `nth`, `where`, `target`) are shadowed by the builtin, so `<data>`
        // keys avoid them — a documented selection-key migration constraint.
        define(
            runtime,
            'mat-autocomplete',
            '{div @class=cem-autocomplete |' +
                ' {slot @name=input | {mat-ac-input @value="{$datadom.attributes.value}" | }}' +
                ' {ul @class=options |' +
                ' {cem:if @test="datadom.data.apple" | {li @class=opt | {$datadom.data.apple.label}}}' +
                ' {cem:if @test="datadom.data.banana" | {li @class=opt | {$datadom.data.banana.label}}}}}'
        );
        const autocomplete = document.createElement('mat-autocomplete');
        autocomplete.setAttribute('value', 'a');
        autocomplete.innerHTML = '<data value="apple">Apple</data><data value="banana">Banana</data>';
        root.append(autocomplete);
        return root;
    },
    play: async ({ canvasElement }) => {
        const autocomplete = requiredElement(canvasElement, 'mat-autocomplete');
        const input = (await waitForElement(autocomplete, 'input')) as HTMLInputElement;
        assertEqual(input.getAttribute('value'), 'a', 'the named-slot fallback nested input composes with the forwarded value');
        const options = Array.from(autocomplete.querySelectorAll('.options .opt')).map((el) => el.textContent?.trim());
        assertEqual(options.join(','), 'Apple,Banana', '<data> payloads expose options under /datadom for selection');
    },
};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

let runtimeSequence = 0;

function makeRuntime(prefix: string, options: Omit<CemElementRuntimeOptions, 'declarationTag'> = {}): CemElementRuntime {
    runtimeSequence += 1;
    return new CemElementRuntime({ ...options, declarationTag: `mat-decl-${prefix}-${runtimeSequence}` });
}

/** Register a CEM-ML component declaration (no DOM attachment, so it auto-defines its tag). */
function define(runtime: CemElementRuntime, tag: string, cemMl: string): void {
    const declaration = document.createElement('div');
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.setAttribute('type', 'text/cem-ml');
    template.textContent = cemMl;
    declaration.appendChild(template);
    assert(runtime.registerDeclaration(declaration), `registered <${tag}>`);
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

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) {
        throw new Error(message);
    }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${String(expected)}, got ${String(actual)}`);
    }
}

function requiredElement(root: ParentNode, selector: string): Element {
    const element = root.querySelector(selector);
    assert(element, `expected ${selector} to exist`);
    return element;
}

function nextFrame(): Promise<void> {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function waitForElement(root: ParentNode, selector: string, frames = 120): Promise<Element> {
    for (let attempt = 0; attempt < frames; attempt += 1) {
        const found = root.querySelector(selector);
        if (found) {
            return found;
        }
        await nextFrame();
    }
    throw new Error(`expected ${selector} within ${frames} frames`);
}

async function waitForCondition(predicate: () => boolean, message: string, frames = 120): Promise<void> {
    for (let attempt = 0; attempt < frames; attempt += 1) {
        if (predicate()) {
            return;
        }
        await nextFrame();
    }
    throw new Error(`${message} within ${frames} frames`);
}
