import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { cemQlWasmFactory } from './cem-ql-wasm-factory.mjs';
import ts from 'typescript';

const root = resolve(import.meta.dirname, '../..');
const transport = process.argv.includes('--resource-transport');
if (process.argv.slice(2).some(arg => arg !== '--resource-transport')) throw Error('Unknown query probe option');
const output = resolve(root, transport ? 'dist/fixtures/query-resource-wasm' : 'dist/fixtures/query-source-wasm');
const target = transport ? 'dist/target/cem_ql' : 'dist/target/cem_ql_component';
const run = (command, args, env = process.env) => {
    const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit' });
    if (result.status !== 0) throw result.error ?? Error(`${command} failed (${result.status})`);
};
// Native source authoring precedes WASM. No schema/query parsing is needed to
// restore the portable declaration in the probe. All generated files stay in dist.
mkdirSync(output, { recursive: true });
run('cargo', ['test', '--locked', '--offline', '-p', 'cem-ql', '--test', 'query_validator_capsule',
    'native_emitted_wasm_handoff_fixture_has_checked_source_and_independent_pins'],
    { ...process.env, CEM_QUERY_HANDOFF_FIXTURE_DIR: output });
run('cargo', ['build', '--locked', '--offline', '-p', 'cem-ql', '--example', 'query_source_probe',
    '--release', '--target', 'wasm32-unknown-unknown', '--target-dir', target,
    ...(transport ? ['--features', 'query-source-runtime'] : ['--no-default-features', '--features', 'component-runtime,query-source-runtime'])]);
const version = spawnSync('wasm-bindgen', ['--version'], { encoding: 'utf8' });
if (version.status !== 0 || version.stdout.trim() !== 'wasm-bindgen 0.2.122') throw Error('wasm-bindgen 0.2.122 is required');
run('wasm-bindgen', [resolve(root, target, 'wasm32-unknown-unknown/release/examples/query_source_probe.wasm'),
    '--target', 'web', '--out-dir', output, '--out-name', 'query_source_probe',
    '--remove-name-section', '--remove-producers-section']);
writeFileSync(resolve(output, 'package.json'), '{"type":"module"}\n');
if (transport) {
    writeFileSync(resolve(output, 'query_factory.js'), cemQlWasmFactory(readFileSync(resolve(output, 'query_source_probe.js'), 'utf8')));
    // Compile the exact shared store in isolation: another package's dist clean
    // must not race this fixture or become a hidden acceptance prerequisite.
    const store = readFileSync(resolve(root, 'packages/cem-elements/src/lib/committed-checkpoint.ts'), 'utf8');
    writeFileSync(resolve(output, 'checkpoint-store.js'), ts.transpileModule(store, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText);
}
run(process.execPath, [transport ? 'packages/cem_ql/tests/query-resource-wasm.mjs' : 'packages/cem_ql/tests/query-source-wasm.mjs', output]);
