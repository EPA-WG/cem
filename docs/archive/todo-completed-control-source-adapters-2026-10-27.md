# Typed control source adapters — 2026-10-27

- [x] Add failing native source-operation and CLI fixtures for command JSON, run-config JSON and Studio project JSON/CEM before changing their source adapters.
- [x] Retain original native syntax with existing typed codecs, preserving source identity/ranges, literal values, malformed/version/duplicate refusals and repair.
- [x] Add standalone run-config lifecycle validation and shared source Parse/Inspect/format routing without executing or preparing authored commands.
- [x] Verify public WASM/worker and independent saved/offline Studio sources, retire the portable-project Inspect exception with exact acceptance evidence, and preserve credential refusal and all recovery limits.

## Cause and implementation

Command source validation already used its native envelope codec but retained no
source syntax for Parse/Inspect or JSON export. Studio project loading retained
the typed project model, while source inspection fell through to generic CEM
and HTML schema validation. Run configurations had an execution loader and
normalizer, but no standalone source adapter. The native baseline fixtures
reproduce all three boundaries, including the portable project's obsolete
unresolved namespace/element diagnostics.

Shared Rust lifecycle adapters now retain a `ControlDocumentAst` containing the
typed command/run-config/project model and original JSON or CEM syntax. CEM
project inspection shares the codec's actual parser allocation. JSON sources
retain their original native syntax owner rather than serializing the model
back to JSON as a parser input. The existing retained-tree presentation exposes
AST, events, tree, summary, diagnostics and source offsets with the actual
source identity, URI and ranges. Refused inputs have no fabricated source tree
and cannot fall back to generic CEM tokenization or HTML schema validation.

Standalone run-config validation reuses its authored parser and semantic
validator. It does not apply execution defaults, normalize resource addresses
or read configured inputs. Command/run-config formatting uses the existing
native JSON writer, preserving submitted fields and literal argv/paths.
Project formatting and cross-encoding conversion retain the established codec
and its reference-path normalization; existing project goldens remain intact.

These source operations do not execute argv, validate argv against the command
grammar, prepare a run plan, import the inner project or fetch its resources.
The existing command/plan inspector and native preparation/execution pipeline
remain separate. Malformed/forward-version/forbidden-state diagnostics retain
their native codes, severity, field paths and absent coordinates where supplied.

CLI Parse-only failure level retains its existing semantic-error exit policy,
with the original diagnostic and no fabricated tree. Strict Parse fixtures use
`--fail-level validate`. Duplicate run inputs retain the original fatal
`cem.run_config.input_uri_duplicate`: an ordinary source command fails with
exit 1 at validation failure level, while `validate --config` continues to
refuse preparation with exit 2 before attempting input reads.

Inspect diagnostic and summary views retain their existing readable-report
policy even when native errors exist; AST Inspect and strict Parse refuse the
source. The catalog's source Parse configurations explicitly select validation
failure level. Neither exit 0 from a diagnostic view nor parse-only mode is
treated as proof of semantic validity.

Parse/Inspect retain source coordinates in their presentation, but common
publication still does not supply fine artifact-source navigation endpoints.
No application invents Provenance links from numeric AST/source IDs.

## Studio and catalog

Portable configurations **1.0.1** adds ten ordinary source configurations:
command Parse/Inspect/format, run-config Validate/Parse/Inspect/format, JSON
project Parse/Inspect and CEM project Parse. Existing project/resource and
configuration IDs remain stable. The project README and catalog workflow
inventory document their actual operations and the separate execution boundary.

The sole remaining catalog source exception for
`portable-config/inspect-project-cem` is retired. Its replacement unit check
refuses attempts to classify the obsolete generic-source errors as accepted
native success. Source assertions and diagnostic severity are not weakened.

## Verification

- Native source/codec regression cohort: **39/39 pass** — six new control-source fixtures, six command-codec fixtures, fifteen project-codec fixtures, eight structured-data source fixtures and four original-source diagnostic fixtures.
- The new native fixtures cover original selectors/ranges, literal Unicode/CRLF values, all Inspect views, format round trips, typed native ownership, shared CEM parser allocation, root-scope identity, cancellation, malformed/unsupported selectors, version/duplicate refusals and repair.
- Final control-source cohort: **6/6 pass**, including explicit native writer MIME/schema assertions.
- Targeted CLI control-source and Login suites: **7/7 pass**, including strict Parse validation, the separate parse-only exit policy, source refusal/repair and `--config` preparation refusal before input reads.
- Full native CLI library: **549/549 pass**.
- Public WASM package: **24/24 pass**, including command/run-config/project Parse, AST Inspect, Validate, Check and format, original native refusals and repair in both Node and browser bindings. Resource-fetch callbacks reject attempts to execute argv, read project resources or transport credentials.
- Package verification passes for browser/Node ABI, **657 integrity records** and **32 schema packages**.
- All **12 CEM-QL WASM regression scripts** pass, including runtime isolation, source reimport, allocator/headroom, native checkpoint, CSS ownership/reuse and URL transport cases.
- Studio browser cohort: **5/5 pass**, including two independent roots for all four control encodings, strict Parse/AST Inspect refusal and repair, readable native diagnostics, existing command/plan preparation, project-relative graph execution, original source diagnostics and the Login live view.
- Catalog acceptance/audit units: **48/48 pass**; documentation audit retains **22 authored projects, 227 configurations, 232 documented workflows and 32 native packages**.
- Assembled demo verification passes: **898 files**, all **333 Studio files**, isolated gallery runtimes, independent saved/offline HTML/CSS preview, **10 unchanged CSS generators** and **120 canonical playgrounds** plus the live tree gallery.
- Complete native catalog acceptance: **264 configurations**, **231 passes**, **33 intentional diagnostic cases**, **zero known source limitations** and **zero companion exceptions**; the seed and **32 native schema-package examples** pass.
- Complete worker catalog acceptance: **264 configurations online and 264 offline**. Each phase retains **228 passes**, **33 intentional diagnostic cases**, **three binary execution policy refusals**, **zero known source limitations** and **zero companion exceptions**. Original projects and saved copies remain unchanged; offline output evidence passes. All **21 Portable configurations** run in both phases.
- Complete live Login recovery: **2/2 pass in isolation**, worker and fallback, with original credential refusal, checkpoint/group budgets, witness guard and deadlines. The earlier invocation overlapping full catalog/package verification passed worker recovery but hit `cem.checkpoint.resume.timeout` at fallback resume 0. The unchanged isolated two-case invocation passes; the existing concurrent release-verifier scheduling TODO retains this observation rather than claiming the overlapping run green.

Native and worker catalog reports share identity
`ef8639959060e16d4b85fa6265305de9feb3f8772d741fd2f594179b78b7490c`.
Both complete optimization passes produce identical bytes. All four deployed
WASMs remain below **25 MiB**, with all capabilities retained. Binaryen
132.0.0 `-Oz` preserves their import/export inventories:

| Runtime | Optimized bytes | SHA-256 | Imports / exports |
| --- | ---: | --- | --- |
| CEM-QL | 23,713,829 | `d31a5cb0597e4163ee4295968302de96a3cb5b2b80c7036bfa62e225ba5ebff4` | 28 / 169 |
| CEM-ML browser and Node | 24,385,572 | `383cd8180c7c1762eb01ed21d32c46aee0c9d87a56b68fda08cf01ea9ebcf07c` | 28 / 170 |

## Shared components and credential privacy

Production changes are shared native library code. Studio uses its existing
declarative components and common command-service results. There is no authored
application/component UI JavaScript, custom JSON/command grammar, extra page or
component-specific source-operation workaround.

Entered Login passwords and one-time codes continue to block recovery until
explicitly cleared. Automatic whole-Studio recovery remains disabled. The
existing 4 MiB checkpoint/group budget, 32,768-node witness guard and original
deadlines remain unchanged. Saved projects/drafts are not cleared or rewritten.
Generated binaries, reports, screenshots and Vitest attachments stay outside
the source commit.
