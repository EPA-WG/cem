# SCSS mixin default-argument binding — completed 2026-10-26

Imported mixin defaults previously evaluated against the caller's variables.
The new native regressions first reproduced a caller shadow replacing a module
default, unresolved module variables, and missing interpolated default output.

## Implementation

- [x] Evaluate omitted positional mixin arguments in a fresh captured module
  variable environment, adding each parameter as it is bound. Later defaults
  can use earlier parameters, including explicit values and zero. Repeated
  calls and aliases do not retain previous arguments.
- [x] Evaluate supplied argument expressions in the original caller scope,
  independently of parameters already bound for this invocation. Preserve the
  existing local-mixin scope profile.
- [x] Attribute default expressions/interpolation to the defining module and
  retain definition, parameter and caller frames. Forwarding and aliases keep
  the original defining module. Native Unicode/CRLF fixtures verify original
  module definition and caller byte ranges.
- [x] Keep the native `CssDocumentAst` handoff, resolver grants, cooperative
  cancellation, recursion/work/output limits and CSS-owned serialization.
  There is no generated-CSS reparse or application-side Sass implementation.
- [x] Update the ordinary SCSS stylesheet project to version 1.0.1:
  `inset($amount: $space)` is called through `@include tokens.inset()`.
  Expected CSS and HTML stay unchanged. Rebuild its catalog archive and index.
- [x] Document this focused supported feature in the SCSS package README,
  conformance matrix and project README. Full local-variable isolation,
  function defaults, captured callable-member environments and content-block
  scoping retain their existing profile limits; full Dart Sass parity is not
  claimed.
- [x] Correct the native stylesheet-project cohort's obsolete selector
  configuration count from five to seven without weakening output,
  diagnostic, repair or provenance assertions.

## Verification

- [x] Native SCSS integration: all 16 tests pass, including the three new
  regressions for module defaults/caller shadows/earlier parameters,
  Unicode/CRLF origins and forwarding/aliases/repeated calls. Existing
  resolver refusal, cycle, abort and budget checks remain passing.
- [x] SCSS package verification, all nine CLI SCSS tests and native converter
  handoff parity pass.
- [x] All four native stylesheet-project tests pass. They execute every
  authored CSS/SCSS/selector configuration with exact results and include a
  saved module edit, omitted defaults and explicit caller-variable arguments.
- [x] Full Studio build and its 21 dependencies pass, including both WASM
  runtimes, 59 declarative components and the 41-requirement state matrix.
  Studio type checking, the catalog audit and all 23 catalog acceptance
  contract tests pass.
- [x] Three focused CSS/SCSS/selector worker cases pass. The SCSS case verifies
  exact unchanged default CSS despite a caller shadow and exact caller-scope
  CSS when the argument is supplied explicitly.
- [x] Credential recovery refusal passes on worker and fallback runtimes.
  Entered passwords and one-time codes continue to block recovery; automatic
  whole-Studio recovery remains disabled.
- [x] Deployed CSS/SCSS/selector acceptance passes: saved module edits,
  original source identities/ranges, exact previews, last-good output,
  independent clones, invalid-query refusal and offline rebuilds. The SCSS
  preview has 12px padding from the default and 16px after the saved module
  edit. Read-only catalog sources remain unchanged.
- [x] Recheck the frozen catalog identity and deployed bytes after acceptance;
  both remain unchanged.

The frozen catalog identity is
`8ce758fd0ec504bbcb57c1d7a02ac0d3863af20f553a16c814bcde2e3390c8b9`;
the deployment SHA-256 is
`970473121b80689e5577d69dc1b6a2aeeadfee5d34660594d42491b848fd7139`.
The catalog remains at 22 authored projects, 213 configurations, 218 documented
workflows and 32 registered native packages.

No Studio UI behavior or component JavaScript was introduced. Tests use
temporary project copies and isolated browser profiles, preserving user
projects and drafts. Generated screenshots, Vitest attachments, reports,
binaries and WASM files remain outside Git.

The CLI's admitted file-URI module-resolution base remains the next separate
task; this change preserves its explicit base/read-map workaround.
