# Studio project README links and file guides

Project homes now render **README** from the existing project-tree data,
using `cem-icon-button` and the normal declarative `open-entity` binding. The
root filename is matched case-insensitively; resource IDs are not hardcoded.
Samples, saved clones and new starter projects share this view. Existing saved
projects and drafts are not migrated or overwritten.

All 22 authored sample guides, the schema showcase and the starter begin with a
title, a short plain-language purpose paragraph and **Essential files**. Each
file link has a purpose and names an included project resource. The showcase
lists all 32 selected content-type examples, their actual served filenames and
the four portable-operation input/template/graph files. Existing detailed
configuration, identity, expected-output, repair and native-limit documentation
remains available below those introductions.

Native Markdown conversion remains the only Markdown-to-HTML path. Its bounded
HTML output uses the existing shared passive artifact preview and its link
event. Studio's routing transport accepts only known paths in the current
project, preserving collection/sample or saved-copy identity, build selection,
read-only originals and draft guards. Unsupported addresses leave the selection
unchanged. Fragment-only navigation stays in the shared preview. Registrations
are disposed on replacement, unsupported source changes and service disposal;
same-source pending/error states may retain explicitly stale success.

The shared host-command adapter now handles primary anchor activation without
a duplicate native page navigation. It reads an explicit bound link value or the
raw href when no value is declared. Native/form-control value ownership and
input/authority revalidation remain first. Modified/non-primary activation and
unhandled links retain native browsing. The behavior belongs to `cem-elements`,
with declarative Studio bindings; no application DOM handler or parser was added.

## Completed checklist

- [x] Expose README navigation on sample, saved-copy and starter project homes.
- [x] Require a title, purpose paragraph, essential file links and descriptions
  in catalog documentation, rejecting missing, external or unavailable files.
- [x] Check every showcased type and exact served filename against catalog
  transport metadata, with refusal fixtures for omissions and stale names.
- [x] Cover shared routed anchors, modifiers, nested values/labels, raw href
  fallback, unhandled links and existing authority/input behavior.
- [x] Reuse passive shared previews, bounded registrations and known-resource
  routing; verify original-buffer Markdown conversion and registration release.
- [x] Verify home URLs/tree selection, no page reload, source file links, build
  retention, saved edits/copies, draft cancellation and cold offline behavior.
- [x] Regenerate catalog archives and refresh the demo; retain credential privacy
  and all existing recovery budgets/deadlines.

## Verification

- Catalog audit: 32 tests pass, including the new minimum-documentation,
  complete showcase and exact served-name checks. Full generated catalog audit
  passes for 22 authored projects, 247 configurations, 252 workflows and 32
  native packages.
- Shared host-view stories: all 16 pass on worker/fallback, including routed
  anchor cases. The original fixtures reproduce duplicate default navigation;
  real Studio acceptance also exposed missing route values on link-mode controls.
- Studio navigation/source-preview browser tests: all 40 pass, including native
  Markdown/table conversion, scope/version cancellation, passive script removal,
  navigation boundaries and replaced-registration revocation.
- Catalog browser acceptance passes for all 23 catalog README resources and the
  source-first starter. Every home link opens the README without reloading;
  source links keep the selected build and work offline. All 32 showcased file
  links exist. Saved edits, independent copies, guard cancellation and unsaved
  drafts survive catalog browsing and offline reload; originals remain read-only.
- Studio lint passes. Changed shared host-view modules have no lint errors or new
  warnings; four pre-existing non-null-assertion warnings remain in older stories.
- Studio/demo builds pass. Deployment remains 898 files (333 Studio files), all
  owned hashes match, all four WASMs remain below 25 MiB, all ten theme CSS
  outputs are unchanged, and Studio subpath/native preview/offline, galleries,
  120 playgrounds and the live tree gallery pass.

- Final isolated live Login recovery passes on worker and fallback (2/2 cases,
  six unrelated cases skipped): three recoveries, twelve edits and two credential
  refusals per mode, with checkpoints within 4 MiB and no deadline changes.
Entered passwords and one-time codes continue to block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. The 4 MiB group budget,
32768 witness bound and allocation/resume deadlines are unchanged. Generated
screenshots and Vitest attachments remain ignored.
