// Build canonical component fixtures with the native compiler, never a JS parser.
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../../', import.meta.url));
execFileSync('cargo', ['build', '--locked', '--offline', '-p', 'cem-ql', '--example', 'compile_declaration_artifact'], { cwd: root, stdio: 'inherit' });
for (const mode of ['worker', 'fallback']) {
    for (const [name, tag] of [['field', 'cem-text-field'], ['action', 'cem-action'], ['card', 'cem-card'], ['conflict', 'cem-text-field']]) {
        execFileSync(`${root}dist/target/debug/examples/compile_declaration_artifact`, [
            `packages/cem-components/src/components/${tag}/${tag}.xhtml`,
            `dist/artifacts/cem_ql/declarations/${mode}-${name}`,
            `https://${name === 'conflict' ? 'other' : 'app'}.test/components/${tag}/${tag}.xhtml`,
            'prod', `cem-aot-${name === 'conflict' ? 'field' : name}-${mode}`,
        ], { cwd: root, stdio: 'inherit' });
    }
}
