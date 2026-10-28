# Formatter and HTML/XML conversion provenance

Native formatter views name original typed source metadata before package
formatting. This includes XML/HTML families, JSON/JSON Schema, YAML, CSV,
Markdown, CSS, XPath and CSS selectors. Borrowed records/sequences retain their
native owner and delegate traversal; original syntax is not serialized into an
AST transport. Existing foreign/module boundaries remain unchanged. Expanded SCSS frames retain their dedicated
module URIs. Only a frame whose module URI matches the original input can supply
that input's coordinates; foreign module ranges cannot be relabeled as root
input. A mixed-origin fixture verifies selection of the matching native callsite,
with all original frames retained. Ordinary
JSON data and generated layout cannot create a source endpoint.

The conversion pipeline retains its final typed writer payload alongside the
explicit public/debug projection. Direct HTML/XML conversion and registered
converter pipelines use that payload for publication, with native fallback
captured at its producer. The common witness binds original input URI/bytes and
final writer bytes. Markdown writer tokens now preserve their native source map
when they have no explicit output span; explicit spans take priority and the
no-map writer policy remains authoritative. Coloring and escaping retain actual
final output byte ranges, without promising linear character equality.

The ordinary Native DOM, AST and events project version 1.0.3 adds `format-xml`,
`render-html` and `render-xml`. Source selection and HTML preview reuse the
existing shared command/result and declarative Studio components. No new
application UI JavaScript or resource authority is introduced. Native projection
input adapters, binary payload validation and read-only binary command admission
remain separate follow-ups.

Entered Login passwords and one-time codes continue to block recovery until
cleared. Automatic recovery stays disabled. The 4 MiB checkpoint budget,
32768-node witness limit, deadlines and complete bounded provenance selector
remain unchanged.

## Verification

- Native fixture failed first on anonymous XML formatter output. Fourteen
  publication paths now cover same-schema formatting, CEM-to-HTML/XML,
  JSON/YAML conversion, CSV-to-XML and SCSS-to-CSS, including UTF-8, CRLF and escaped text.
- Colored JSON, Markdown and XML fixtures cover two independent original URIs,
  terminal/HTML writers, actual output bounds and source/output mutation refusal.
- Native naming guards preserve borrowed source metadata, foreign/module
  boundaries and unmapped generated values. Registered HTML fallback retains
  exact native bytes and an original-buffer witness.
- Native common-command tests: 70 pass. Native conversion cohort: 161 pass with
  the independently reproduced reader-trace test below excluded. CLI: 550 pass.
- Catalog audit: 22 authored projects, 234 authored configurations, 239
  documented workflows and 32 native schema packages. Catalog unit checks pass
  48/48; authored Studio source/script lint passes.

The initial release optimization was stopped after review exposed the SCSS
module-origin ambiguity. A failing foreign-module naming fixture established the
risk; the shared boundary selector was corrected before the replacement release
build. No stale deployment from that attempt was published.

## Independent existing failure

`conversion_output_pipeline_reads_cemt_artifacts_through_environment_reader`
expects four reads but receives eight, including repeated native helper reads.
The identical ordered read trace and failure reproduce on unchanged commit
`457f3818` in an isolated clean worktree. Neither source-reader behavior nor
that assertion was changed here; the follow-up is recorded in the active TODO.

## Release integration

The corrected release is published with 898 managed/total files, including all
333 Studio files. Binaryen 132 `-Oz` produces CEM-QL at 23,899,308 bytes and
CEM-ML at 24,576,778 bytes; all four deployed WASM files remain below 25 MiB.

The public browser/Node distribution verifies its ABI, 657 integrity records
and all 32 packages. WASM runtime tests pass 28/28, including the formatter,
HTML/XML, SCSS and stale-publication cases on both targets. All 12 CEM-QL WASM
scripts pass. Studio's focused workbench cohort passes 32/32, and the real native
projection/formatting/conversion export test passes. The initial overlapping
cohort had a 15-second cold test deadline; it passed unchanged on rerun. A
subsequent stale-provenance fixture now explicitly awaits its repository-driven
navigation before closing its test database. No assertions or deadlines changed.

The deployed demo verifier passes complete managed hashes, paired runtime bytes,
isolated factory ownership, native rendering/highlighting, saved-copy HTML/CSS
preview and offline rebuild. All 10 theme generators retain exact CSS, and all
120 canonical playground pages plus the live tree gallery pass.

The complete native catalog executes all 271 configurations: 235 succeed and 36
produce their intentional diagnostics, with zero known limits, companion
diagnostic limits or policy refusals. Its catalog identity is
`557c35747ec44b82b14fc6713824c20befe393e6cd7f8b822ad12a8cc336c0d8`.

Saved/offline native-projection navigation passes all catalog configurations,
original diagnostic and formatter/conversion source selection, exact binary
transfers and read-only refusal. Unicode edits invalidate previous ranges;
independent copies rebuild and select their own original input ranges after
offline reload. Repairs persist and original catalog bytes remain unchanged.
The verifier explicitly includes all three new configurations in both online
and offline source-selection checks.


Isolated complete live Login recovery passes on both worker and fallback modes.
Each mode performs three recoveries and twelve edits, verifies two credential
privacy refusals, and checks both checkpoint/group bytes against the unchanged
4 MiB cap. The 32768-node witness and original deadlines remain unchanged.
The source-naming guard, original boundary test and nine native SCSS checks also
pass after the module-coordinate correction.
