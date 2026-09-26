// Explicit query API JSON control boundary; URL semantics remain in Rust.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import init, { evaluateQuerySource } from '../dist/wasm/cem_ql.js';

await init({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const evaluate = source => JSON.parse(evaluateQuerySource(source, '{}'));

for (const [source, expected] of [
    ['url:href("../b", "https://h/a/c")', 'https://h/b'],
    ['import "cem:stdlib/url" as u\nu:href("https://h")', 'https://h/'],
    ['url:assemble({ href: "https://h", hash: "f" })', 'https://h/#f'],
    ['url:with_parts("https://h", { pathname: "/b" })', 'https://h/b'],
]) {
    const result = evaluate(source);
    assert.equal(result.error, null);
    assert.deepEqual(result.diagnostics, []);
    assert.deepEqual(result.items, [{ kind: 'atomic', type: 'any-uri', value: expected }]);
}
assert.deepEqual(evaluate('url:parse("bad")').items, []);
assert.deepEqual(evaluate('url:can_parse("bad")').items, [{ kind: 'atomic', type: 'boolean', value: false }]);

const partial = evaluate('url:with_parts("https://h:8443/a", { protocol: "mailto:", host: "other:70000", hash: "done" })');
assert.equal(partial.error, null);
assert.equal(partial.items[0].value, 'https://other:8443/a#done');
assert.deepEqual(partial.diagnostics.map(d => d.code), ['cem.ql.url_setter_ignored', 'cem.ql.url_setter_ignored']);
assert.match(partial.diagnostics[0].message, /protocol/);
assert.match(partial.diagnostics[1].message, /port/);
const invalid = evaluate('url:href("https://h", "bad")');
assert.ok(invalid.error);
assert.deepEqual(invalid.items, []);
assert.deepEqual(invalid.diagnostics.map(d => d.code), ['cem.ql.url_base_invalid']);
const upstream = evaluate('url:parse(report:raise("test.upstream", "failed"))');
assert.ok(upstream.error);
assert.deepEqual(upstream.diagnostics.map(d => d.code), ['test.upstream']);
assert.equal(evaluate('declare function url:href(x) { x } url:href(42)').items[0].value, 42);
console.log('URL query WASM: typed results, aliases, warnings, errors and user-function isolation passed.');

// Parameter operations execute in Rust through the same public WASM query API.
const parameterCases = [
    ['url:params_size(url:params("a=1&a=2"))', 'integer', 2],
    ['url:params_string(url:params({b: "2", a: "1"}))', 'string', 'a=1&b=2'],
    ['url:params_get(url:params("a=1&a=2"), "a")', 'string', '1'],
    ['url:params_has(url:params("a=1&a=2"), "a", "2")', 'boolean', true],
    ['url:params_string(url:params_append(url:params("a=1"), "a", "2"))', 'string', 'a=1&a=2'],
    ['url:params_string(url:params_set(url:params("b=0&a=1&a=2"), "a", "3"))', 'string', 'b=0&a=3'],
    ['url:params_string(url:params_delete(url:params("a=1&a=2"), "a", "1"))', 'string', 'a=2'],
    ['url:params_string(url:params_delete(url:params("a=1&a=2"), "a"))', 'string', ''],
    ['url:params_string(url:params_sort(url:params("=last&😀=first&😀=second")))', 'string', '%F0%9F%98%80=first&%F0%9F%98%80=second&%EE%80%80=last'],
    ['import "cem:stdlib/url" as u\nu:params_string(u:params("?x=+&x=%2B&bare"))', 'string', 'x=+&x=%2B&bare='],
];
for (const [source, type, value] of parameterCases) {
    const result = evaluate(source);
    assert.equal(result.error, null, source);
    assert.deepEqual(result.diagnostics, [], source);
    assert.deepEqual(result.items, [{kind: 'atomic', type, value}], source);
}
for (const [operation, expected] of [
    ['params_keys', ['a', 'a', 'b']], ['params_values', ['1', '2', '3']],
    ['params_get_all', ['1', '2']],
]) {
    const source = `url:${operation}(url:params("a=1&a=2&b=3")${operation === 'params_get_all' ? ', "a"' : ''})`;
    const result = evaluate(source);
    assert.equal(result.error, null);
    assert.deepEqual(result.items, expected.map(value => ({kind: 'atomic', type: 'string', value})));
}
for (const source of ['url:params()', 'url:params(())', 'url:params_get((), "missing")']) {
    const result = evaluate(source);
    assert.equal(result.error, null);
    assert.deepEqual(result.items, []);
}
assert.deepEqual(evaluate('url:params_entries(url:params("a=1&a=2"))'), evaluate('url:params("a=1&a=2")'));
for (const source of ['url:params_has((), "x", ())', 'url:params_get((), ())', 'url:params({a: ()})']) {
    const result = evaluate(source);
    assert.ok(result.error, source);
    assert.deepEqual(result.items, []);
    assert.deepEqual(result.diagnostics.map(d => d.code), ['cem.ql.type_error']);
    assert.equal(result.diagnostics[0].byteOffset, 0);
}
assert.equal(evaluate('declare function url:params_size(x) { x } url:params_size(42)').items[0].value, 42);
const parameterFailure = evaluate('url:params_string(report:raise("test.upstream", "failed"))');
assert.deepEqual(parameterFailure.diagnostics.map(d => d.code), ['test.upstream']);
console.log('URL parameters WASM: all 13 operations, exact types, duplicate order and diagnostics passed.');

const evaluateBound = (source, input) => JSON.parse(evaluateQuerySource(source, JSON.stringify({input})));
const pairs = evaluateBound('url:params_string(url:params($input))', {$stream: [['b', '2'], ['a', '1'], ['b', '3']]});
assert.equal(pairs.error, null);
assert.deepEqual(pairs.items, [{kind: 'atomic', type: 'string', value: 'b=2&a=1&b=3'}]);
for (const input of [['one'], [['x', '1']], {$stream: [['x', '1'], 'bad']}]) {
    const result = evaluateBound('url:params($input)', input);
    assert.ok(result.error);
    assert.deepEqual(result.diagnostics.map(d => d.code), ['cem.ql.type_error']);
    assert.equal(result.diagnostics[0].byteOffset, 0);
}

const matrix = JSON.parse(await readFile(new URL('../fixtures/url/query-matrix.json', import.meta.url), 'utf8'));
for (const row of matrix) {
    const result = evaluate(row.query);
    assert.deepEqual(result.items, row.items, row.id);
    assert.deepEqual(result.diagnostics.map(d => d.code), row.diagnosticCodes, row.id);
    assert.equal(result.error !== null, row.error, row.id);
    for (const diagnostic of result.diagnostics) {
        assert.equal(typeof diagnostic.byteOffset, 'number', row.id);
        assert.ok(diagnostic.sourceMap.frames.length, row.id);
    }
}
console.log(`Shared URL matrix: ${matrix.length} Node WASM cases passed.`);
