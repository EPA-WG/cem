// Linked evidence for the selected application build. Run after the release build.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { closeSync, mkdirSync, mkdtempSync, openSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { wasmSections } from './audit-components-wasm.mjs';
import { WASM_OPT_VERSION } from './wasm-opt.mjs';

const root = resolve(import.meta.dirname, '../..');
const output = resolve(root, 'dist/reports/components-wasm/selected-link');
const raw = resolve(root, 'dist/target/cem_ql_selected_application/wasm32-unknown-unknown/release/cem_ql.wasm');
const release = resolve(root, 'packages/cem_ql/dist/wasm-selected-application/cem_ql_bg.wasm');
const optimization = JSON.parse(readFileSync(resolve(root, 'dist/reports/wasm-optimization/cem-ql-selected-application.json'), 'utf8'));
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const decoder = new TextDecoder('utf-8', { fatal: true });
const binaryen = resolve(root, 'node_modules/binaryen');
const wasmBindgen = 'wasm-bindgen 0.2.122';

function reader(bytes) {
    let offset = 0;
    const take = size => {
        const end = offset + size;
        assert(end <= bytes.length, 'truncated WASM field');
        const result = bytes.subarray(offset, end);
        offset = end;
        return result;
    };
    const byte = () => take(1)[0];
    const u32 = () => {
        let result = 0;
        for (let shift = 0; shift <= 28; shift += 7) {
            const next = byte();
            result += (next & 127) * 2 ** shift;
            assert(result <= 0xffffffff, 'WASM integer overflow');
            if (!(next & 128)) return result;
        }
        throw Error('overlong WASM integer');
    };
    return { take, byte, u32, text: () => decoder.decode(take(u32())), done: () => offset === bytes.length };
}

function sectionsWithBytes(bytes) {
    let offset = 8;
    return wasmSections(bytes).map(section => {
        const start = offset;
        offset += section.totalBytes;
        const payload = bytes.subarray(start + section.framingBytes, offset);
        return {
            ...section, payload, encoded: bytes.subarray(start, offset),
            customName: section.id === 0 ? reader(payload).text() : undefined,
        };
    });
}

function namedFunctions(module) {
    const [section, ...rest] = WebAssembly.Module.customSections(module, 'name');
    assert(section && rest.length === 0, 'expected one WASM name section');
    const names = new Map();
    const subsections = reader(new Uint8Array(section));
    while (!subsections.done()) {
        const id = subsections.byte();
        const bytes = subsections.take(subsections.u32());
        if (id !== 1) continue;
        assert.equal(names.size, 0, 'duplicate function names subsection');
        const entries = reader(bytes);
        const count = entries.u32();
        for (let i = 0; i < count; i++) {
            const index = entries.u32();
            assert(!names.has(index), 'duplicate WASM function index');
            names.set(index, entries.text());
        }
        assert(entries.done());
    }
    return names;
}

function functionExports(section, names) {
    const entries = reader(section.payload);
    const result = new Map();
    const count = entries.u32();
    for (let i = 0; i < count; i++) {
        const name = entries.text(), kind = entries.byte(), index = entries.u32();
        if (kind === 0) result.set(name, names.get(index));
    }
    assert(entries.done());
    return result;
}

function tableFunctions(section, names) {
    const entries = reader(section.payload);
    const result = new Set();
    const count = entries.u32();
    for (let i = 0; i < count; i++) {
        // The selected toolchain emits active function-index segments for table 0.
        // Refuse other encodings rather than infer an incomplete set of roots.
        assert.equal(entries.u32(), 0, 'unsupported WASM element segment');
        assert.equal(entries.byte(), 0x41, 'expected constant table offset');
        entries.u32();
        assert.equal(entries.byte(), 0x0b, 'expected end of table offset');
        const size = entries.u32();
        for (let j = 0; j < size; j++) {
            const name = names.get(entries.u32());
            assert(name, 'unnamed WASM table function');
            result.add(name);
        }
    }
    assert(entries.done());
    return result;
}

function directCalls(dot) {
    const edges = new Map();
    for (const line of dot.split('\n')) {
        if (!line.endsWith('; // call')) continue;
        const match = /^  "((?:[^"\\]|\\.)*)" -> "((?:[^"\\]|\\.)*)"; \/\/ call$/.exec(line);
        assert(match, 'unexpected Binaryen direct call syntax');
        const decode = symbol => symbol.replace(/\\(["\\])/g, '$1');
        const from = decode(match[1]), to = decode(match[2]);
        if (!edges.has(from)) edges.set(from, new Set());
        edges.get(from).add(to);
    }
    return edges;
}

function trace(edges, roots) {
    const pending = [...roots];
    const predecessors = new Map(pending.map(root => [root, null]));
    for (const from of pending) {
        for (const to of edges.get(from) ?? []) {
            if (predecessors.has(to)) continue;
            predecessors.set(to, from);
            pending.push(to);
        }
    }
    return predecessors;
}

function pathTo(symbol, predecessors) {
    const path = [];
    for (let current = symbol; current !== null; current = predecessors.get(current)) path.push(current);
    return path.reverse();
}

assert.equal(optimization.mode, 'Oz');
assert.equal(optimization.binaryen, WASM_OPT_VERSION);
assert.equal(JSON.parse(readFileSync(resolve(binaryen, 'package.json'), 'utf8')).version, WASM_OPT_VERSION);
assert.equal(execFileSync('wasm-bindgen', ['--version'], { encoding: 'utf8' }).trim(), wasmBindgen);
const shipped = readFileSync(release);
assert.equal(sha(shipped), optimization.outputSha256, 'optimized artifact does not match its report');
assert.equal(shipped.length, optimization.outputBytes);
const optimized = new WebAssembly.Module(shipped);

mkdirSync(output, { recursive: true });
const temporary = mkdtempSync(resolve(output, 'named-'));
try {
    // Keep the production out-name and linker input, so the name custom
    // section is the only difference from the recorded optimizer input.
    execFileSync('wasm-bindgen', [raw, '--target', 'web', '--out-dir', temporary,
        '--out-name', 'cem_ql', '--remove-producers-section']);
    const namedPath = resolve(temporary, 'cem_ql_bg.wasm');
    const named = readFileSync(namedPath);
    const sections = sectionsWithBytes(named);
    const stripped = Buffer.concat([named.subarray(0, 8),
        ...sections.filter(section => section.customName !== 'name').map(section => section.encoded)]);
    assert.equal(sha(stripped), optimization.inputSha256, 'named link differs from the optimizer input');
    assert.equal(stripped.length, optimization.inputBytes);
    const module = new WebAssembly.Module(named);
    const imports = WebAssembly.Module.imports(module);
    const exports = WebAssembly.Module.exports(module);
    assert.deepEqual(imports, WebAssembly.Module.imports(optimized));
    assert.deepEqual(exports, WebAssembly.Module.exports(optimized));
    const exportNames = new Set(exports.map(entry => entry.name));
    assert(!exportNames.has('renderCompiledApplication'));
    assert(!exportNames.has('applicationSourceOpenV1'));

    const names = namedFunctions(module);
    const importedFunctions = imports.filter(entry => entry.kind === 'function').length;
    const definedFunctions = reader(sections.find(section => section.name === 'code').payload).u32();
    const definedNames = [...names].filter(([index]) => index >= importedFunctions && index < importedFunctions + definedFunctions)
        .map(([, name]) => name);
    const dotPath = resolve(output, 'call-graph.dot');
    const fd = openSync(dotPath, 'w');
    try {
        const result = spawnSync(process.execPath, [resolve(binaryen, 'bin/wasm-opt'), namedPath, '--print-call-graph'],
            { stdio: ['ignore', fd, 'inherit'] });
        if (result.status !== 0) throw result.error ?? Error('could not read linked direct calls');
    } finally {
        closeSync(fd);
    }
    const dot = readFileSync(dotPath);
    const edges = directCalls(dot.toString('utf8'));
    const exportFunctions = functionExports(sections.find(section => section.name === 'export'), names);
    const roots = [...exportFunctions.keys()].filter(name => name.startsWith('compileddeclarationowner_')
        || name.startsWith('__wbg_compileddeclarationowner_') || name === 'compiledDeclarationInventory'
        || name.includes('SelectedApplication') || name.startsWith('selectedApplication'));
    const traces = new Map(roots.map(name => {
        const root = exportFunctions.get(name);
        assert(root && definedNames.includes(root), `missing named application export: ${name}`);
        return [name, trace(edges, [root])];
    }));
    const table = tableFunctions(sections.find(section => section.name === 'element'), names);
    const tableTrace = trace(edges, table);
    const categories = {
        compiledEntry: /cem_ql::application_runtime::selected::SelectedApplication::/,
        selectedEvaluator: /cem_ql::eval::selected::(?:call|pipeline_step)(?:_linked)?::/,
        graphqlGuard: /cem_ml::validation::json::graphql::GraphqlContracts::/,
        jsonGrammar: /cem_ml::validation::json::JsonParser::parse_value::/,
        jsonSourceContractCompilation: /cem_ml::validation::json::JsonSchemaContractCatalog::from_schema_source::/,
        schemaSourceCompilation: /cem_ml::schema::document_model::(?:compile_document_model_with_seen|parse_cem_document)::/,
        cemSourceTokenizer: /cem_ml::tokenizer::cem::CemTokenizer::from_source_inner::/,
        legacyPipelineDispatch: /cem_ql::eval::pipeline::apply_builtin_step::/,
        unselectedStringCase: /<str>::to_(?:lower|upper)case$/,
        unselectedSequenceQuantifiers: /cem_ql::eval::pipeline::any_all_sequence::/,
        graphqlBuildCompiler: /apollo_(?:compiler|parser)::/,
        optionalApplicationEndpoint: /cem_ql::application_source_endpoint::/,
        optionalQueryEndpoint: /cem_ql::query_source_endpoint::/,
        templateSourceCompiler: /cem_ql::render::compile_template/,
        deferredLifecycleAdapters: /<cem_ml::lifecycle::(?:Csv|Yaml|Markdown|Scss|Xslt|CustomElementXsltCompat)Adapter as cem_ml::lifecycle::LifecycleAdapter>::/,
        deferredParserProviders: /(?:Csv|Markdown|Scss|Xslt)ParserContract as cem_ml::schema::parser_contracts::NativeParserContract>::/,
        transformEncoderRegistration: /cem_ml::transform_template::TransformTemplateEncodeImplementationRegistry::with_builtin_encoders::/,
        csv: /cem_ml::validation::csv::(?:extract_csv_parse_report|csv_document_ast_from_source_bytes_with_contracts)::/,
        yaml: /cem_ml::validation::yaml::(?:extract_yaml_parse_report|yaml_document_ast_from_source_bytes)::/,
        markdown: /cem_ml::validation::markdown::markdown_document_ast_from_source_bytes(?:_with_contracts)?::/,
        xslt: /cem_ml::validation::xslt::(?:xslt_stylesheet_ast_from_source_bytes_with_modules_and_contracts|validate_xslt_compat_source_bytes)::/,
        highlight: /cem_ml::api::source_highlight::highlight_source_scope::/,
        scss: /cem_ml::validation::scss::ScssEvaluator::/,
    };
    const witnesses = Object.fromEntries(Object.entries(categories).map(([label, pattern]) => {
        const symbols = definedNames.filter(name => pattern.test(name));
        const directPaths = Object.fromEntries([...traces].flatMap(([root, predecessors]) => {
            const symbol = [...predecessors.keys()].find(name => pattern.test(name));
            return symbol === undefined ? [] : [[root, pathTo(symbol, predecessors)]];
        }));
        const tableSymbol = [...tableTrace.keys()].find(name => pattern.test(name));
        return [label, {
            namedBodies: symbols.length,
            examples: symbols.slice(0, 3),
            directPaths,
            pathFromTableFunction: tableSymbol === undefined ? null : pathTo(tableSymbol, tableTrace),
        }];
    }));
    for (const family of ['compiledEntry', 'selectedEvaluator', 'graphqlGuard']) {
        assert(witnesses[family].directPaths.renderSelectedApplicationResponse, `missing selected response path to ${family}`);
    }
    assert(witnesses.jsonGrammar.namedBodies > 0, 'selected profile lost its native JSON grammar');
    assert(witnesses.jsonGrammar.directPaths.renderSelectedApplicationResponse, 'guarded responses lost native JSON parsing');
    assert(witnesses.jsonGrammar.directPaths.compileddeclarationowner_renderJson, 'declarations lost native JSON parsing');

    for (const family of ['deferredLifecycleAdapters', 'deferredParserProviders', 'transformEncoderRegistration',
        'csv', 'yaml', 'markdown', 'xslt', 'highlight', 'scss', 'jsonSourceContractCompilation',
        'schemaSourceCompilation', 'cemSourceTokenizer', 'legacyPipelineDispatch',
        'unselectedStringCase', 'unselectedSequenceQuantifiers']) {
        assert.equal(witnesses[family].namedBodies, 0, `selected profile retained ${family}`);
    }

    const report = {
        contract: 'cem-selected-application-link-audit/1',
        nativeLinkClosureVerified: false,
        wasmBindgen, optimization,
        linkerOutputSha256: sha(readFileSync(raw)), namedOptimizerInputSha256: sha(named),
        directCallGraphSha256: sha(dot),
        linked: { definedFunctions, namedDefinedFunctions: definedNames.length, directCallSources: edges.size, tableFunctions: table.size, applicationExports: roots, witnesses },
        shipped: { imports, exports, sections: wasmSections(shipped) },
        scope: 'Named function bodies and direct calls in the exact pre-optimization input. The shipped stripped ABI and hash match the optimization record.',
        limits: ['A path from a table function proves a linked reference and direct edges from it. Dynamic targets, table liveness, inlining and optimized per-language bytes remain open.'],
    };
    writeFileSync(resolve(output, 'audit.json'), JSON.stringify(report, null, 2) + '\n');
    for (const [family, { namedBodies, directPaths, pathFromTableFunction }] of Object.entries(witnesses)) {
        const sources = Object.keys(directPaths);
        if (pathFromTableFunction) sources.push('a function table entry');
        console.log(`${family}: ${namedBodies} named bodies, direct paths from ${sources.join(', ') || 'none of the inspected roots'}`);
    }
} finally {
    rmSync(temporary, { recursive: true, force: true });
}
