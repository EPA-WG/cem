import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const root = resolve(import.meta.dirname, '../..');
const output = resolve(root, 'dist/fixtures/application-source-wasm');
const target = 'dist/target/cem_ql_component';
const run = (command, args, env = process.env) => {
    const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit' });
    if (result.status !== 0) throw result.error ?? Error(`${command} failed (${result.status})`);
};
mkdirSync(output, { recursive: true });
run('cargo', ['test', '--locked', '--offline', '-p', 'cem-ql', '--test', 'datatype_assembly_runtime',
    'source_handoff_reimports_original_mixed_consumers_with_fresh_explicit_owners'],
    { ...process.env, CEM_APPLICATION_HANDOFF_FIXTURE_DIR: output });
run('cargo', ['build', '--locked', '--offline', '-p', 'cem-ql', '--example', 'application_source_probe',
    '--release', '--target', 'wasm32-unknown-unknown', '--target-dir', target,
    '--no-default-features', '--features', 'component-runtime,application-source-runtime']);
const version = spawnSync('wasm-bindgen', ['--version'], { encoding: 'utf8' });
if (version.status !== 0 || version.stdout.trim() !== 'wasm-bindgen 0.2.122') throw Error('wasm-bindgen 0.2.122 is required');
run('wasm-bindgen', [resolve(root, target, 'wasm32-unknown-unknown/release/examples/application_source_probe.wasm'),
    '--target', 'web', '--out-dir', output, '--out-name', 'application_source_probe',
    '--remove-name-section', '--remove-producers-section']);
writeFileSync(resolve(output, 'package.json'), '{"type":"module"}\n');
run(process.execPath, ['packages/cem_ql/tests/application-source-wasm.mjs', output]);
