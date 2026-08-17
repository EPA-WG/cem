export const fixtureDirectory = '/cem-ml-all-operations';

export const fixtureFiles = Object.freeze({
    'input.cem': `@doc cem-ml 1
@ns cem = "https://cem.dev/ns/core/1"
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{main @cem:screen="fixture" | {h1 | Fixture}}
`,
    'data.xml': '<catalog><item id="one"/><item id="two"/></catalog>\n',
    'template.xpath': '/catalog/item\n',
    'graph.cem': `{run |
  {import @id=data @src="data.xml" @content-type="application/xml" |
    {transform @id=items @src="template.xpath" @template-content-type="application/vnd.cem.xpath" @template-schema="https://cem.dev/ns/query/xpath/1" |
      {export @id=result @out="graph-output.json" @content-type="application/vnd.cem.xpath-result+json" @schema="https://cem.dev/ns/query/xpath/1"}
    }
  }
}
`,
});

export const commandCases = Object.freeze([
    {
        name: 'parse',
        operation: 'parse',
        argv: ['parse', 'input.cem', '--format', 'ast-json', '--preserve-source-offsets'],
    },
    {
        name: 'validate',
        operation: 'validate',
        argv: ['validate', 'input.cem', '--format', 'json', '--report-json', 'validate-report.json'],
    },
    {
        name: 'check',
        operation: 'check',
        argv: ['check', 'input.cem', '--format', 'json', '--zero-hard-violations'],
    },
    {
        name: 'inspect',
        operation: 'inspect',
        argv: ['inspect', 'input.cem', '--show', 'summary', '--format', 'json'],
    },
    {
        name: 'convert',
        operation: 'convert',
        argv: ['convert', 'input.cem', '--to-format', 'dom-json', '--preserve-source-offsets'],
    },
    {
        name: 'query',
        operation: 'query',
        argv: [
            'query',
            'data.xml',
            '--query',
            '//item',
            '--query-content-type',
            'application/vnd.cem.xpath',
            '--output',
            'json',
        ],
    },
    {
        name: 'transform-direct',
        operation: 'transform',
        sourceKind: 'direct',
        argv: [
            'transform',
            'data.xml',
            '--data-content-type',
            'application/xml',
            '--template',
            'template.xpath',
            '--template-content-type',
            'application/vnd.cem.xpath',
            '--to-content-type',
            'application/vnd.cem.xpath-result+json',
            '--to-schema',
            'https://cem.dev/ns/query/xpath/1',
        ],
    },
    {
        name: 'transform-graph',
        operation: 'transform',
        sourceKind: 'graph',
        argv: ['transform', '--config', 'graph.cem'],
    },
    {
        name: 'trace',
        operation: 'trace',
        argv: ['trace', 'input.cem', '--format', 'json'],
    },
    {
        name: 'version-capabilities',
        operation: 'version-capabilities',
        argv: ['version'],
    },
]);

export const portableOperationKinds = Object.freeze([
    'parse',
    'validate',
    'check',
    'inspect',
    'convert',
    'query',
    'transform',
    'trace',
    'version-capabilities',
]);

export function normalizeWasmCommandResult(fixtureCase, result) {
    const portable = result?.result?.storage === 'inline' ? result.result.value : undefined;
    if (portable?.kind !== fixtureCase.operation) {
        throw new Error(
            `${fixtureCase.name} returned ${portable?.kind ?? 'no inline result'} instead of ${fixtureCase.operation}`,
        );
    }

    switch (fixtureCase.name) {
        case 'parse':
            return normalizeAst(portable.value.primary);
        case 'validate':
            return normalizeReport(portable.value.report);
        case 'check':
            return normalizeReport(portable.value.report);
        case 'inspect':
            return normalizeInspect(portable.value.body);
        case 'convert':
            return normalizeAst(singleOutput(portable.value.outputs).response.primary);
        case 'query':
            return normalizeSequence(portable.value.output);
        case 'transform-direct':
            return normalizeSequence(singleOutput(portable.value.value.outputs).response.primary);
        case 'transform-graph':
            return normalizeSequence(singleArtifact(portable.value.value.artifacts).primary);
        case 'trace':
            return normalizeTrace(portable.value.body);
        case 'version-capabilities':
            return normalizeVersion(portable.value.version.commonVersion);
        default:
            throw new Error(`unsupported CEM-ML parity fixture ${fixtureCase.name}`);
    }
}

export function normalizeNativeCommandResult(fixtureCase, stdout, files = {}) {
    if (fixtureCase.name === 'version-capabilities') {
        return normalizeVersion(/^cem-ml (\d+\.\d+\.\d+(?:[-+][^\s]+)?)$/m.exec(stdout)?.[1]);
    }
    const value = fixtureCase.name === 'transform-graph' ? files['graph-output.json'] : JSON.parse(stdout);
    switch (fixtureCase.name) {
        case 'parse':
        case 'convert':
            return normalizeAst(value);
        case 'validate':
        case 'check':
            return normalizeReport(value);
        case 'inspect':
            return normalizeInspect(value);
        case 'query':
        case 'transform-direct':
        case 'transform-graph':
            return normalizeSequence(value);
        case 'trace':
            return normalizeTrace(value);
        default:
            throw new Error(`unsupported CEM-ML parity fixture ${fixtureCase.name}`);
    }
}

function normalizeAst(value) {
    const nodes = Array.isArray(value) ? value : value?.children;
    if (!Array.isArray(nodes)) throw new Error('parity AST result does not contain a node sequence');
    const elements = [];
    visit(nodes, (entry) => {
        if (entry?.kind === 'element') elements.push(entry.name);
    });
    return {
        kind: Array.isArray(value) ? 'ast' : value.kind,
        topLevelCount: nodes.length,
        elements,
        sourceMaps: summarizeSourceMaps(value),
    };
}

function normalizeReport(report) {
    if (report?.summary === undefined) throw new Error('parity report result is missing its summary');
    return {
        kind: 'report',
        summary: {
            inputCount: report.summary.inputCount,
            infoCount: report.summary.infoCount,
            warningCount: report.summary.warningCount,
            errorCount: report.summary.errorCount,
            fatalCount: report.summary.fatalCount,
            hardViolationCount: report.summary.hardViolationCount,
        },
        diagnosticCount: report.diagnostics?.length ?? 0,
        sourceMaps: summarizeSourceMaps(report),
    };
}

function normalizeInspect(body) {
    return {
        kind: body?.kind,
        elements: body?.elements,
        attributes: body?.attributes,
        diagnosticCount: body?.diagnosticCount,
        sourceMaps: summarizeSourceMaps(body),
    };
}

function normalizeSequence(value) {
    const result = value?.result ?? value;
    const sequence = result?.sequence;
    if (!Array.isArray(sequence?.items)) {
        throw new Error('parity query/transform result is missing its sequence');
    }
    return {
        kind: 'sequence',
        language: value?.language ?? result.language ?? 'xpath',
        contentType: result.contentType,
        sequenceType: sequence.sequenceType,
        itemCount: sequence.items.length,
        itemKinds: sequence.items.map(({ kind }) => kind),
        sourceMaps: summarizeSourceMaps(result),
    };
}

function normalizeTrace(body) {
    if (!Array.isArray(body?.events)) throw new Error('parity trace result is missing events');
    const eventKinds = {};
    for (const { kind } of body.events) eventKinds[kind] = (eventKinds[kind] ?? 0) + 1;
    return {
        kind: body.kind,
        eventCount: body.events.length,
        eventKinds,
        report: normalizeReport(body.report),
        parserStageCount: body.report?.reportAst?.parserStages?.stageCount,
        sourceMaps: summarizeSourceMaps(body),
    };
}

function normalizeVersion(commonVersion) {
    if (typeof commonVersion !== 'string') throw new Error('parity version result is missing the common version');
    return { kind: 'version-capabilities', commonVersion, sourceMaps: { count: 0, frameCount: 0 } };
}

function singleOutput(outputs) {
    if (outputs?.originalCount !== 1 || outputs.items?.length !== 1) {
        throw new Error('parity operation did not produce exactly one output');
    }
    return outputs.items[0];
}

function singleArtifact(artifacts) {
    if (!Array.isArray(artifacts) || artifacts.length !== 1) {
        throw new Error('parity graph transform did not produce exactly one artifact');
    }
    return artifacts[0];
}

function summarizeSourceMaps(value) {
    let count = 0;
    let frameCount = 0;
    visit(value, (entry, key) => {
        if (key !== 'sourceMap' || !Array.isArray(entry?.frames)) return;
        count += 1;
        frameCount += entry.frames.length;
    });
    return { count, frameCount };
}

function visit(value, visitor, key) {
    if (value === null || typeof value !== 'object') return;
    visitor(value, key);
    if (Array.isArray(value)) {
        for (const entry of value) visit(entry, visitor);
        return;
    }
    for (const [childKey, entry] of Object.entries(value)) visit(entry, visitor, childKey);
}
