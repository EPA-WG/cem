# Studio field captions and output navigation

- [x] Apply the canonical tag typography role to shared `cem-select` field
  labels. Keep the UI role for values/help and `--cem-gap-related` between
  caption and control; no local color, spacing or target-size defaults.
- [x] Cover label typography, non-interactive semantics, accessible naming and
  geometric grouping across four themes and four density profiles.
- [x] Cover output presentation with a common `generated/` folder, another
  common folder, root files, mixed folders and absolute artifact addresses.
  Verify nested disclosure, retained selection and full-path URL navigation.
- [x] Omit the synthetic output root and lift the children of one sole top
  directory in the declarative Studio view. Preserve domain trees, complete
  artifact paths, legacy folder URLs and genuine nested folders.
- [x] Update the navigation contract and Markdown-site acceptance to check the
  visible artifact roots rather than expanding a hidden `generated/` row.

All output trees share the synthetic domain root; the enclosing directory comes
from admitted artifact paths and is not required to be named `generated`.
Neither tree data nor persisted selection identities are rewritten. The change
adds no application or component JavaScript behavior.

## Verification

- Shared select: all 15 stories pass. The new fixture fails on the original
  equal-sized label/value typography and passes with the canonical tag role.
- Studio navigation/source-preview: all 45 tests pass. The five output-root
  cases fail against the original presentation; all five also pass after
  adding nested collapse/selection and hidden-folder URL checks.
- Rebuilt CSS stylesheet example: 14px/700 label, 16px value and 4px related
  gap. Forced colors preserve readable inherited field captions and accessible
  dropdown naming. `cards.css` and `cards.html` are level-one rows; selecting
  the latter retains `artifact=generated/cards.html`.
- Markdown-site browser acceptance passes: catalog and saved builds, source
  navigation, metadata repair, retained output on failure, portable import,
  pinned previews and offline rebuilding. Both output-tree checks pass with
  `index.html` directly under the tree and no redundant root rows.
- Demo build/copy and deployment acceptance pass: 898 files, all owned hashes
  match, all four WASMs remain below 25 MiB, all 10 theme CSS outputs remain
  unchanged, and Studio, 120 playgrounds and the live tree gallery pass.
- Studio lint passes; select stories have no lint errors or new warnings.
  Existing non-null assertions in older stories remain outside this change.

Credential privacy, checkpoint budgets and automatic recovery policy are
unchanged. Browser checks use isolated profiles, preserving saved user projects
and drafts. Generated screenshots and Vitest attachments remain ignored.
