# Native projection projects and shared validation, 2026-10-25

- [x] Add an ordinary project comparing parser-owned CEM AST/events, retained
  XML/plain-text inspection and explicit native DOM/AST/event exports.
- [x] Preserve native CLI export fixtures, cross-host binary bytes, original
  UTF-8 ranges and four diagnostic/repair buffers.
- [x] Route registered projection source validation through the common native
  engine without enabling document import, decoder/replay or binary text parsing.
- [x] Guard original catalog binary previews and use declarative shared controls
  to download exact owned bytes; keep saved binary inputs non-runnable.
- [x] Repair duplicate multiline editor input and ancestor checkpoint admission
  in shared `cem-elements`, preserving native editing and composition boundaries.
- [x] Preserve authored terminal LF/CRLF through native whole-source selection,
  paste and undo using the shared editor's disposable caret placeholder.
- [x] Verify final worker, deployed, saved-copy and offline workflows and archive
  completion evidence while preserving drafts, catalog originals and credentials.

## Ordinary project and ownership

[Native DOM, AST and events](../../examples/projects/native-projections/README.md)
contains **15 authored resources and 19 configurations**. It appears in Schema
authoring and Queries and transformations as a normal cloneable project.
It reuses shared source editing, command/result inspection, provenance selection,
structured data presentation and downloads. No example page, component-local
JavaScript behavior or JSON AST execution handoff is introduced.

The original CEM source is 162 UTF-8 bytes. Parser AST/events use native CEM-ML
presentation, not implicit JSON. Retained XML inspection preserves namespaces,
comments, CDATA and its original buffer; plain text uses the existing native
loader and CEM-ML view without claiming a registered schema or formatter.

`Café Noord` occupies 11 bytes at offset 147 in the CEM source. Native DOM debug
metadata retains its broader original range; its source-map stack and lexical
events retain the finer range. Tests preserve both facts. Imported projection
JSON contains values describing these ranges, not an original source owner or
permission to navigate arbitrary buffers.

All six explicit export targets have exact native CLI fixtures. Shared worker
JSON publication currently serializes the same complete value compactly, while
the CLI indents it. The three JSON configurations declare this comparison mode;
no fields or source facts are omitted. Binary bytes match across hosts, and
same-host repeat/offline outputs remain exact. Native sealed chunk replay is
tested through typed artifact APIs after the source owner is dropped, including
contiguous bytes, child identities, source deltas and changed-input hashes.

## Shared native repair and preserved limits

The CLI already used the registered projection validators. The common engine
previously fell through to its output-only lifecycle adapters, producing
unsupported-adapter and text-tokenizer errors for otherwise valid sources.
Failing common-engine regressions precede the new validation-only dispatch.

Common Validate/Check now reuse the existing native DOM/AST/event JSON shape and
binary-header validators. Explicit context stages retain precedence. Selection
uses registered MIME/schema identities, including the existing DOM alias;
conflicting identities remain refused. Optional source-asset admission remains
required. Binary input reaches these validators as bytes, without UTF-8 decoding.

Generic projection input adapters remain output-only. Validation creates no
retained parser AST or replay capability. Binary validation checks headers, not
complete payload integrity. The native AST debug writer still exports an array,
while the registered JSON validator requires a document object. The unchanged
native array and separate authored object demonstrate that boundary; no wrapper
or artificial round trip conceals it. Native follow-ups remain in the active TODO.

Parse/Inspect's typed retained owner and coordinate presentation also remain
distinct from common command provenance publication. The current native
presentation/result boundary drops named source-map references; the Provenance
panel is correctly empty. Deployed source selection instead follows a real native
`repair-source` diagnostic against its admitted original CEM buffer, including
UTF-8-to-UTF-16 selection checks. Neither Studio nor the example turns serialized
inspection fields or numeric source IDs into owned source links. Native
presentation-provenance publication is an explicit follow-up.

The three JSON-kind failures retain native shape codes and null text coordinates.
The unclosed CEM source retains all three original errors, including the tokenizer
EOF at line 8, column 1, byte 160. Its UTF-16 offset differs because of `é`.
Each repair restores its exact valid buffer.

## Binary source and presentation policy

Catalog file transport now applies the existing shared preview policy before
text decoding or native source preparation. Binary/unknown/invalid-text resources
remain navigable and download exact owned bytes. Their declarative project-picker
view uses shared CEM alert/action components instead of a source editor.

Saved binary resources remain non-editable and non-runnable. Explicit binary
exports from owned text source use the existing download result view, refuse
clipboard text copying and preserve the `.cem-bin` extension. This is file
transport and native command orchestration; visible UI remains declarative.

## Verification and privacy

The deployed JSON repair exposed a shared editing defect: Chromium emitted 1,987
trusted input fragments after one 994-line insertion had already installed its
complete text. This repeated host publication and ancestor checkpoint work. The
large edit also exceeded its existing acknowledgment timeout. A small
native-input regression reproduces 31 publications for 16 lines; a nested-owner
checkpoint regression reproduces 32 epochs instead of two.

The shared plaintext capability now classifies each native event consistently
across ancestor owners and the editor. Only identical fragments within that edit
are suppressed. The first notification stays synchronous; separate same-value
edits, explicit synthetic inputs and composition boundaries retain their
observable lifecycle. Native DOM, focus, selection and undo stay browser-owned.
There is no Studio-specific input workaround or increased timeout. The focused
nested-owner regression passes on both worker and fallback runtimes.

A native CPU profile attributes 29 of 30.6 seconds to browser work, versus about
one second reading the control. A timeline trace records more than 1,400 layout
and style-tree passes in its first ten seconds, repeatedly laying out the entire
document while native insertion grows the editor subtree. Bounded JavaScript
publication alone therefore does not establish acceptable large-edit latency.

Both experimental editor containment profiles still exceed the original
ten-second limit and are discarded. A diagnostic native clipboard paste of the same
24,585-character buffer takes **93 ms** (114 ms including the settled snapshot),
with one trusted `insertFromPaste` input, one text node, exact source and the same
editor. The slow path is CDP's multiline `insertText`, which constructs 1,987
fragments and takes about 29.3 seconds. Deployed bulk-edit acceptance uses native
paste with exact-text assertions; it does not assign model state, alter editor
DOM or increase timeouts. The automation-path performance limit remains recorded
in the active TODO. No speculative containment CSS is shipped.

The deployed AST repair also exposes a separate terminal-newline defect:
Chromium treats an external value's last LF as an invisible caret line, so native
Select All/paste retains it. Even an explicit full DOM range leaves that LF.
External values ending LF now receive the same disposable terminal BR used by
native editing. The source reader preserves exact text beside it, including raw
CRLF. A new regression fails before the repair and passes for six LF/CRLF buffers,
readonly rendering, complete native selection, paste and exact native undo/redo.
Unrelated renders retain the same control and markup; no caret BR becomes source.
Coloring asserts unchanged markup with one text node and its sole terminal BR;
syntax paint still introduces no DOM markup.

The native authored/common-engine suite passes **eight tests**, covering all 19
configurations, exact CLI exports, four repairs, typed chunk replay, source
ownership, binary header refusals, explicit identities and registered-stage
precedence. Generic load remains output-only after successful validation.

The catalog binary regression and two text-availability regressions fail before
the transport repairs and pass after them. Missing bytes and invalid UTF-8 beyond
the preview prefix never become empty source input. All **26 navigation tests**,
**52 worker tests**, **nine source-editor stories** and **four selected
checkpoint stories** pass. The checkpoint stories cover empty/entered credential
refusal, portable editor state, native multiline owner admission and colored
editor resume on fresh runtimes. Focused authored-source ESLint has zero errors
(existing and colocated-test lint warnings remain); Studio type
checking and the two new native files' formatting checks pass.
The workbench suite passes 180 cases; its remaining graph-edge form case reaches
the existing 15-second timeout during parallel native compilation and passes in
isolation in 10.33 seconds with the same timeout. No assertion or timeout is
weakened. Both focused common-engine tests also pass with `external-schema-assets`
in a separate native target, preserving the complete CLI build and optional CSV
missing-source refusal.

The native catalog verifier passes all 32 registered schema-package examples and
records **191 existing workflow entries**. Output verification passes **218
declared assets / 332 files**. Two clean builds match with aggregate SHA-256
`07a7823a3e172c32602dc5c5b3add9596633d30117e57c903112049b01324f3a`.
The final static file inventory and every byte hash match that clean-build report.

All **24 deployed acceptance flags** pass: both catalog groups, readonly original
sources, all 19 configurations and four repairs, exact binary transfers and copy
refusals, native diagnostic selection, saved Unicode/range invalidation,
independent copies and complete offline commands/downloads. The final catalog
README bytes match the original file too. Final UI acceptance runs the target's
configured Node script directly to preserve live progress; after editor-only
repairs, existing native/CLI evidence is retained while Studio, two clean static
builds and the affected browser/checkpoint gates are rerun. Verification errors
now print before bounded state/screenshot diagnostics; a busy browser cannot hide
the initial failure behind those observations.

Acceptance uses isolated browser storage and temporary saved copies; it never
clears user projects or drafts. Entered passwords and one-time codes continue to
block recovery until explicitly cleared. Automatic whole-Studio recovery remains
disabled. Generated screenshots, Vitest attachments, reports and WASM builds stay
outside Git; the three small authored binary fixtures are intentional sources.

Continue with the catalog README/coverage audit and full catalog acceptance.
