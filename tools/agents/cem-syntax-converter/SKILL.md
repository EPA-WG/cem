---
name: cem-syntax-converter
description: Convert requested markup to CEM-ML or queries to CEM-QL, preserving semantics and using the native tabular formatter. Use for explicit syntax conversion requests, including gallery examples.
---

# CEM syntax converter

Work only on the conversion assigned by the parent agent. Read `CLAUDE.md` and
re-read the target source before editing; other work may already be present.
Do not delegate this assignment again.

## Choose the language

- HTML/XML elements, attributes, and component examples become CEM-ML markup.
- Query expressions become CEM-QL. CEM-QL is not a substitute name for CEM-ML.
- Follow the user's explicit target. If the requested language conflicts with
  the source or intended behavior, ask before performing that conversion.

Read only the relevant syntax context:

- Markup: `docs/cem-ml-syntax.md`, then the actual component declarations for
  runtime bindings. The syntax document includes sketches; the native parser
  and current executable examples establish supported syntax.
- Queries: `packages/cem_ql/README.md` and
  `packages/cem_ml/schema-packages/cem-ql/v1/README.md`. Check supported query
  functions and syntax before translating expressions.
- CLI identity and export details: `docs/cem-ml-cli-contract.md` as needed.

## Convert and format

Use the repository's native `cem-ml` CLI and typed import/export pipeline.
The existing binary is `dist/target/cem_ml_cli/debug/cem-ml`; if missing, prefer
`yarn nx run cem_ml_cli:build`. Read `convert --help` before choosing flags.
Do not implement a regex-based syntax converter or route through JSON.

Always select the **tabular** formatter for converted output. For HTML markup:

```sh
dist/target/cem_ml_cli/debug/cem-ml convert /tmp/input.html \
  --from-format html --to-format cem \
  --cemt-formatter-profile tabular --output-color-type none --out /tmp/output.cem
```

For XML, use `--from-format xml`. Use temporary paths unique to the task.
For CEM-QL, select its explicit content/schema identity from the query package
README and run the tabular formatter for that language. Do not claim a generic
markup export translates query semantics. The current CEM-QL tabular profile
preserves source layout; do not invent alignment the formatter does not emit.
A verified query-module formatter invocation is:

```sh
dist/target/cem_ml_cli/debug/cem-ml convert /tmp/input.cemql \
  --content-type application/vnd.cem.query+cem-ql \
  --to-content-type application/vnd.cem.query+cem-ql \
  --cemt-formatter-profile tabular --output-color-type none --out /tmp/output.cemql
```

Inspect diagnostics as well as the exit code. The current CLI may report
`cem.lifecycle.adapter_unsupported` or `cem.lifecycle.target_adapter_unsupported`
for the standalone expression identity even when it exits successfully. Do not
report those runs as successful conversion or wrap an expression in an invented
module to hide the problem. Report the missing language adapter to the parent.

Keep file output free of ANSI escapes. Preserve formatter-produced layout when
embedding the result; do not manually realign it afterward.

Keep IDs, attributes, source values, markup, bindings, and behavior intact.
Preserve empty versus absent attributes and significant whitespace. If the
source includes dynamic bindings, verify their language boundaries explicitly.

For `cem-demo-element`, use `type="cem-ml"` and keep the converted source in its
inert `template slot="source"`. Preserve `legend`, `description`, coverage
attributes, and case IDs. Do not add page-local JavaScript or change unrelated
examples. Avoid literal HTML parsing of raw CEM source containing angle brackets;
follow existing source-template escaping conventions when needed.

## Verify and hand back

Parse/re-export the converted source using the native CLI and compare the
relevant elements, attributes, text, and behavior with the original. For a
syntax-only conversion, a round-trip structure comparison is useful. Prefer
XML export for markup comparison: the current HTML export can add syntax-coloring
`class` and `data-role` attributes even with output colors disabled. Use
existing story/gallery checks for runtime behavior when available. Report
browser/environment limits separately from successful native checks.

Return the changed paths, formatter command, verification results, and any
remaining ambiguity. Leave commits and pushes to the parent agent. If fixture
work is added, record it in `docs/todo.md` or tell the parent exactly what to add.
