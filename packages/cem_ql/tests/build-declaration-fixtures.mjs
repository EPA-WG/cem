// Build canonical component fixtures with the native compiler, never a JS parser.
import { execFileSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, symlinkSync, truncateSync, writeFileSync } from 'node:fs';
import assert from 'node:assert/strict';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
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

execFileSync('cargo', ['build', '--locked', '--offline', '-p', 'cem-ql', '--example', 'compile_declaration_closure'], { cwd: root, stdio: 'inherit' });
const inputs = `${root}dist/artifacts/cem_ql/closure-inputs`;
mkdirSync(inputs, { recursive: true });
for (const tag of ['cem-text-field', 'cem-action', 'cem-card', 'cem-text', 'cem-alert'])
    cpSync(`${root}packages/cem-components/src/components/${tag}`, `${inputs}/${tag}`, { recursive: true });
cpSync(`${root}packages/cem_ql/fixtures/component-application/compiled-form/cem-compiled-form-app`, `${inputs}/cem-compiled-form-app`, { recursive: true });
execFileSync(`${root}dist/target/debug/examples/compile_declaration_closure`, [inputs, 'https://app.test/components/',
    `${root}dist/artifacts/cem_ql/compiled-form`, 'prod', '--compiled-css', 'cem-compiled-form-app'], { cwd: root, stdio: 'inherit' });

// Keep the ordinary loader's 1 MiB response policy, including compiled CSS.
for (const name of readdirSync(`${root}dist/artifacts/cem_ql/compiled-form`)) {
    if (name.endsWith('.bin') && statSync(`${root}dist/artifacts/cem_ql/compiled-form/${name}`).size > 1024 * 1024)
        throw Error(`Compiled form asset exceeds the existing response limit: ${name}`);
}

// The opt-in filesystem adapter cannot turn pinned URL requirements into
// outside-root access. Failed builds publish no partial assets.
const temporary = mkdtempSync(join(tmpdir(), 'cem-css-closure-'));
try {
    const inputs = join(temporary, 'inputs'), tag = 'cem-css-probe';
    const folder = join(inputs, tag); mkdirSync(folder, { recursive: true });
    const external = join(temporary, 'external.css'); writeFileSync(external, ':scope {color:red}');
    symlinkSync(external, join(folder, 'linked.css'));
    writeFileSync(join(folder, 'oversized.css'), ''); truncateSync(join(folder, 'oversized.css'), 16 * 1024 * 1024 + 1);
    for (const [name, specifier, expected] of [
        ['outside', '../../external.css', /outside the declared source root/],
        ['symlink', './linked.css', /outside the source root/],
        ['missing', './missing.css', /No such file/],
        ['oversized', './oversized.css', /oversized/],
    ]) {
        writeFileSync(join(folder, `${tag}.xhtml`), `<cem-element tag="${tag}"><template id="${tag}" type="text/cem-ml">{style | \`\`\` @import "${specifier}"; :scope {display:block} \`\`\`}{p | Build policy}</template></cem-element>`);
        const output = join(temporary, name);
        assert.throws(() => execFileSync(`${root}dist/target/debug/examples/compile_declaration_closure`, [inputs,
            'https://app.test/components/', output, 'prod', '--compiled-css', tag], { cwd: root, stdio: 'pipe' }), expected);
        assert.equal(existsSync(output), false, `failed ${name} build published assets`);
    }
    const staticOutput = join(temporary, 'static-requirements');
    const probe = join(folder, `${tag}.xhtml`);
    writeFileSync(probe, `<cem-element tag="${tag}"><template id="${tag}" type="text/cem-ml">{p @title="{str:lower(datadom.attributes.label)}" | Static requirements}</template></cem-element>`);
    const buildStatic = output => execFileSync(`${root}dist/target/debug/examples/compile_declaration_closure`, [inputs,
        'https://app.test/components/', output, 'prod', '--require-static-requirements', '--selected-registry', tag], {cwd:root,stdio:'pipe'});
    buildStatic(staticOutput);
    const report = JSON.parse(readFileSync(join(staticOutput, 'requirements.json'), 'utf8'));
    assert.equal(report.contract, 'cem-application-requirements/1');
    assert.equal(report.nativeLinkClosureVerified, false);
    assert.ok(report.declarations[tag].functions.some(f => f.module === 'cem:stdlib/strings' && f.name === 'lower' && f.arity === 1));
    assert.deepEqual(report.declarations[tag].unresolved, []);
    assert.match(readFileSync(join(staticOutput, 'requirements.hash'), 'utf8').trim(), /^cem-bin\/1\+blake3:[0-9a-f]{64}$/);
    writeFileSync(probe, `<cem-element tag="${tag}"><template id="${tag}" type="text/cem-ml">{slot}</template></cem-element>`);
    const refused = join(temporary, 'open-requirements');
    assert.throws(() => buildStatic(refused), /unresolved dynamic\/host\/source contracts/);
    assert.equal(existsSync(refused), false, 'strict requirements failure published assets');
    writeFileSync(probe, `<cem-element tag="${tag}"><template id="${tag}" type="text/cem-ml">{p @title="{num:abs(1)}" | Unreviewed registry}</template></cem-element>`);
    const unreviewed = join(temporary, 'unreviewed-registry');
    assert.throws(() => buildStatic(unreviewed), /outside reviewed selected core/);
    assert.equal(existsSync(unreviewed), false, 'unreviewed registry published assets');
    const plan = JSON.parse(readFileSync(join(staticOutput, 'selected-registry.json'), 'utf8'));
    assert.equal(plan.nativeLinkClosureVerified, false);
    assert.deepEqual(plan.entries, ['string-lower']);
    assert.ok(plan.executionUnits.includes('unicode-case'));
    assert.match(readFileSync(join(staticOutput, 'selected-registry.hash'), 'utf8').trim(), /^cem-bin\/1\+blake3:[0-9a-f]{64}$/);
    // Compile and execute the generated Rust against the actual native library.
    const messages = execFileSync('cargo', ['build', '--locked', '--offline', '-p', 'cem-ql', '--lib', '--message-format=json'], {cwd:root,encoding:'utf8',stdio:['ignore','pipe','pipe']});
    const library = messages.trim().split('\n').map(line => JSON.parse(line)).find(row => row.reason === 'compiler-artifact' && row.target.name === 'cem_ql').filenames.find(name => name.endsWith('.rlib'));
    const program = join(staticOutput, 'check.rs'), executable = join(staticOutput, 'check');
    writeFileSync(program, 'include!("selected_registry.rs");\nfn main() { let registry = selected_registry(); assert!(registry.resolve("cem:stdlib/strings", "lower", 1).is_some()); assert!(registry.resolve("cem:stdlib/strings", "upper", 1).is_none()); }\n');
    execFileSync('rustc', ['--edition=2021', program, '-L', `dependency=${root}dist/target/debug/deps`, '--extern', `cem_ql=${library}`, '-o', executable], {cwd:root,stdio:'pipe'});
    execFileSync(executable, [], {cwd:root,stdio:'pipe'});

} finally { rmSync(temporary, { recursive: true, force: true }); }
console.log('Compiled closure fixtures: response budgets and outside-root/symlink/missing/oversized CSS and static-requirements admission/refusal passed.');
