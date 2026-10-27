# Lossless mixed-quote CEM-ML inspection — 2026-10-27

- [x] Reproduce the native mixed-quote scalar refusal before changing the tokenizer/writer contract.
- [x] Add native round trips for mixed quotes, embedded fences, literal references/calls, backslashes, Unicode, LF/CRLF and trailing CR, retaining ordinary quote behavior and unsupported-control/unterminated-literal refusals.
- [x] Admit line-fenced literal attribute values with collision-free delimiters and matching framing line endings through shared native code.
- [x] Synchronize lexical, Tree-sitter and TextMate grammars, including incremental editor edits and original source ranges.
- [x] Verify canonical component inspection through CLI, both WASM bindings and Studio, with credential/privacy and deployment checks.

## Representation and ownership

The native string encoder previously refused any value containing both single
and double quotes. The original `cem-card.xhtml` template contains both styles
and triple-backtick content fences, so default CEM inspection could not publish
its complete source scalar. The new native regression reproduces that refusal
before the implementation change; ordinary quoted/backslash compatibility checks
already pass.

The shared tokenizer and writer now admit the
[line-fenced attribute literal](../cem-ml-syntax.md#line-fenced-attribute-literals).
An opening run of at least three backticks is followed immediately by LF or
CRLF. Its closing run has exactly the same length and preceding line-ending
style. The canonical writer selects LF boundaries and a delimiter longer than
every payload backtick run. Only the two framing line endings are removed;
there is no escaping, indentation stripping, trimming or query evaluation.
In particular, LF framing does not swallow a payload's trailing CR.

These values remain native literal attributes with no expression/value nodes.
Their UTF-8 source ranges include the exact authored delimiters and payload.
Ordinary quoted values retain their raw backslashes and original delimiter
choice; bare inline backticks and content-plane triple-backtick enclosures keep
their established meaning. Unsupported control characters still refuse native
encoding. An incomplete or mismatched fence reports the existing schema-owned
unterminated-string fact; no new diagnostic protocol or replacement source is
introduced. The schema's literal policy and all shipped editor grammars describe
the same admitted representation.

Typed AST/tree inspection publishes these scalars through the native writer.
Native reimport preserves the complete value, original input URI and original
CDATA UTF-8 byte range. Source text that resembles references or calls remains
inert. The shared native highlighter keeps the entire fenced value in one string
span, and Tree-sitter incremental edits preserve the following attribute and
sibling under both LF and CRLF. TextMate keeps embedded queries/fences inside
the literal scope.

## Studio and shared components

HTML and CEM card **1.0.1** uses default CEM inspection again, with unchanged
resource/configuration IDs and the original canonical component snapshot. Native
CLI acceptance navigates the native inspection vocabulary and compares the
complete template text with that unchanged declaration. Studio's worker fixture
also requires the complete literal body, without a JSON-only configuration.
Explicit JSON remains a named human-facing source export; a WASM fixture uses
it only to assert the result of native CEM reimport. It never becomes an
application AST handoff or a JavaScript parser.

No Studio/component behavior JavaScript is introduced. Existing saved project
configurations remain independent, including those explicitly selecting JSON.
Entered passwords and one-time codes still block recovery until explicit user
clearing. Automatic whole-Studio recovery remains disabled; the 4 MiB limits and
existing acceptance deadlines are unchanged. Browser acceptance uses isolated
fixture-owned state and preserves user projects/drafts.

## Verification

- Native integration: **21/21 pass** (four literal/highlighting cases, two
  typed inspection cases, seven native/editor parity cases and eight existing
  XML inspection boundary cases).
- Native unit checks: **27/27 pass** (25 tokenizer/schema-diagnostic cases,
  the token encoder contract and named/default invocation preparation).
- HTML/CEM card CLI: **4/4 pass**, including the exact complete declaration
  body, authored configurations, native policy repair, independent saved edits
  and original provenance.
- TextMate: **16 cases pass**, including LF/CRLF literal scopes and following
  sibling highlighting.

- Complete CEM-QL WASM cohort: **12 scripts pass**, including XML reload,
  checked compilation/cache leases, native values, constraints, recovery,
  isolation, retained rendering/CSS and URL operations.
- Common CEM-ML WASM package: **16/16 pass**. Both Node and browser bindings
  inspect mixed-quote XML and natively reimport the published CEM before an
  explicitly selected JSON assertion export. Exact literals, highlighting,
  publication hashes, literal ownership and handle cleanup pass.
- Package verification: both ABIs, **657 integrity records and 32 schema packages**.
- Studio source worker acceptance: **2 selected cases pass**, covering canonical
  component inspection, all card configurations, semantic parity, native policy
  repair, saved source/template edits, exact HTML, build origins and independent
  saved data. This is focused acceptance, not a new full 276-case source run.
- Credential-refusal browser story: **1/1 pass**.

Assembled-demo acceptance passes: all owned hashes and all 333 Studio files
match the build, paired runtimes retain independent memories, native rendering
and highlighting work, Studio selection/saved-copy/HTML-CSS preview/offline
rebuild and worker scope pass, all ten theme generators retain their frozen CSS,
and all 120 canonical playgrounds plus the live tree gallery pass.

The assembled deployment has **898 files, 114,690,057 bytes (109.38 MiB)**,
including all **333 Studio files**. All four WASM files remain below 25 MiB:
CEM-QL is **23,513,463 bytes** per copy and the common bridge is **24,163,453
bytes** per copy. All 169/170 exports, 28 imports, default debug-control and all
32 schema packages remain available. Binaryen 132.0.0 `-Oz` and inventory/hash
cleanup policies are unchanged.

Optimized hashes:

- CEM-QL: `4ad58d018b95f001399a15fa5d70ea26d13239eadb85e0498ad38f7e7824edbf`.
- Common bridge: `ea7153b339f4b1184d9a753ba31d7cf1bcb01afc9a0aa133a18bd3c6b7166e56`.

Generated WASM, native build output, screenshots and Vitest attachments are not
included in the source commit. Generated Tree-sitter parser sources are shipped
source assets and are updated with their grammar.

The independent Login source-identity, JSON/YAML Parse and formatter-profile
conformance issues remain in the active TODO list. This change does not claim
those repairs, generic AST JSON validator compatibility, or newly owned
artifact-source navigation endpoints.
