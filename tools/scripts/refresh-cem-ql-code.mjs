// Refresh JS code-delivery assets for an existing, verified optimized full runtime.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cemQlCodeAssets } from './cem-ql-code-assets.mjs';
import { cemQlWasmFactory, cemQlWasmFactoryTypes } from './cem-ql-wasm-factory.mjs';

export function refreshCemQlCode(root) {
    const output=resolve(root,'packages/cem_ql/dist/wasm');
    const bytes=readFileSync(resolve(output,'cem_ql_bg.wasm'));
    const report=JSON.parse(readFileSync(resolve(root,'dist/reports/wasm-optimization/cem-ql.json'),'utf8'));
    assert.equal(createHash('sha256').update(bytes).digest('hex'),report.outputSha256,'full runtime differs from its optimization receipt');
    assert.equal(bytes.length,report.outputBytes);
    const bindings=readFileSync(resolve(output,'cem_ql.js'),'utf8');
    const assets=cemQlCodeAssets(bytes,bindings);
    writeFileSync(resolve(output,'cem_ql_code.js'),assets.js);
    writeFileSync(resolve(output,'cem_ql_code.d.ts'),assets.dts);
    writeFileSync(resolve(output,'cem_ql_factory.js'),cemQlWasmFactory(bindings,true));
    writeFileSync(resolve(output,'cem_ql_factory.d.ts'),cemQlWasmFactoryTypes);
    return assets.identity;
}
if(process.argv[1] && resolve(process.argv[1])===import.meta.filename) {
    console.log('Refreshed full-runtime code pins:',refreshCemQlCode(resolve(import.meta.dirname,'../..')).wasmSha256);
}
