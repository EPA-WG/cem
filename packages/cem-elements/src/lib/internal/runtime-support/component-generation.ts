import type { CemCompiledDeclarationMetadata } from './cem-ql-render.js';
import type { CemModuleUrlContextWire } from './module-url-resolution.js';
import type { CemOwnedStylesheet, CemStylesheetConsumerLease } from '../../declaration-style-ownership.js';

/** Internal wasm-bindgen values never escape the generation adapter. */
export interface CemComponentNativeDeclaration {
    describe(): string;
    renderJson(bytes: Uint8Array, uri: string, binding: string): string;
    renderInputs(bytes: Uint8Array, bindings: string): string;
    stylesheet(consumer: string, index: number, context: string): string;
    releaseConsumer(consumer: string): number;
    dispose(): void;
    free(): void;
}
export interface CemComponentWasmBindings {
    initialize(module: WebAssembly.Module): Promise<void>;
    open(bytes: Uint8Array, metadata: string, pin: string): CemComponentNativeDeclaration;
    inventory(): string;
    memory(): WebAssembly.Memory;
    release(): void;
}
export interface CemComponentInventory {
    contract: 'cem-component-owner-inventory/1';
    revision: number;
    liveDeclarationOwners: number;
    stylesheetConsumerLeases: number;
}
export interface CemComponentDeclaration {
    /** A control-metadata copy; never a native data/AST projection. */
    metadata(): CemCompiledDeclarationMetadata;
    renderJson(bytes: Uint8Array, uri: string, binding: string): string;
    /** One opaque multi-root CEMV artifact; routing is control metadata only. */
    renderInputs(bytes: Uint8Array, bindings: readonly CemComponentInputBinding[]): string;
    installStylesheets(consumer: string, context: CemModuleUrlContextWire, lease: CemStylesheetConsumerLease): { dispose(): void };
    dispose(): void;
}
export type CemComponentInputBinding =
    | { target: 'attribute'; name: string; index: number }
    | { target: 'slice'; name: string; index: number; attribute?: string };

/** Selected-profile ownership only: no full WASM facade, UI rendering or source fallback. */
export async function createCemComponentGeneration(module: WebAssembly.Module, bindings: CemComponentWasmBindings) {
    await bindings.initialize(module);
    let retired = false, closing = false, calls = 0;
    const declarations = new Set<CemComponentDeclaration>();
    const active = () => { if (retired || closing) throw new Error('cem.component.generation.retired'); };
    const invoke = <T>(run: () => T): T => {
        active(); calls++;
        try { return run(); } finally { calls--; }
    };
    const idle = () => { if (calls) throw new Error('cem.component.generation.busy'); };
    const census = (): CemComponentInventory => {
        const value: CemComponentInventory = JSON.parse(bindings.inventory());
        if (value.contract !== 'cem-component-owner-inventory/1'
            || !Number.isSafeInteger(value.liveDeclarationOwners) || value.liveDeclarationOwners < 0
            || !Number.isSafeInteger(value.stylesheetConsumerLeases) || value.stylesheetConsumerLeases < 0) {
            throw new Error('cem.component.generation.inventory');
        }
        return Object.freeze(value);
    };
    const inventory = () => { active(); return census(); };
    const generation = {
        inventory,
        memory(): WebAssembly.Memory { active(); return bindings.memory(); },
        open(bytes: Uint8Array, metadata: string, pin: string): CemComponentDeclaration {
            active();
            if (bytes.byteLength > 1024 * 1024 || metadata.length > 1024 * 1024 || pin.length > 256) throw new Error('cem.component.declaration.input-limit');
            const native = invoke(() => bindings.open(bytes, metadata, pin));
            type Descriptor = { metadata: CemCompiledDeclarationMetadata; stylesheets: {index: number; scope: string | null}[] };
            let descriptor: Descriptor;
            try { descriptor = JSON.parse(native.describe()); }
            catch (error) { native.dispose(); native.free(); throw error; }
            let closed = false, sequence = 0;
            const installations = new Set<{ dispose(): void }>();
            const current = () => { active(); if (closed) throw new Error('cem.component.declaration.disposed'); };
            const declaration: CemComponentDeclaration = Object.freeze({
                metadata() { current(); return structuredClone(descriptor.metadata); },
                renderJson(data: Uint8Array, uri: string, binding: string) {
                    current();
                    if (data.byteLength > 1024 * 1024 || uri.length > 4096 || binding.length > 256) throw new Error('cem.component.declaration.input-limit');
                    return invoke(() => native.renderJson(data, uri, binding));
                },
                renderInputs(data: Uint8Array, inputs: readonly CemComponentInputBinding[]) {
                    current();
                    if (data.byteLength > 1024 * 1024 || inputs.length > 64) throw new Error('cem.component.declaration.input-limit');
                    const routing = JSON.stringify(inputs);
                    if (routing.length > 16 * 1024) throw new Error('cem.component.declaration.input-limit');
                    current(); // Serializing host routing may invoke a reentrant callback.
                    return invoke(() => native.renderInputs(data, routing));
                },
                installStylesheets(consumer: string, context: CemModuleUrlContextWire, lease: CemStylesheetConsumerLease) {
                    current();
                    if (typeof consumer !== 'string' || consumer.length > 200 || !consumer || sequence >= Number.MAX_SAFE_INTEGER) throw new Error('cem.component.stylesheet.consumer');
                    const attempt = `${consumer}:${++sequence}`;
                    let released = false;
                    const installation = { dispose() {
                        if (released) return;
                        released = true; installations.delete(installation);
                        lease.signal.removeEventListener('abort', installation.dispose);
                        try { native.releaseConsumer(attempt); } finally { lease.release(); }
                    } };
                    installations.add(installation);
                    lease.signal.addEventListener('abort', installation.dispose, {once:true});
                    try {
                        if (lease.signal.aborted) throw new Error('cem.component.stylesheet.stale');
                        const control = JSON.stringify(context);
                        const outputs: CemOwnedStylesheet[] = descriptor.stylesheets.map(sheet => ({
                            index: sheet.index,
                            scope: sheet.scope === null ? {kind:'private'} : {kind:'shared', name:sheet.scope},
                            output: JSON.parse(invoke(() => native.stylesheet(attempt, sheet.index, control))),
                        }));
                        current();
                        if (released || lease.signal.aborted || !lease.commit(outputs, installation.dispose)) throw new Error('cem.component.stylesheet.stale');
                        current();
                        if (released || lease.signal.aborted) throw new Error('cem.component.stylesheet.stale');
                        return Object.freeze(installation);
                    } catch (error) { installation.dispose(); throw error; }
                },
                dispose() {
                    if (closed) return;
                    idle(); closed = true;
                    // Native owners are freed while the binding closure still exists.
                    const errors: unknown[] = [];
                    for (const installation of [...installations]) {
                        try { installation.dispose(); } catch (error) { errors.push(error); }
                    }
                    native.dispose(); native.free(); declarations.delete(declaration);
                    if (errors.length) throw new AggregateError(errors, 'cem.component.stylesheet.cleanup');
                },
            });
            declarations.add(declaration);
            return declaration;
        },
        dispose() {
            if (retired || closing) return;
            active(); idle();
            // Raw bindings cannot hide a foreign native root behind our owner set.
            if (inventory().liveDeclarationOwners !== declarations.size) throw new Error('cem.component.generation.foreign-owner');
            closing = true;
            try {
                const errors: unknown[] = [];
                for (const declaration of [...declarations]) {
                    try { declaration.dispose(); } catch (error) { errors.push(error); }
                }
                const after = census();
                if (after.liveDeclarationOwners || after.stylesheetConsumerLeases) throw new Error('cem.component.generation.owned');
                bindings.release(); retired = true;
                if (errors.length) throw new AggregateError(errors, 'cem.component.generation.cleanup');
            } finally { closing = false; }
        },
    };
    return Object.freeze(generation);
}
