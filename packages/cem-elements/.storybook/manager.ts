import { createElement, useEffect, useRef, useState } from 'react';
import { addons, types, useGlobals } from 'storybook/manager-api';
import { IconButton, PopoverProvider } from 'storybook/internal/components';
import type { CemElementRuntime } from '../src/index.js';
import { themeMode, themeModes } from './theme.js';

type ThemeSelect = HTMLElement & { value: string };
let installation: Promise<CemElementRuntime> | undefined;

function installThemeControl(): Promise<CemElementRuntime> {
    return installation ??= (async () => {
        const assets = new URL('./cem-manager-assets/', document.baseURI);
        const stylesheet = document.createElement('link');
        stylesheet.rel = 'stylesheet';
        stylesheet.href = new URL('theme/lib/css/cem-combined.css', assets).href;
        document.head.append(stylesheet);
        const runtimeUrl = new URL('runtime/index.js', assets).href;
        const { installCemElementRuntime } = await import(/* @vite-ignore */ runtimeUrl) as typeof import('../src/index.js');
        const runtime = installCemElementRuntime(window);
        const response = await fetch(new URL('components/cem-select/cem-select.xhtml', assets));
        if (!response.ok) throw new Error(`Theme declaration: HTTP ${response.status}`);
        const template = document.createElement('template');
        template.innerHTML = await response.text();
        const declaration = template.content.firstElementChild as HTMLElement;
        declaration.hidden = true;
        declaration.dataset.cemStorybookDeclaration = 'manager-theme-select';
        document.body.prepend(declaration);
        await runtime.whenDeclarationSettled(declaration);
        const errors = runtime.diagnosticsFor(declaration).filter(item => ['error', 'fatal'].includes(item.severity));
        if (errors.length) throw new Error(errors.map(item => item.message).join('\n'));
        await customElements.whenDefined('cem-select');
        return runtime;
    })();
}

function ThemeControl() {
    const [globals, updateGlobals] = useGlobals();
    const mode = themeMode(globals.cemTheme);
    const host = useRef<HTMLDivElement>(null);
    const select = useRef<ThemeSelect | null>(null);
    const current = useRef({ mode, updateGlobals });
    current.current = { mode, updateGlobals };
    const [error, setError] = useState('');
    useEffect(() => {
        let alive = true;
        let cleanup: (() => void) | undefined;
        void installThemeControl().then(async runtime => {
            if (!alive || !host.current) return;
            const control = document.createElement('cem-select') as ThemeSelect;
            control.setAttribute('label', 'Theme');
            control.setAttribute('value', current.current.mode);
            for (const [value, label] of themeModes) {
                const option = document.createElement('cem-option');
                option.setAttribute('value', value);
                option.textContent = label;
                control.append(option);
            }
            const change = () => {
                const value = themeMode(control.value);
                if (value !== current.current.mode) current.current.updateGlobals({ cemTheme: value });
            };
            control.addEventListener('change', change);
            host.current.append(control);
            select.current = control;
            cleanup = () => { control.removeEventListener('change', change); control.remove(); select.current = null; };
            await runtime.whenRenderSettled(control);
            if (alive) control.querySelector<HTMLElement>('[role="combobox"]')?.focus();
        }).catch(reason => { if (alive) setError(String(reason)); });
        return () => { alive = false; cleanup?.(); };
    }, []);
    useEffect(() => {
        if (select.current && select.current.value !== mode) select.current.value = mode;
    }, [mode]);
    return createElement('div', {
        ref: host, 'data-cem-theme-tool': '', 'data-theme': `cem-theme-${mode}`,
        style: { width: '220px', padding: '12px', minHeight: '260px' },
    }, error ? createElement('span', { role: 'alert' }, error) : null);
}

function ThemeTool() {
    return createElement(PopoverProvider, {
        ariaLabel: 'Theme settings', placement: 'bottom',
        popover: createElement(ThemeControl),
        children: createElement(IconButton, { title: 'Theme', 'aria-label': 'Theme' }, 'Theme'),
    });
}

addons.register('cem/theme', () => {
    addons.add('cem/theme/tool', { title: 'Theme', type: types.TOOL, render: ThemeTool });
});
