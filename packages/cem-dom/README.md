# `@epa-wg/cem-dom`

Schema, parser, validator, and transform pipeline foundation for CEM semantic HTML documents.

## Exports

```ts
import { formatDiagnostics, parseCemDom, validateCemDom } from '@epa-wg/cem-dom';
```

- `parseCemDom(source, options)` returns a normalized light-DOM tree, flat element list, source locations, and parse diagnostics.
- `validateCemDom(source, options)` returns parse and validation diagnostics for semantic CEM documents.
- `formatDiagnostics(diagnostics)` renders diagnostics for CLI and report output.

## CLI

Current implemented commands:

```bash
yarn nx run @epa-wg/cem-dom:build
node packages/cem-dom/dist/cli.js help
node packages/cem-dom/dist/cli.js parse examples/semantic/login.html
node packages/cem-dom/dist/cli.js validate examples/semantic/login.html
node packages/cem-dom/dist/cli.js check examples/semantic/login.html --fail-level validate
node packages/cem-dom/dist/cli.js parse examples/semantic/login.html --format ast
node packages/cem-dom/dist/cli.js inspect examples/semantic/login.html --show tree
node packages/cem-dom/dist/cli.js convert examples/semantic/login.html --from-format html --to-format ast
node packages/cem-dom/dist/cli.js bench examples/semantic/login.html --iterations 25 --format json
node packages/cem-dom/dist/cli.js fixture validate
node packages/cem-dom/dist/cli.js version
```

During development the CLI can also run directly through Node's native TypeScript support:

```bash
node packages/cem-dom/src/cli.ts validate examples/semantic/login.html
```

Implemented options include `--fail-level parse|validate|strict`, `--format text|json|markdown|dom-json|ast|events|tree`,
`--from-format html|xml`, `--to-format dom-json|ast|events`,
`--preserve-source-offsets`, `--show summary|ast|diagnostics|source-offsets|tree`, `--out`, `--report-json`,
`--report-md`, `--schema`, `--content-type`, `--base-uri`, `--zero-hard-violations`, `--iterations`, `--budget-ms`,
`--profile`, `--cold-cache`, `--quiet`, `--verbose`, and `--no-color`.

### CLI roadmap

The current CLI implements the Tier A surface plus parser-backed `inspect`, parser-backed `convert`, and
parser/validator-backed `bench` slices. The broader CLI direction is documented in:

- [`docs/cli-ideas.md`](docs/cli-ideas.md) — brainstormed command surface for parse, validate, check, transform,
  conversion, schema tooling, fixture workflows, inspection, trace, benchmarks, and plugins.
- [`docs/cli-ac.md`](docs/cli-ac.md) — acceptance criteria and an implementation-planning prompt for the CLI.

Completed CLI work centers on `parse`, `validate`, `check`, parser-backed `inspect`, parser-backed `convert`,
parser/validator-backed `bench`, `fixture validate`, stable diagnostics, JSON/Markdown reports, and
`--fail-level parse|validate|strict`. Advanced conversion beyond HTML/XML input to DOM JSON/AST/events, transform,
schema emission/sample generation, advanced inspection, tracing, transform benchmarking, profiler integration, and
plugin workflows are reserved future or experimental work until implemented and tested.

## Nx Targets

```bash
yarn nx run @epa-wg/cem-dom:build
yarn nx run @epa-wg/cem-dom:typecheck
yarn nx run @epa-wg/cem-dom:lint
yarn nx run @epa-wg/cem-dom:test
yarn nx run @epa-wg/cem-dom:validate-fixtures
```

The `test` target uses the native Node test runner with native coverage. It does not use `ts-node`, `tsx`, Babel, Jest,
or Vitest.

## Key Paths

- `src/index.ts` — public library exports.
- `src/lib/cem-dom.ts` — parser, validator, and diagnostic formatter.
- `src/cli.ts` — `cem-dom` CLI entrypoint.
- `scripts/validate-fixtures.ts` — fixture validation report generator.
- `docs/cli-ac.md` — CLI acceptance criteria and implementation-planning prompt.
- `docs/cli-ideas.md` — raw CLI proposal notes.
- `dist/cem-dom.report.{md,json}` — generated fixture validation reports.

## Related Docs

- [`docs/dom-library-plan.md`](../../docs/dom-library-plan.md)
- [`docs/cem-dom-ac.md`](../../docs/cem-dom-ac.md)
