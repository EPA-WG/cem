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
