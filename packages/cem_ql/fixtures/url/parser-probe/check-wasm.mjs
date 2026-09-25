// Execute the isolated native parser implementations without a host URL API.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const bytes = await readFile(process.argv[2]);
const module = await WebAssembly.compile(bytes);
assert.deepEqual(WebAssembly.Module.imports(module), [], 'probe must need no host imports');
const instance = await WebAssembly.instantiate(module, {});
assert.equal(instance.exports.parser_probe(), 7 << 16);
console.log('WASM: upstream 7/51 cases differ; Ada 0/51; zero host imports');
