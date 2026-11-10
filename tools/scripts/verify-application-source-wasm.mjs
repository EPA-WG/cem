import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import ts from 'typescript';
import { cemQlWasmFactory } from './cem-ql-wasm-factory.mjs';

const root = resolve(import.meta.dirname, '../..');
const transport = process.argv.includes('--resource-transport');
const selectedCore = process.argv.includes('--selected-core');
if (selectedCore && !transport) throw Error('--selected-core requires --resource-transport');
const core = resolve(root, 'packages/cem_ql/dist/wasm-selected-application');
if (selectedCore && ['cem_ql_bg.wasm', 'cem_ql_factory.js'].some(file => !existsSync(resolve(core, file)))) {
    throw Error('Build the selected cart first with cem_ql:test:selected-application-runtime');
}
const output = resolve(root, selectedCore ? 'dist/fixtures/selected-application-handoff' : 'dist/fixtures/application-source-wasm');
const target = transport ? 'dist/target/cem_ql' : 'dist/target/cem_ql_component';
const run = (command, args, env = process.env) => {
    const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit' });
    if (result.status !== 0) throw result.error ?? Error(`${command} failed (${result.status})`);
};
mkdirSync(output, { recursive: true });
run('cargo', ['test', '--locked', '--offline', '-p', 'cem-ql', '--test', 'datatype_assembly_runtime',
    'source_handoff_reimports_original_mixed_consumers_with_fresh_explicit_owners'],
    { ...process.env, CEM_APPLICATION_HANDOFF_FIXTURE_DIR: output });
run('cargo', ['test', '--locked', '--offline', '-p', 'cem-ql', '--features', 'application-source-runtime', '--test', 'application_handoff'],
    { ...process.env, CEM_APPLICATION_HANDOFF_FIXTURE_DIR: output });
run('cargo', ['build', '--locked', '--offline', '-p', 'cem-ql', '--example', 'application_source_probe',
    '--release', '--target', 'wasm32-unknown-unknown', '--target-dir', target,
    ...(transport ? ['--features','application-source-runtime'] : ['--no-default-features', '--features', 'component-runtime,application-source-runtime'])]);
const version = spawnSync('wasm-bindgen', ['--version'], { encoding: 'utf8' });
if (version.status !== 0 || version.stdout.trim() !== 'wasm-bindgen 0.2.122') throw Error('wasm-bindgen 0.2.122 is required');
run('wasm-bindgen', [resolve(root, target, 'wasm32-unknown-unknown/release/examples/application_source_probe.wasm'),
    '--target', 'web', '--out-dir', output, '--out-name', 'application_source_probe',
    '--remove-name-section', '--remove-producers-section']);
writeFileSync(resolve(output, 'package.json'), '{"type":"module"}\n');
run(process.execPath, ['packages/cem_ql/tests/application-source-wasm.mjs', output]);

if (transport) {
    writeFileSync(resolve(output,'application_factory.js'),cemQlWasmFactory(readFileSync(resolve(output,'application_source_probe.js'),'utf8')));
    const hostModule = name => ts.transpileModule(readFileSync(resolve(root, 'packages/cem-elements/src/lib', name), 'utf8'),
        { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext } }).outputText;
    writeFileSync(resolve(output, 'checkpoint-store.js'), hostModule('committed-checkpoint.ts'));
    writeFileSync(resolve(output, 'checkpoint-resume.js'), hostModule('checkpoint-resume.ts')
        .replace('./committed-checkpoint.js', './checkpoint-store.js')
        .replace('./internal/runtime-support/native-generation.js', '../../../packages/cem_ql/runtime/native-generation.mjs'));
    run(process.execPath, ['packages/cem_ql/tests/application-resource-wasm.mjs', output,
        ...(selectedCore ? [core] : [])]);
}
