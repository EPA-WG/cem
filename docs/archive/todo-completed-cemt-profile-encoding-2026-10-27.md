# CEMT package profiles and writer encoding — 2026-10-27

- [x] Reproduce the original CLI profile ambiguity fixture without changing its expected CEM source bytes; add failing native writer fixtures before changing profile selection.
- [x] Separate package colorizer profiles from writer encoding in the shared Rust implementation; retain explicit aliases, selector precedence and the legacy `html` request.
- [x] Share the writer's HTML decision with public output identity and final-newline handling, preserving native source owners and output spans.
- [x] Verify independent formatter/colorizer ambiguity, explicit stage ownership, CLI stdout/file output and terminal color suppression.
- [x] Verify native source formatting, both WASM bindings and Studio's independent Login formatting/coloring configurations.
- [x] Verify all catalog configurations through native CLI and deployed online/offline command services, retaining independent diagnostic limits and refusal policies.
- [x] Preserve entered-credential refusal, complete worker/fallback Login recovery and assembled demo acceptance without relaxing budgets or deadlines.

## Cause and correction

The typed writer interpreted the package colorizer profile `classes` as an
explicit HTML output selector. Selecting formatter/colorizer artifacts therefore
changed CEM source into a `<pre>` preview. A separate public-identity helper made
the same decision, and a post-writer conversion step appended a source newline
outside the preview container.

The shared native decision now distinguishes these options:

- `--cemt-color-profile classes`, `inline-style` and `css-custom-properties`
  select package artifacts while retaining CEM source bytes, MIME/schema and
  original writer spans.
- `--output-color-type` selects writer encoding, including explicit HTML aliases
  such as `classes`, `class-based`, `html.classes` and `css-vars`. Explicit
  selectors, including `none`, take precedence over the legacy `html` color
  profile shorthand.
- Live HTML conversion still materializes attributes from the selected
  colorizer. It remains distinct from an escaped HTML view of CEM source.

The typed writer owns final newlines, including those inside HTML previews;
conversion no longer appends source syntax outside its artifact. Public
MIME/schema, writer metadata and output bytes use the same native selection.
No HTML stripping, second source parser, serialized AST handoff or Studio
behavior workaround is introduced.

## Verification

- Focused native source-formatting, structured-data source, XML inspection and mixed-quote tests: **23/23 pass**. Fixtures cover palette-only CEM output, explicit HTML aliases, terminal output, exact text, Unicode, escaping, CRLF, formatter options and original writer/source spans.
- Full native CLI library: **548/548 pass**. The unchanged exact raw-CEM assertion now passes alongside independent formatter and colorizer ambiguity, explicit stage provenance, stdout/file output and `--no-color` behavior. The same profile fixture also passes with `--no-default-features`.
- Node and browser WASM package tests: **20/20 pass**, including public command-service publication, exact artifact hashes, palette/output-selector precedence and request disposal. All **12 CEM-QL WASM scripts pass**.
- Package integrity: **657 records**, browser/Node ABI and all **32 schema packages** pass.
- Studio source browser checks: **2 selected cases pass**, covering both authored Login formatter configurations and inert HTML source colors in independent copies.
- Catalog acceptance/audit units: **47/47 pass**; coverage remains 22 authored projects, 217 configurations and 222 documented workflows.
- Native catalog and all **32 schema-package examples pass**: **254 configurations executed**, with 189 passing, 31 verified Inspect outputs, 33 intentional diagnostics and one unchanged source limit; five companion diagnostic limits remain separate evidence.
- Deployed Studio worker catalog: **254 configurations checked online and offline** in independent saved copies. Each phase records 217 passing executions, 33 intentional diagnostics, one unchanged source limit and three explicit binary-source policy refusals. Exact offline output checks pass; originals and copies remain unchanged.
- Complete live Login recovery: **2 cases pass**, worker and fallback, including entered password/one-time-code refusal until explicitly cleared, fresh-generation handoff and source edits. The combined selected invocation takes 198.19 seconds; original per-case deadlines and runtime budgets are unchanged.
- Assembled demo verification passes: all owned hashes and 333 Studio assets match, independent saved/offline Studio preview works, all ten theme generators retain their CSS, and 120 canonical playgrounds plus the live tree gallery pass. Verification uses fixture-owned copies and preserves users' projects and drafts.

The full demo build and inventory publication pass. Deployment retains **898
files**, including **333 Studio assets**, with all four WASM copies below 25 MiB.
Pinned Binaryen **132.0.0**, `-Oz`, preserves all capabilities and unchanged ABI
export/import counts:

| Runtime | Optimized bytes | SHA-256 | Exports/imports |
| --- | ---: | --- | --- |
| CEM-QL | 23,497,021 | `300a390723c800e76b5d6021848a48fa9220451c57f6df06e48ed69426bbd4c4` | 169 / 28 |
| CEM-ML | 24,146,496 | `a3f5f6ea48a6cc11b40157aca1430b8d7a25d31b85f20d7d4da733f20269908f` | 170 / 28 |

No production UI JavaScript or component styling is added. Credential controls
still block recovery while passwords or one-time codes are entered, until the
user explicitly clears them. Automatic recovery remains disabled; the 4 MiB
checkpoint/group budget, 32,768-node witness guard and original deadlines remain
unchanged. Generated binaries, reports, screenshots and Vitest attachments are
excluded from the source commit.

The independent native `{.busy}` conversion loss remains an active TODO. This
completion does not assert that all native integration suites pass.
