import ts from 'typescript';

/** Pin the wasm-bindgen module shape at build time. Each invocation owns every
 * binding/cache/registry closure; no eval, blob module or per-generation URL. */
export function cemQlWasmFactory(source) { return ownedWasmFactory(source, 'cem_ql', 'CemQl'); }

export function ownedWasmFactory(source, stem, name) {
    if (!['cem_ql', 'cem_ml'].includes(stem) || !['CemQl', 'CemMl'].includes(name)) throw new Error('Unknown owned WASM module');
    const ast = ts.createSourceFile(`${stem}.js`, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
    const edits = [], functions = [];
    let finalizers = 0, initializer = false;
    for (const statement of ast.statements) {
        if (ts.isImportDeclaration(statement)) throw new Error('Owned WASM factory cannot contain shared binding imports');
        if (ts.isExportDeclaration(statement)) {
            if (statement.getText(ast) !== 'export { initSync, __wbg_init as default };') throw new Error('Unexpected WASM binding exports');
            edits.push([statement.getStart(ast), statement.end, '']); initializer = true;
        } else for (const modifier of statement.modifiers ?? []) if (modifier.kind === ts.SyntaxKind.ExportKeyword) {
            if (!ts.isFunctionDeclaration(statement) && !ts.isClassDeclaration(statement)) throw new Error('Unexpected WASM binding declaration');
            edits.push([modifier.getStart(ast), modifier.end, '']);
            if (ts.isFunctionDeclaration(statement)) functions.push(statement.name.text);
        }
    }
    const visit = node => {
        if (ts.isNewExpression(node) && node.expression.getText(ast) === 'FinalizationRegistry') {
            const callback = node.arguments?.[0];
            if (!callback || !ts.isArrowFunction(callback) || !ts.isCallExpression(callback.body)
                || !callback.body.expression.getText(ast).startsWith('wasm.')) throw new Error('Unexpected WASM finalizer');
            edits.push([callback.body.getStart(ast), callback.body.end, `wasm === undefined ? undefined : ${callback.body.getText(ast)}`]);
            finalizers++;
        }
        ts.forEachChild(node, visit);
    };
    visit(ast);
    if (!initializer || finalizers !== 2 || functions.length < 100 || !source.includes('let wasmModule, wasmInstance, wasm;')
        || !source.includes('cachedDataViewMemory0') || !source.includes('cachedUint8ArrayMemory0')) throw new Error('Unsupported wasm-bindgen owned-factory shape');
    for (const [start, end, value] of edits.sort((a, b) => b[0] - a[0])) source = source.slice(0, start) + value + source.slice(end);
    source = source.replace(`/* @ts-self-types="./${stem}.d.ts" */`, '');
    return `/* @ts-self-types="./${stem}_factory.d.ts" */
// Generated from pinned wasm-bindgen bindings. Owned lifecycle controls only.
// One immutable code module per installed asset; never cache an Instance,
// Memory, binding closure, custom input or an unbounded set of URLs.
let defaultModule;
function loadDefaultModule() {
    return defaultModule ??= fetch(new URL('${stem}_bg.wasm', import.meta.url)).then(async response => {
        if (!response.ok) throw new Error('cem.runtime.generation.load');
        if (typeof WebAssembly.compileStreaming === 'function' && response.headers.get('content-type')?.split(';')[0].trim() === 'application/wasm')
            return WebAssembly.compileStreaming(response);
        return WebAssembly.compile(await response.arrayBuffer());
    }).catch(error => { defaultModule = undefined; throw error; });
}
function cancellable(input, signal) {
    if (!signal) return input;
    signal.throwIfAborted();
    return new Promise((resolve, reject) => {
        const cancel = () => reject(signal.reason);
        signal.addEventListener('abort', cancel, { once: true });
        Promise.resolve(input).then(resolve, reject).finally(() => signal.removeEventListener('abort', cancel));
    });
}
export function create${name}WasmGeneration() {
${source}
    const confirmationOf = CommandNativeRetirement.prototype.confirmation;
    const abandonPermit = CommandNativeRetirement.prototype.__destroy_into_raw;
    const freePermit = CommandNativeRetirement.prototype.free;
    const calls = Object.freeze({ ${functions.join(', ')} });
    let initialization, retirement, initialized = false, dead = false;
    const alive = () => { if (dead) throw new Error('cem.runtime.generation.retired'); };
    const ready = () => { alive(); if (!initialized) throw new Error('cem.runtime.generation.unavailable'); };
    const release = () => {
        dead = true;
        cachedDataViewMemory0 = null; cachedUint8ArrayMemory0 = null;
        wasm = undefined; wasmInstance = undefined; wasmModule = undefined;
    };
    return Object.freeze({
        async initialize(input, signal) {
            alive();
            initialization ??= (async () => {
                signal?.throwIfAborted();
                const module = await cancellable(input === undefined ? loadDefaultModule() : input, signal);
                alive(); signal?.throwIfAborted();
                await __wbg_init({ module_or_path: module });
                alive(); signal?.throwIfAborted(); initialized = true;
            })().catch(error => { release(); throw error; });
            await initialization;
        },
        memory() { return initialized && !dead ? wasm.memory : undefined; },
        call(name, args) {
            ready();
            if (!Object.hasOwn(calls, name)) throw new Error('cem.runtime.generation.entrypoint');
            return Reflect.apply(calls[name], undefined, args);
        },
        prepareRetirement(expectedRevision) {
            ready();
            if (retirement) throw new Error('cem.runtime.generation.retirement-pending');
            closeCommandNativeAdmission();
            const text = commandNativeResourceInventory();
            if (text.length > 16384) throw new Error('cem.runtime.generation.inventory');
            const inventory = JSON.parse(text);
            if (inventory.revision !== expectedRevision) throw new Error('cem.runtime.generation.inventory-stale');
            const permit = prepareCommandNativeRetirement(expectedRevision);
            try {
                const confirmation = JSON.parse(Reflect.apply(confirmationOf, permit, []));
                retirement = { permit, confirmation };
                return Object.freeze({ ...confirmation });
            } catch (error) { Reflect.apply(freePermit, permit, []); throw error; }
        },
        abortRetirement() {
            ready();
            if (!retirement) return;
            const permit = retirement.permit; retirement = undefined;
            Reflect.apply(freePermit, permit, []);
        },
        dispose(current = () => true) {
            ready();
            let permit;
            try {
                if (retirement) permit = retirement.permit;
                else {
                    closeCommandNativeAdmission();
                    const text = commandNativeResourceInventory();
                    if (text.length > 16384) throw new Error('cem.runtime.generation.inventory');
                    permit = prepareCommandNativeRetirement(JSON.parse(text).revision);
                }
                const confirmation = JSON.parse(Reflect.apply(confirmationOf, permit, []));
                if (current() !== true) throw new Error('cem.runtime.generation.ownership-stale');
                // Keep native mutation frozen while dropping its sole JS owner.
                Reflect.apply(abandonPermit, permit, []);
                retirement = undefined; release();
                return Object.freeze(confirmation);
            } catch (error) {
                retirement = undefined;
                if (!dead && permit) Reflect.apply(freePermit, permit, []);
                throw error;
            }
        },
    });
}
`;
}

export const cemQlWasmFactoryTypes = ownedWasmFactoryTypes('cem_ql', 'CemQl');

export function ownedWasmFactoryTypes(stem, name) { return `import type { InitInput } from './${stem}.js';
export interface ${name}WasmGeneration {
    initialize(input?: InitInput, signal?: AbortSignal): Promise<void>;
    memory(): WebAssembly.Memory | undefined;
    call(name: string, args: unknown[]): unknown;
    prepareRetirement(expectedRevision: number): Readonly<{ schemaVersion: 1; frozen: true; inventoryRevision: number; frozenRevision: number; replacementAllowed: false }>;
    abortRetirement(): void;
    dispose(current?: () => boolean): Readonly<{ schemaVersion: 1; frozen: true; inventoryRevision: number; frozenRevision: number; replacementAllowed: false }>;
}
export declare function create${name}WasmGeneration(): ${name}WasmGeneration;
`;
}
