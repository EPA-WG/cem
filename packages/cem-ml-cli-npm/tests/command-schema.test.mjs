import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { mkdtemp, writeFile, rm, symlink, readdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import * as browserApi from '../dist/browser.js';
import * as nodeApi from '../dist/node.js';

const fixture = JSON.parse(
    await readFile(new URL('./command-roundtrip.fixture.json', import.meta.url), 'utf8'),
);

test('shared command schema is generated from native grammar and common capabilities', () => {
    const schema = nodeApi.commandSchema;
    assert.equal(schema.schemaVersion, fixture.schemaVersion);
    assert.equal(schema.commonVersion, nodeApi.commandSchema.commonVersion);
    assert.deepEqual(browserApi.commandSchema, schema);
    assert.equal(schema.binaryName, 'cem-ml');

    for (const name of [
        'parse',
        'validate',
        'check',
        'inspect',
        'convert',
        'query',
        'transform',
        'trace',
        'version',
    ]) {
        const command = schema.commands.find((candidate) => candidate.name === name);
        assert.ok(command, `missing portable command ${name}`);
        assert.equal(command.availability.wasmNode, 'available');
        assert.equal(command.availability.wasmBrowserWorker, 'available');
    }
    const bench = schema.commands.find(({ name }) => name === 'bench');
    assert.equal(bench?.availability.native, 'available');
    assert.equal(bench?.availability.wasmNode, 'unavailable');
    assert.equal(bench?.availability.wasmBrowserWorker, 'unavailable');
});

test('native-accepted command fixtures normalize and round trip through both npm hosts', () => {
    for (const fixtureCase of fixture.cases) {
        const nodeParsed = nodeApi.parseCemMlCommand(fixtureCase.argv, { runtime: 'wasm-node' });
        const browserParsed = browserApi.parseCemMlCommand(fixtureCase.argv, {
            runtime: 'wasm-browser-worker',
        });
        assert.deepEqual(browserParsed, nodeParsed, fixtureCase.name);
        const serialized = nodeApi.serializeCemMlCommand(nodeParsed);
        assert.deepEqual(
            nodeApi.parseCemMlCommand(serialized, { runtime: 'wasm-node' }),
            nodeParsed,
            `${fixtureCase.name} did not survive schema-driven serialization`,
        );
    }

    const parse = nodeApi.parseCemMlCommand(fixture.cases[0].argv);
    assert.equal(parse.options.format, 'ast');
    assert.deepEqual(parse.options.namespaces, [
        'ui=https://cem.dev/ns/ui',
        'data=https://cem.dev/ns/data',
    ]);
    const query = nodeApi.parseCemMlCommand(
        fixture.cases.find(({ name }) => name.startsWith('query-')).argv,
    );
    assert.equal(query.options.output, 'terminal');
});

test('literal command text preserves normalized argv without evaluating shell syntax', () => {
    const parsed = nodeApi.parseCemMlCommand([
        'parse',
        'studio://feature-tour/data/cem ml/author\'s basic.cem',
        '--content-type',
        'application/cem',
        '--schema',
        'https://cem.dev/ns/cem-ml/1',
        '--format',
        'events',
        '--no-color',
    ], { runtime: 'wasm-node' });
    const text = nodeApi.serializeCemMlCommandText(parsed);
    assert.match(text, /^cem-ml .*parse /);
    assert.ok(text.includes("'studio://feature-tour/data/cem ml/author'\\''s basic.cem'"));
    assert.deepEqual(
        browserApi.parseCemMlCommandText(text, { runtime: 'wasm-browser-worker' }),
        parsed,
    );
    assert.throws(
        () => nodeApi.parseCemMlCommandText('cem-ml parse "unterminated'),
        (error) => error instanceof nodeApi.CemMlCommandError && error.code === 'cem.command.text_quote',
    );
    assert.throws(
        () => nodeApi.parseCemMlCommandText('other-cli parse input.cem'),
        (error) => error instanceof nodeApi.CemMlCommandError && error.code === 'cem.command.binary_name',
    );
    assert.ok(Object.values(
        nodeApi.parseCemMlCommandText("cem-ml parse ''").positionals,
    ).includes(''));
    assert.throws(
        () => nodeApi.parseCemMlCommandText("cem-ml parse 'unsafe\0input'"),
        (error) => error instanceof nodeApi.CemMlCommandError && error.code === 'cem.command.text_control',
    );
});

test('authored CLI command resources preserve literal argv without persisting a run plan', () => {
    const parsed = nodeApi.parseCemMlCommand([
        'inspect',
        'studio://feature-tour/data/cem ml/basic.cem',
        '--show',
        'source-offsets',
        '--format',
        'cem',
    ], { runtime: 'wasm-node' });
    const source = nodeApi.serializeCemMlCommandResource(parsed);
    const nodeResource = nodeApi.parseCemMlCommandResource(source, { runtime: 'wasm-node' });
    const browserResource = browserApi.parseCemMlCommandResource(source, {
        runtime: 'wasm-browser-worker',
    });

    assert.deepEqual(browserResource.resource, nodeResource.resource);
    assert.deepEqual(browserResource.command, nodeResource.command);
    assert.deepEqual(nodeResource.resource.argv, nodeApi.serializeCemMlCommand(parsed));
    assert.equal(nodeResource.resource.binaryName, 'cem-ml');
    assert.equal(nodeResource.resource.commandSchemaVersion, nodeApi.commandSchema.schemaVersion);
    assert.equal(nodeResource.resource.commonVersion, nodeApi.commandSchema.commonVersion);
    assert.ok(!('runPlan' in nodeResource.resource));
    assert.equal(nodeApi.serializeCemMlCommandResource(nodeResource.command), source);

    assert.throws(
        () => nodeApi.parseCemMlCommandResource(source.replace('"cem-ml"', '"other-cli"')),
        (error) => error instanceof nodeApi.CemMlCommandError && error.code === 'cem.cli_command.binary_name_invalid',
    );
    assert.throws(
        () => nodeApi.parseCemMlCommandResource(source.replace('"0.1.0"', '"0.1.0-01"')),
        (error) => error instanceof nodeApi.CemMlCommandError && error.code === 'cem.cli_command.common_version_invalid',
    );
});

test('schema-driven parser rejects invalid fixtures with stable codes', () => {
    for (const fixtureCase of fixture.invalidCases) {
        assert.throws(
            () => nodeApi.parseCemMlCommand(fixtureCase.argv),
            (error) => error instanceof nodeApi.CemMlCommandError && error.code === fixtureCase.code,
            fixtureCase.name,
        );
    }
    assert.throws(
        () => nodeApi.parseCemMlCommand(['bench'], { runtime: 'wasm-node' }),
        (error) => error instanceof nodeApi.CemMlCommandError && error.code === 'cem.command.unavailable',
    );
});

test('project filesystem mappings round-trip as a native-only host option', async () => {
    const text = "cem-ml --resolver-project 'project.cem' convert studio://demo/input.cem";
    const parsed = nodeApi.parseCemMlCommandText(text, { runtime: 'native' });
    assert.equal(parsed.globalOptions.resolver_project, 'project.cem');
    assert.deepEqual(nodeApi.parseCemMlCommandText(nodeApi.serializeCemMlCommandText(parsed)), parsed);
    for (const runtime of ['wasm-node', 'wasm-browser-worker']) {
        assert.throws(() => nodeApi.parseCemMlCommandText(text, { runtime }), error => error.code === 'cem.cli_command.native_only_argument');
    }
    const { buildCommandInvocationWith } = await import('../dist/invocation-core.js');
    await assert.rejects(buildCommandInvocationWith(undefined, undefined, parsed, undefined), error => error.code === 'cem.cli_command.native_only_argument');
    await assert.rejects(nodeApi.buildNodeCommandInvocation(parsed, undefined), error => error.code === 'cem.cli_command.native_only_argument');
});

test('project bootstrap management uses native transport and the owned browser API', () => {
    const parsed = nodeApi.parseCemMlCommand(['project-bootstrap', '-'], { runtime: 'native' });
    assert.deepEqual(parsed.commandPath, ['project-bootstrap']);
    for (const runtime of ['wasm-node', 'wasm-browser-worker']) {
        assert.throws(() => nodeApi.parseCemMlCommand(['project-bootstrap'], { runtime }), error => error.code === 'cem.command.unavailable');
    }
});

test('stdin shell serialization preserves literal source and command arguments', { skip: process.platform === 'win32' }, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'cem-cli-stdin-'));
    try {
        await writeFile(join(directory, 'cem-ml'), `#!${process.execPath}\nconst fs=require('node:fs');process.stdout.write(JSON.stringify({argv:process.argv.slice(2),source:fs.readFileSync(0,'utf8')}));\n`, { mode: 0o755 });
        const parsed = nodeApi.parseCemMlCommand(['convert', '-', '--content-type', 'application/yaml', '--to-content-type', 'application/json']);
        for (const source of ["message: '$(touch unexpected)' `touch also-unexpected`\nquote: \"Café 🧬\"\n", '~', '', '-n', 'a\r\nb\n']) {
            const text = nodeApi.serializeCemMlCommandWithStdinText(parsed, source);
            const result = spawnSync('bash', ['-c', text], { cwd: directory, env: { ...process.env, PATH: `${directory}:${process.env.PATH}` }, encoding: 'utf8' });
            assert.equal(result.status, 0, result.stderr);
            assert.deepEqual(JSON.parse(result.stdout), { argv: nodeApi.serializeCemMlCommand(parsed), source });
        }
        assert.throws(() => nodeApi.serializeCemMlCommandWithStdinText(parsed, 'a\0b'), error => error.code === 'cem.cli_command.stdin_nul');
        await assert.rejects(readFile(join(directory, 'unexpected')));
        await assert.rejects(readFile(join(directory, 'also-unexpected')));
    } finally { await rm(directory, { recursive: true, force: true }); }
});

test('stdin shell commands execute conversions without saving source or output', { skip: process.platform === 'win32' }, async () => {
    const directory = await mkdtemp(join(tmpdir(), 'cem-cli-pipe-'));
    try {
        await symlink(fileURLToPath(new URL('../dist/bin.js', import.meta.url)), join(directory, 'cem-ml'));
        for (const [contentType, outputType, source, expected] of [
            ['application/yaml', 'application/json', 'message: "Café $(touch unexpected)"\nenabled: true\n', { message: 'Café $(touch unexpected)', enabled: true }],
            ['application/json', 'application/json', '{"message":"it\'s literal","count":3}', { message: "it's literal", count: 3 }],
            ['application/cem', 'text/html', '{article | {strong | Hello}}', '<article><strong>Hello</strong></article>'],
        ]) {
            const parsed = nodeApi.parseCemMlCommand(['convert', '/dev/stdin', '--content-type', contentType, '--to-content-type', outputType, '--output-color-type', 'none', '--cemt-color-profile', 'none', '--cemt-formatter-profile', 'tabular']);
            const result = spawnSync('bash', ['-c', nodeApi.serializeCemMlCommandWithStdinText(parsed, source)], {
                cwd: directory, env: { ...process.env, PATH: `${directory}:${process.env.PATH}` }, encoding: 'utf8', timeout: 30_000,
            });
            assert.equal(result.status, 0, result.stderr);
            assert.deepEqual(outputType === 'application/json' ? JSON.parse(result.stdout) : result.stdout.trim(), expected);
            assert.equal(result.stdout.includes('\x1b'), false);
        }
        assert.deepEqual(await readdir(directory), ['cem-ml']);
    } finally { await rm(directory, { recursive: true, force: true }); }
});
