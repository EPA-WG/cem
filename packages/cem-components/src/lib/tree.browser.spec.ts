import '@epa-wg/cem-theme/styles.css';
import { CemElementRuntime } from '@epa-wg/cem-elements';
import { userEvent } from 'vitest/browser';

import '../styles.css';
import treeContractFixture from '../../tests/tree/contract.html?raw';
import { installCemComponentPrimitives } from './primitives.js';
import {
    assertAccessibleName,
    assertAriaReferenceIntegrity,
    createComponentHarness,
    nextRenderFrame,
    type ComponentHarness,
} from './testing/component-harness.js';

interface TestCemTree extends HTMLElement {
    expandedValues: string[];
    selectedValues: string[];
}

interface TreeParts {
    host: TestCemTree;
    owner: HTMLElement;
    nodes: HTMLButtonElement[];
    groups: HTMLElement[];
}

describe('tree contract fixture', () => {
    let harness: ComponentHarness;
    let runtime: CemElementRuntime;

    beforeAll(async () => {
        runtime = new CemElementRuntime({ declarationTag: 'cem-components-tree-declaration' });
        const result = await installCemComponentPrimitives(runtime);
        expect(result.diagnostics).toEqual([]);
    });

    afterEach(() => {
        harness?.cleanup();
    });

    it('renders one named tree with exact native treeitems, explicit hierarchy metadata, and stable owned groups', async () => {
        expect(treeContractFixture).not.toMatch(/<script\b/i);
        expect(treeContractFixture).not.toMatch(/\son[a-z]+\s*=/i);

        const root = await renderFixture();
        const parts = treeParts(root, '#project-tree');
        const apps = treeNode(parts, 'apps');
        const web = treeNode(parts, 'web');
        const components = treeNode(parts, 'components');
        const readme = treeNode(parts, 'readme');

        expect(parts.owner.parentElement).toBe(parts.host);
        expect(parts.owner.getAttribute('role')).toBe('tree');
        expect(assertAccessibleName(parts.owner, 'Project files')).toBe('Project files');
        expect(parts.owner.hasAttribute('aria-multiselectable')).toBe(false);
        expect(parts.nodes).toHaveLength(10);
        expect(parts.groups).toHaveLength(5);
        expect(parts.host.querySelector('[role="treegrid"], [role="listbox"], [role="menu"]')).toBeNull();

        expect(apps).toBeInstanceOf(HTMLButtonElement);
        expect(apps.type).toBe('button');
        expect(apps.getAttribute('role')).toBe('treeitem');
        expect(apps.getAttribute('aria-level')).toBe('1');
        expect(apps.getAttribute('aria-posinset')).toBe('1');
        expect(apps.getAttribute('aria-setsize')).toBe('4');
        expect(apps.getAttribute('aria-expanded')).toBe('true');
        expect(assertAccessibleName(apps, 'Applications')).toBe('Applications');

        const appsGroup = ownedGroup(parts, apps);
        expect(appsGroup.getAttribute('role')).toBe('group');
        expect(appsGroup.hidden).toBe(false);
        expect(appsGroup.previousElementSibling).toBe(apps);
        expect(web.closest('[role="group"]')).toBe(appsGroup);
        expect(web.getAttribute('aria-level')).toBe('2');
        expect(web.getAttribute('aria-posinset')).toBe('1');
        expect(web.getAttribute('aria-setsize')).toBe('2');
        expect(web.hasAttribute('aria-expanded')).toBe(false);

        const componentsGroup = ownedGroup(parts, components);
        expect(components.getAttribute('aria-expanded')).toBe('false');
        expect(componentsGroup.hidden).toBe(true);
        expect(web.getAttribute('aria-selected')).toBe('true');
        expect(readme.getAttribute('aria-selected')).toBe('false');
        expect(parts.nodes.filter((node) => node.tabIndex === 0)).toEqual([web]);
        expect(parts.host.querySelector('cem-tree-item')).toBeNull();
        expect(() => assertAriaReferenceIntegrity(parts.host)).not.toThrow();

        const malformed = requiredElement<TestCemTree>(root, '#malformed-tree');
        expect(malformed.querySelector('.cem-tree--invalid')?.hasAttribute('hidden')).toBe(true);
        expect(malformed.querySelector('[role="tree"], [role="treeitem"], [role="group"]')).toBeNull();
    });

    it('toggles parents and activates leaves through one native path while selection and disabled state stay application-owned', async () => {
        const root = await renderFixture();
        let parts = treeParts(root, '#project-tree');
        const appsIdentity = treeNode(parts, 'apps');
        const events: Array<{ type: string; detail: unknown }> = [];
        for (const eventName of ['cem-tree-toggle', 'cem-tree-activate']) {
            parts.host.addEventListener(eventName, (event) => events.push({
                type: event.type,
                detail: (event as CustomEvent).detail,
            }));
        }

        await userEvent.click(appsIdentity);
        await waitFor(() => treeNode(treeParts(root, '#project-tree'), 'apps').getAttribute('aria-expanded') === 'false');
        parts = treeParts(root, '#project-tree');
        expect(treeNode(parts, 'apps')).toBe(appsIdentity);
        expect(parts.host.expandedValues).not.toContain('apps');
        expect(events).toEqual([{
            type: 'cem-tree-toggle',
            detail: { value: 'apps', expanded: false },
        }]);

        await userEvent.click(treeNode(parts, 'apps'));
        await waitFor(() => treeNode(treeParts(root, '#project-tree'), 'apps').getAttribute('aria-expanded') === 'true');
        parts = treeParts(root, '#project-tree');
        await userEvent.click(treeNode(parts, 'web'));
        await nextRenderFrame();
        expect(parts.host.getAttribute('selected-values')).toBe('readme web');
        expect(treeNode(treeParts(root, '#project-tree'), 'web').getAttribute('aria-selected')).toBe('true');
        expect(events.at(-1)).toEqual({ type: 'cem-tree-activate', detail: { value: 'web' } });

        treeNode(parts, 'web').focus();
        await userEvent.keyboard('{Enter}');
        expect(events.filter((event) => event.type === 'cem-tree-activate')).toHaveLength(2);
        treeNode(parts, 'api').click();
        await nextRenderFrame();
        expect(events.filter((event) => event.type === 'cem-tree-activate')).toHaveLength(2);
        expect(parts.host.getAttribute('selected-values')).toBe('readme web');
    });

    it('moves roving focus through visible nodes, parent-child arrows, boundaries, and typeahead without selection', async () => {
        const root = await renderFixture();
        let parts = treeParts(root, '#project-tree');
        const toggles: unknown[] = [];
        parts.host.addEventListener('cem-tree-toggle', (event) => toggles.push((event as CustomEvent).detail));

        requiredElement<HTMLButtonElement>(root, '[data-tree-focus-start]').focus();
        await userEvent.tab();
        expect(document.activeElement).toBe(treeNode(parts, 'web'));
        await userEvent.keyboard('{ArrowDown}');
        await nextRenderFrame();
        parts = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(parts, 'source'));
        expect(parts.host.getAttribute('selected-values')).toBe('readme web');

        await userEvent.keyboard('{ArrowRight}');
        await nextRenderFrame();
        parts = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(parts, 'components'));
        await userEvent.keyboard('{ArrowRight}');
        await waitFor(() => treeNode(treeParts(root, '#project-tree'), 'components').getAttribute('aria-expanded') === 'true');
        parts = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(parts, 'components'));
        await userEvent.keyboard('{ArrowRight}');
        await nextRenderFrame();
        parts = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(parts, 'button'));
        await userEvent.keyboard('{ArrowLeft}');
        await nextRenderFrame();
        parts = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(parts, 'components'));
        await userEvent.keyboard('{ArrowLeft}');
        await waitFor(() => treeNode(treeParts(root, '#project-tree'), 'components').getAttribute('aria-expanded') === 'false');

        await userEvent.keyboard('{Home}');
        await nextRenderFrame();
        parts = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(parts, 'apps'));
        await userEvent.keyboard('{ArrowUp}');
        expect(document.activeElement).toBe(treeNode(parts, 'apps'));
        await userEvent.keyboard('{End}');
        await nextRenderFrame();
        parts = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(parts, 'readme'));
        await userEvent.keyboard('{ArrowDown}');
        expect(document.activeElement).toBe(treeNode(parts, 'readme'));
        await userEvent.keyboard('so');
        await nextRenderFrame();
        parts = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(parts, 'source'));
        expect(parts.nodes.filter((node) => node.tabIndex === 0)).toEqual([treeNode(parts, 'source')]);
        expect(toggles).toEqual([
            { value: 'components', expanded: true },
            { value: 'components', expanded: false },
        ]);
        expect(parts.host.getAttribute('selected-values')).toBe('readme web');
    });

    it('projects silent single/multiple/none selection and application loading with late stable children', async () => {
        const root = await renderFixture();
        let project = treeParts(root, '#project-tree');
        const events: string[] = [];
        project.host.addEventListener('cem-tree-toggle', () => events.push('toggle'));
        project.host.addEventListener('cem-tree-activate', () => events.push('activate'));
        const web = treeNode(project, 'web');
        const asyncIdentity = treeNode(project, 'async');
        const asyncGroupIdentity = ownedGroup(project, asyncIdentity);
        const asyncId = asyncIdentity.id;
        const asyncGroupId = asyncGroupIdentity.id;

        expect(web.getAttribute('aria-selected')).toBe('true');
        project.host.selectedValues = ['readme', 'unknown', 'readme'];
        await waitFor(() => treeNode(treeParts(root, '#project-tree'), 'readme').getAttribute('aria-selected') === 'true');
        project = treeParts(root, '#project-tree');
        expect(treeNode(project, 'web').getAttribute('aria-selected')).toBe('false');
        expect(project.host.selectedValues).toEqual(['readme', 'unknown']);
        expect(events).toEqual([]);

        const multi = treeParts(root, '#multi-tree');
        expect(multi.owner.getAttribute('aria-multiselectable')).toBe('true');
        expect(treeNode(multi, 'alpha').getAttribute('aria-selected')).toBe('true');
        expect(treeNode(multi, 'beta').getAttribute('aria-selected')).toBe('false');
        expect(treeNode(multi, 'gamma').getAttribute('aria-selected')).toBe('true');
        const plain = treeParts(root, '#plain-tree');
        expect(treeNode(plain, 'plain').hasAttribute('aria-selected')).toBe(false);

        expect(asyncIdentity.getAttribute('aria-busy')).toBe('true');
        expect(asyncIdentity.textContent).toContain('Loading children');
        const payload = dataIsland(project.host);
        const asyncPayload = requiredElement<HTMLElement>(payload.content, 'cem-tree-item[value="async"]');
        const late = document.createElement('cem-tree-item');
        late.setAttribute('value', 'late');
        late.setAttribute('label', 'Late child');
        asyncPayload.removeAttribute('loading');
        asyncPayload.append(late);
        await waitFor(() => treeParts(root, '#project-tree').nodes.some((node) => node.dataset.treeValue === 'late'));
        project = treeParts(root, '#project-tree');
        const currentAsync = treeNode(project, 'async');
        const currentAsyncGroup = ownedGroup(project, currentAsync);
        expect(currentAsync.id).toBe(asyncId);
        expect(currentAsync.hasAttribute('aria-busy')).toBe(false);
        expect(currentAsyncGroup.id).toBe(asyncGroupId);
        expect(currentAsync.getAttribute('aria-owns')).toBe(asyncGroupId);
        expect(currentAsyncGroup.hidden).toBe(false);
        expect(treeNode(project, 'late').getAttribute('aria-level')).toBe('3');
        expect(events).toEqual([]);
    });

    it('suppresses disabled subtrees and keeps programmatic expanded values silent without rewriting unknown facts', async () => {
        const root = await renderFixture();
        let project = treeParts(root, '#project-tree');
        const disabled = treeParts(root, '#disabled-tree');
        const events: string[] = [];
        const projectEvents: string[] = [];
        disabled.host.addEventListener('cem-tree-toggle', () => events.push('toggle'));
        disabled.host.addEventListener('cem-tree-activate', () => events.push('activate'));
        project.host.addEventListener('cem-tree-toggle', () => projectEvents.push('toggle'));
        project.host.addEventListener('cem-tree-activate', () => projectEvents.push('activate'));

        expect(disabled.nodes.every((node) => node.getAttribute('aria-disabled') === 'true')).toBe(true);
        expect(disabled.nodes.every((node) => node.tabIndex === -1)).toBe(true);
        expect(treeNode(project, 'api').getAttribute('aria-disabled')).toBe('true');
        expect(treeNode(project, 'archived').getAttribute('aria-disabled')).toBe('true');
        expect(treeNode(project, 'old').getAttribute('aria-disabled')).toBe('true');
        treeNode(disabled, 'locked').click();
        treeNode(disabled, 'locked-child').click();
        await nextRenderFrame();
        expect(events).toEqual([]);
        expect(disabled.host.expandedValues).toEqual(['locked']);

        treeNode(project, 'components').focus();
        expect(document.activeElement).toBe(treeNode(project, 'components'));
        project.host.expandedValues = ['apps', 'unknown', 'apps'];
        await waitFor(() =>
            treeNode(treeParts(root, '#project-tree'), 'source').getAttribute('aria-expanded') === 'false',
        );
        await nextRenderFrame();
        project = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(project, 'source'));
        expect(project.host.expandedValues).toEqual(['apps', 'unknown']);
        expect(project.host.getAttribute('expanded-values')).toBe('apps unknown');
        expect(projectEvents).toEqual([]);
    });

    it('keeps hover, focus-visible, selected-active paint on the exact node with stable geometry and transient silence', async () => {
        const root = await renderFixture();
        let parts = treeParts(root, '#project-tree');
        const node = treeNode(parts, 'web');
        const wrapper = node.parentElement;
        if (!(wrapper instanceof HTMLElement)) throw new Error('Expected tree node wrapper');
        await userEvent.hover(requiredElement<HTMLButtonElement>(root, '[data-tree-focus-start]'));
        const pointerEvents: string[] = [];
        const componentEvents: string[] = [];
        node.addEventListener('pointerenter', (event) => pointerEvents.push(`pointerenter:${event.isTrusted}`));
        node.addEventListener('pointerleave', (event) => pointerEvents.push(`pointerleave:${event.isTrusted}`));
        parts.host.addEventListener('cem-tree-toggle', () => componentEvents.push('toggle'));
        parts.host.addEventListener('cem-tree-activate', () => componentEvents.push('activate'));
        const baseline = transientSnapshot(parts.host, node, wrapper);
        const wrapperBackground = getComputedStyle(wrapper).backgroundColor;

        await userEvent.hover(node);
        await nextRenderFrame();
        const hovered = transientSnapshot(parts.host, node, wrapper);
        expect(node.matches(':hover')).toBe(true);
        expect(hovered.backgroundColor).toBe(
            resolveTokenColor(node, '--cem-content-interaction-selected-hover-background'),
        );
        expect(hovered.color).toBe(resolveTokenColor(node, '--cem-content-interaction-selected-hover-text'));
        expect(getComputedStyle(wrapper).backgroundColor).toBe(wrapperBackground);
        expectStableTransient(hovered, baseline);

        requiredElement<HTMLButtonElement>(root, '[data-tree-focus-start]').focus();
        await userEvent.tab();
        parts = treeParts(root, '#project-tree');
        expect(document.activeElement).toBe(treeNode(parts, 'web'));
        expect(treeNode(parts, 'web').matches(':focus-visible')).toBe(true);
        expect(node.matches(':hover')).toBe(true);
        const focused = transientSnapshot(parts.host, node, wrapper);
        expect(focused.outlineWidth).toBe(resolveTokenLength(node, '--cem-stroke-focus'));
        expectStableTransient(focused, baseline);

        await userEvent.keyboard('[Space>]');
        expect(node.matches(':active')).toBe(true);
        const active = transientSnapshot(parts.host, node, wrapper);
        expect(active.backgroundColor).toBe(
            resolveTokenColor(node, '--cem-content-interaction-selected-active-background'),
        );
        expectStableTransient(active, baseline);
        expect(componentEvents).toEqual([]);
        await userEvent.keyboard('[/Space]');
        expect(componentEvents).toEqual(['activate']);

        await userEvent.unhover(node);
        await nextRenderFrame();
        expect(pointerEvents).toEqual(['pointerenter:true', 'pointerleave:true']);
        expect(parts.host.getAttribute('selected-values')).toBe('readme web');
        expect(parts.host.getAttribute('expanded-values')).toBe('apps source async');
    });

    async function renderFixture(): Promise<HTMLElement> {
        harness = createComponentHarness({ runtime });
        const root = await harness.render(treeContractFixture);
        await waitFor(() => root.querySelectorAll('cem-tree > .cem-tree[role="tree"]').length === 4);
        return root;
    }
});

function treeParts(root: ParentNode, selector: string): TreeParts {
    const host = requiredElement<TestCemTree>(root, selector);
    const owner = requiredElement<HTMLElement>(host, ':scope > .cem-tree[role="tree"]');
    const nodes = Array.from(owner.querySelectorAll<HTMLButtonElement>('button.cem-tree__node[role="treeitem"]'));
    const groups = Array.from(owner.querySelectorAll<HTMLElement>('.cem-tree__group[role="group"]'));
    return { host, owner, nodes, groups };
}

function treeNode(parts: TreeParts, value: string): HTMLButtonElement {
    const node = parts.nodes.find((candidate) => candidate.dataset.treeValue === value);
    if (!node) throw new Error(`Expected tree node ${value}`);
    return node;
}

function ownedGroup(parts: TreeParts, node: HTMLButtonElement): HTMLElement {
    const id = node.getAttribute('aria-owns');
    if (!id) throw new Error(`Expected ${node.dataset.treeValue} to own a group`);
    const group = parts.groups.find((candidate) => candidate.id === id);
    if (!group) throw new Error(`Expected owned group ${id}`);
    return group;
}

function dataIsland(host: HTMLElement): HTMLTemplateElement {
    return requiredElement<HTMLTemplateElement>(host, 'template[data-cem-island="instance"]');
}

async function waitFor(predicate: () => boolean, label = 'condition'): Promise<void> {
    for (let attempt = 0; attempt < 60; attempt += 1) {
        if (predicate()) return;
        await nextRenderFrame();
    }
    throw new Error(`Timed out waiting for ${label}`);
}

function requiredElement<T extends Element = HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector);
    if (!element) throw new Error(`Expected element matching ${selector}`);
    return element;
}

function rectTuple(element: Element): [number, number, number, number] {
    const rect = element.getBoundingClientRect();
    return [rect.x, rect.y, rect.width, rect.height];
}

function transientSnapshot(host: TestCemTree, node: HTMLButtonElement, wrapper: HTMLElement) {
    const style = getComputedStyle(node);
    return {
        hostAttributes: Array.from(host.attributes).map((attribute) => `${attribute.name}=${attribute.value}`).join('|'),
        nodeRect: rectTuple(node),
        wrapperRect: rectTuple(wrapper),
        backgroundColor: style.backgroundColor,
        color: style.color,
        outlineWidth: Number.parseFloat(style.outlineWidth),
    };
}

function expectStableTransient(
    actual: ReturnType<typeof transientSnapshot>,
    expected: ReturnType<typeof transientSnapshot>,
): void {
    expect(actual.hostAttributes).toBe(expected.hostAttributes);
    expect(actual.nodeRect).toEqual(expected.nodeRect);
    expect(actual.wrapperRect).toEqual(expected.wrapperRect);
}

function resolveTokenColor(owner: Element, token: string): string {
    const probe = document.createElement('span');
    probe.style.color = `var(${token})`;
    owner.append(probe);
    const value = getComputedStyle(probe).color;
    probe.remove();
    return value;
}

function resolveTokenLength(owner: Element, token: string): number {
    const probe = document.createElement('span');
    probe.style.inlineSize = `var(${token})`;
    owner.append(probe);
    const value = Number.parseFloat(getComputedStyle(probe).inlineSize);
    probe.remove();
    return value;
}
