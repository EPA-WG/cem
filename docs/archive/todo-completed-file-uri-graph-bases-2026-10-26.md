# File-URI graph module-resolution bases — completed 2026-10-26

The filesystem graph provider decoded a file-URI configuration address and
returned plain paths for relative imports. SCSS could then resolve a module
only with an additional explicit `--base-uri`, despite an admitted read map.
Native regressions first reproduced the lost scheme and failed ordinary build.

## Implementation

- [x] Preserve file-URI addresses when resolving relative resources against a
  local file-URI configuration or dependency. Encode filesystem names and
  normalize newly joined URI dot segments without canonicalizing filesystem
  targets or rewriting explicitly supplied source URIs.
- [x] Keep local glob discovery bounded and cancellation-aware. Decode file
  URIs for filesystem discovery/reads, then retain file-URI identities for the
  discovered sources. Registered virtual/remote listing and plain-path config
  behavior remain on their existing paths.
- [x] Use decoded filesystem names for folder and output bindings, retaining
  escaped source URIs for native ownership/provenance. The rooted-folder
  regression first reproduced a Unicode filename being escaped twice. Check
  URI-relative spelling before decoding and the local binding afterward, so
  encoded traversal/separators cannot bypass existing root guards.
- [x] Remove the stylesheet test helper's explicit base-URI workaround while
  retaining its bounded read map. Exercise a temporary project root containing
  spaces, Unicode and a literal percent sign. Without the module read grant,
  the native build refuses and publishes no output artifacts.
- [x] Retain an admitted `file://localhost/` authority on derived imports,
  relative dependencies and glob results. A native probe reproduced an
  authority-stripping read-map mismatch; native provider and CLI fixtures now
  verify that the localhost-prefixed module grant succeeds.
- [x] Document native file-URI graph builds and update SCSS stylesheet 1.0.2
  with the simpler CLI command. Preserve source/configuration IDs, exact
  expected CSS/HTML and existing explicit base-URI overrides.
- [x] Retain the shared native graph/import/SCSS paths and typed CSS handoff.
  No application JavaScript, duplicate parser or component behavior is needed.
- [x] Keep file-path URL construction behind the URL library's native
  filesystem capability. The first WASM rebuild exposed that API's platform
  restriction; browser command execution keeps the manifest provider and does
  not acquire filesystem access.

## Verification

- [x] All 31 native graph tests pass, including the filesystem/manifest
  provider cohort, encoded/Unicode paths, direct and recursive glob imports,
  relative resource reads, explicit source identities, rooted output bindings
  and encoded-traversal refusal.
- [x] All 16 native SCSS tests pass, including original ranges, module/caller
  scopes, resolver policies, cycles, cancellation and evaluation/output limits.
- [x] All six native stylesheet-project tests pass. The new CLI regressions
  verify exact CSS/HTML without an extra base URI, missing-grant refusal
  before publication, original file URIs in the CSS sidecar and the exact
  module `$space` definition byte range. Existing module edits, repairs and
  independent outputs remain covered.
- [x] All six CLI graph publication/source-map tests pass.
- [x] Full Studio build and its 21 dependencies pass, including both WASM
  runtimes, 59 declarative components and the 41-requirement state matrix.
  Regenerated catalog integrity, Studio type checking, the catalog audit and
  all 23 catalog acceptance contract tests pass.
- [x] Three focused CSS/SCSS/selector worker regressions pass, retaining exact
  outputs, source diagnostics and the existing virtual-resource workflow.
- [x] Entered-credential recovery refusal passes on worker and fallback
  runtimes. Entered passwords and one-time codes still block recovery;
  automatic whole-Studio recovery remains disabled.
- [x] Deployed CSS/SCSS/selector acceptance passes: exact outputs and repairs,
  persisted source/module edits, scoped previews, original provenance,
  last-good output, independent saved copies, invalid-query refusal and offline
  rebuilds. Read-only catalog originals remain unchanged.
- [x] Frozen catalog and deployment identities match after acceptance.

The frozen catalog identity is
`1750e4768e6ce95b9b4f9f0ba1241bbc31d68d210beca87e64bf0442540372d7`;
the deployment SHA-256 is
`dd7aa2e34b6f75a1d706a1594e2c1d38e0ee05db5f4ced5be6849615261e3de4`.
The catalog remains at 22 authored projects, 213 configurations, 218 documented
workflows and 32 registered native packages.

Existing plain-path configurations retain their source identity behavior;
file-URI configurations supply the imported URI bases covered here. A base or
source URI alone grants no module read capability. Tests use temporary native
project copies and isolated browser profiles, preserving user projects and
drafts. Generated screenshots, Vitest attachments, reports, binaries and WASM
files remain outside Git.
