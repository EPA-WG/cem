# JSON/YAML source Parse identities and diagnostics — 2026-10-27

- [x] Add failing native JSON, YAML and JSON Schema Parse/Inspect fixtures before changing source dispatch, including MIME aliases, literal query-looking data, Unicode/CRLF, original ranges and malformed/unsupported/over-budget refusals.
- [x] Consume the retained native import owner for structured-data source operations; preserve explicit JSON exports and distinguish source identity from projection identity.
- [x] Retain the native JSON Schema owner while sharing JSON import limits and generic-data mapping; verify original lexical ownership.
- [x] Present imported-tree events using the registered event vocabulary, original source ranges and retained ownership, without another source parser.
- [x] Preserve original native diagnostics, URI, severity and coordinates; refuse structural publication without turning absent output into a CLI write error.
- [x] Verify strict CLI behavior, both WASM bindings, Studio commands, saved/offline projects and complete Login recovery.
- [x] Update example configurations, generated bundles, coverage and documentation; retire only proven obsolete diagnostic exceptions.
- [x] Preserve credential privacy, shared component composition, all capabilities and deployment/recovery budgets.

## Cause and correction

Lifecycle loading already produced native JSON/YAML documents. Parse then sent
those original bytes through the CEM tokenizer and rules. Inspect could present
an imported tree while collecting unrelated diagnostics from that second parse.
JSON Schema also lacked a retained generic-data import path. Valid objects,
`$defs`/`$ref` and query-looking literal strings consequently acquired CEM errors.

The shared Rust source-operation path now consumes the native import directly.
It retains the native JSON Schema wrapper and maps its JSON values through the
existing generic-data importer, sharing document size, depth and value limits.
Native schema-source dialect admission remains enforced. No application parser,
serialized AST handoff, instance validator or reference resolver is introduced.

AST/events output identities describe the presentation vocabulary. Root metadata
preserves the selected source schema/MIME, including admitted aliases, with
original URI and ranges. The imported CEM projection is not a lossless
JSON/YAML/schema language AST. Events describe common imported nodes using
registered `open`/`close`/`value` kinds and explicit syntax/node-kind metadata;
fixtures check the actual registered vocabulary. Literal strings stay inert.

Malformed sources, unsupported YAML aliases/anchors/tags and import-limit
refusals publish no partial tree. Diagnostic inspection remains available.
Structural refusal errors and time-budget diagnostics retain the source URI.
CLI Parse preserves native reports and the requested failure policy instead of
turning an absent presentation into a write/IO failure. Command services do not
publish a null placeholder artifact for a refused import. Existing failure-level
semantics remain unchanged; the new strict examples select `--fail-level validate`.

JSON/YAML report and JSON Schema projects now include ordinary Parse
configurations; JSON Schema also exposes structural Inspect. Their versions and
Module maps are **1.0.1**, retaining existing resource/configuration IDs and
paths. Module-map inspection expectations now identify the selected v1/v2/v3
schema rather than generic CEM-ML. Source syntax inspection does not enforce
version-specific map semantics or grant reads. Saved projects and drafts retain
their independent contents; no automatic migration is added.

Five obsolete companion exceptions are removed: CEM-QL Orders JSON inspection,
JSON/YAML report JSON inspection and all three module-map inspections. The
Supplier XML source limit and five unrelated Inspect-companion limits remain
pinned with their original diagnostic facts. The separate Login CLI/worker
undeclared-binding discrepancy, control-source adapters, AST JSON contract and
owned output-provenance publication remain active TODOs.

## Verification

- Native source operations, XML inspection and mixed-quote regressions: **18/18 pass**.
- Native CLI structured-source and Login identity/compatibility fixtures: **4/4 pass**.
- Targeted authored JSON/YAML/schema/map operations: **13 checks pass**, including strict Parse under each map version and native Inspect diagnostic companions.
- Node and browser WASM package: **18/18 pass**; all **12 CEM-QL WASM scripts pass**.
- Package verification: **657 integrity records**, browser/Node ABI and all **32 schema packages** pass.
- Catalog acceptance/audit units: **47/47 pass**. Coverage records 22 authored projects, 217 authored configurations and 222 documented workflows.
- Native catalog: **254 configurations executed**: 189 passing, 31 verified Inspect outputs, 33 intentional diagnostic outcomes and one unchanged source limit; five companion limits remain separate evidence.
- Studio worker catalog: **254 configurations checked online and offline** in independent saved copies. Each phase records 217 passing executions, 33 intentional diagnostics, one pinned source limit and three explicit binary-source policy refusals. Exact offline output checks pass; originals and copies remain unchanged.
- Studio structured-source browser fixture: **1 selected case passes**, covering authored Parse, AST/events/diagnostics, independent source identities and strict failures across all three formats.
- Complete live Login recovery: **2 cases pass**, worker and fallback, including entered password/one-time-code refusal until explicitly cleared. Automatic recovery remains disabled. Existing 4 MiB checkpoint/group budgets, 32,768-node witness guard and original deadlines are unchanged.
- Full demo build, inventory publication and demo verification pass: **898 files**, **333 Studio assets**, all ten theme generators with unchanged CSS, 120 canonical playgrounds and the live tree gallery. Independent saved/offline Studio HTML/CSS preview and isolated gallery runtime checks pass.

Pinned Binaryen **132.0.0**, `-Oz`, preserves all capabilities and unchanged ABI
export/import counts. All four deployed WASM copies remain below 25 MiB:

| Runtime | Optimized bytes | SHA-256 | Exports/imports |
| --- | ---: | --- | --- |
| CEM-QL | 23,508,816 | `18a82571457a2d63cf154204dd83954500657e3103d2cd023c7ad8793da3b3be` | 169 / 28 |
| CEM-ML | 24,158,277 | `f6ad2ae8c2cb43a116123d94e6bdf2258bfc20ce6425065bc1f1136509ebce30` | 170 / 28 |

No Studio-specific behavior JavaScript or component styling is added. Shared
controls and declarative project configurations remain the UI boundary.
Generated binaries, screenshots, Vitest attachments and reports are excluded
from the source commit.
