import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
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
