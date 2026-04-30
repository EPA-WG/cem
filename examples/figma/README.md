# CEM Figma Token Workflow

The MVP Figma workflow is read-only from generated token artifacts. Markdown token specs remain the source of truth;
Figma changes must be converted into spec edits before they enter the build.

## Tokens Studio Pull-Only Setup

1. Run the token export pipeline so `packages/cem-theme/dist/lib/tokens/figma/` contains:
    - `cem-light.tokens.json`
    - `cem-dark.tokens.json`
    - `cem-contrast-light.tokens.json`
    - `cem-contrast-dark.tokens.json`
    - `cem-native.tokens.json`
    - `cem-figma-report.md`
2. In Tokens Studio, create one token project or collection named `CEM`.
3. Configure sync to pull the generated Figma token files from the repository.
4. Import each generated file as a separate theme/mode:
    - `light`
    - `dark`
    - `contrast-light`
    - `contrast-dark`
    - `native`
5. Keep push/write-back disabled.
6. Before sharing the collection, check `cem-figma-report.md` for excluded tokens, concrete alias fallbacks, warnings,
   and validation errors.

`native` mode values are Chromium-computed browser-reference values. They are not iOS or Android system color
equivalents.

## Sample Application

Use [sample-token-application.md](./sample-token-application.md) as the local fixture for applying imported variables
to a button and card in a Figma test file. Replace it with screenshots after the Tokens Studio pull has been validated
in a real Figma collection.

## REST API Sync Policy

Do not enable Figma REST API write/sync in local builds or CI until file import is stable and token governance exists.
If REST sync is added later, it must start as a manual script with:

- explicit Figma file id configuration
- scoped write token
- dry-run/report mode
- no default local build execution

CI REST sync may only be considered after manual sync is proven. It must be gated behind protected branch or release
workflows, required approval, secret-scoped tokens, generated diff/report artifacts, and rollback instructions.

## Developer Prompt: Direct Figma Variables Import

Use this prompt if the MVP workflow intentionally switches from Tokens Studio pull-only to direct Figma Variables file
import:

```text
Update the CEM token export Figma workflow to use direct Figma Variables file import instead of Tokens Studio.
Keep one CEM collection. Use the generated files in dist/lib/tokens/figma/ as the only Figma input:
cem-light.tokens.json, cem-dark.tokens.json, cem-contrast-light.tokens.json, cem-contrast-dark.tokens.json,
and cem-native.tokens.json. Preserve read-only governance: Figma changes must become markdown spec edits, not
write-backs. Update docs/todo.md, packages/cem-theme/docs/token-export.md, and examples/figma/README.md.
```

## Developer Prompt: Split Figma Collections

Use this prompt only if Figma collection limits or designer navigation justify splitting the one-collection workflow:

```text
Update the CEM token export Figma workflow to split the single CEM collection into dimension-specific collections
only if Figma collection limits or designer navigation justify it. Proposed collections: CEM Color, CEM Dimension,
CEM Typography, CEM Motion, and CEM Platform Notes. Keep markdown specs as source of truth, keep Figma read-only,
and document cross-collection alias handling. If aliases cannot be preserved safely across collections, duplicate
only resolved values and list the loss of alias semantics in cem-figma-report.md. Update docs/todo.md,
packages/cem-theme/docs/token-export.md, and examples/figma/README.md.
```
