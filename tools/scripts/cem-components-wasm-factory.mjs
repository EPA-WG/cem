import ts from 'typescript';

/** Isolate the pinned small ABI's bindings; ownership policy lives in cem-elements. */
export function componentWasmFactory(source) {
    const ast = ts.createSourceFile('cem_ql.js', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.JS);
    const edits = [], functions = [], classes = []; let initializer = false, finalizers = 0;
    for (const statement of ast.statements) {
        if (ts.isImportDeclaration(statement)) throw new Error('Component bindings cannot share imports');
        if (ts.isExportDeclaration(statement)) {
            if (statement.getText(ast) !== 'export { initSync, __wbg_init as default };') throw new Error('Unexpected component initializer');
            edits.push([statement.getStart(ast), statement.end, '']); initializer = true;
        } else for (const modifier of statement.modifiers ?? []) if (modifier.kind === ts.SyntaxKind.ExportKeyword) {
            if (!ts.isFunctionDeclaration(statement) && !ts.isClassDeclaration(statement)) throw new Error('Unexpected component binding');
            edits.push([modifier.getStart(ast), modifier.end, '']);
            (ts.isFunctionDeclaration(statement) ? functions : classes).push(statement.name.text);
        }
    }
    function visit(node) {
        if (ts.isNewExpression(node) && node.expression.getText(ast) === 'FinalizationRegistry') {
            const callback = node.arguments?.[0];
            if (!callback || !ts.isArrowFunction(callback) || !ts.isCallExpression(callback.body)
                || !callback.body.expression.getText(ast).startsWith('wasm.')) throw new Error('Unexpected component finalizer');
            edits.push([callback.body.getStart(ast), callback.body.end, `wasm === undefined ? undefined : ${callback.body.getText(ast)}`]); finalizers++;
        }
        ts.forEachChild(node, visit);
    }
    visit(ast);
    if (!initializer || finalizers !== 1 || JSON.stringify(classes) !== '["CompiledDeclarationOwner"]'
        || JSON.stringify(functions.sort()) !== '["compiledDeclarationInventory","renderCompiledApplication"]'
        || !source.includes('let wasmModule, wasmInstance, wasm;') || source.includes('cachedDataViewMemory0')
        || !source.includes('cachedUint8ArrayMemory0')) throw new Error('Unsupported component WASM binding shape');
    for (const [start, end, value] of edits.sort((a,b) => b[0]-a[0])) source = source.slice(0,start)+value+source.slice(end);
    source = source.replace('/* @ts-self-types="./cem_ql.d.ts" */','');
    return `/* @ts-self-types="./cem_ql_factory.d.ts" */
// Generated binding isolation only. Shared lifecycle policy: cem-elements.
export function createCemComponentWasmBindings() {
${source}
    let retired = false, initializing = false;
    const active = () => { if (retired || initializing || wasm === undefined) throw new Error('cem.component.generation.inactive'); };
    return Object.freeze({
        async initialize(module) {
            if (retired || initializing || wasm !== undefined || !(module instanceof WebAssembly.Module)) throw new Error('cem.component.generation.initialize');
            initializing = true;
            try { await __wbg_init({ module_or_path: module }); }
            catch (error) { retired = true; cachedUint8ArrayMemory0 = null; wasmModule = wasmInstance = wasm = undefined; throw error; }
            finally { initializing = false; }
        },
        open(bytes, metadata, pin) { active(); return new CompiledDeclarationOwner(bytes, metadata, pin); },
        inventory() { active(); return compiledDeclarationInventory(); },
        memory() { active(); return wasm.memory; },
        release() {
            active(); const census = JSON.parse(compiledDeclarationInventory());
            if (census.liveDeclarationOwners !== 0 || census.stylesheetConsumerLeases !== 0) throw new Error('cem.component.generation.owned');
            retired = true; cachedUint8ArrayMemory0 = null;
            wasmModule = wasmInstance = wasm = undefined;
        },
    });
}
`;
}
export const componentWasmFactoryTypes = `import type { CompiledDeclarationOwner } from './cem_ql.js';
export declare function createCemComponentWasmBindings(): {
    initialize(module: WebAssembly.Module): Promise<void>;
    open(bytes: Uint8Array, metadata: string, pin: string): CompiledDeclarationOwner;
    inventory(): string;
    memory(): WebAssembly.Memory;
    release(): void;
};
`;
