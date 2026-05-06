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
node packages/cem-dom/dist/cli.js version
```

During development the CLI can also run directly through Node's native TypeScript support:

```bash
node packages/cem-dom/src/cli.ts validate examples/semantic/login.html
```

### CLI roadmap

The current CLI is intentionally small. The proposed CLI direction is documented in:

- [`docs/cli-ideas.md`](docs/cli-ideas.md) — brainstormed command surface for parse, validate, check, transform,
  conversion, schema tooling, fixture workflows, inspection, trace, benchmarks, and plugins.
- [`docs/cli-ac.md`](docs/cli-ac.md) — acceptance criteria and an implementation-planning prompt for the CLI.

Planned Tier A CLI work centers on `parse`, `validate`, `check`, `fixture validate`, stable diagnostics, JSON/Markdown
reports, and `--fail-level parse|validate|strict`. Transform, conversion, schema emission/sample generation,
inspection, tracing, benchmarking, and plugin workflows are future or experimental until implemented and tested.

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
