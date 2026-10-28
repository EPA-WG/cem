# Direct configuration URL text admission — 2026-10-28

## Completed contracts

- [x] Add direct configuration URL fixtures for unavailable resource bytes,
  invalid UTF-8 (including beyond the preview prefix), binary/unknown content
  types, oversized active markup, valid empty/Unicode text and exact downloads.
- [x] Refuse a missing configuration resource without losing the previous URL,
  tree selection or saved-project draft.
- [x] Reuse catalog file text admission for configuration routes and verify the
  existing declarative read-only source/download controls and offline behavior.

Previously `sample=…&configuration=…` decoded missing bytes as empty text and
replaced invalid UTF-8. Catalog file and configuration routes now call one
transport helper: source bytes must exist, the declared preview type must be
admitted by the existing bounded policy, and the complete source must decode as
strict UTF-8. A prefix-only preview cannot admit malformed later bytes.

Valid empty text remains available. Unavailable bytes never become an editor or
native preparation source; unsupported binary/unknown types and inadmissible text
retain their original bytes, MIME and download behavior. Missing configuration
resources refuse navigation before selection changes. These are transport checks;
configuration syntax, schemas and execution admission remain native concerns.
No JavaScript configuration parser or application-local UI behavior was added.
The production declarative project picker, shared source editor, alerts and
actions are unchanged. Theme tokens, source/result limits, checkpoint budgets,
node witnesses and all allocation/resume deadlines remain unchanged.

## Verification

- Initial fixtures reproduced seven failures; both valid-text cases already
  passed. Final complete navigation suite passes 37/37, including nine new cases.
  Assertions cover actual production CEM rendering, absent text editors for
  inadmissible input, disabled downloads for unavailable bytes, exact transfers,
  no fabricated preparation/preview, URL selection and retained drafts.
- Authored Studio source/scripts lint and `git diff --check` pass.
- `yarn copy:demo` passes using unchanged cached native/WASM outputs. Deployment
  remains 898 managed files, including all 333 Studio files and four WASMs below
  25 MiB. No generated screenshots or Vitest attachments are included in Git.
- An isolated deployed-browser acceptance opens the Supplier configuration
  bookmark cold, compares its read-only text and downloads to original catalog
  bytes/MIME, creates a separate saved copy and retains its unsaved stock draft
  while browsing the sample configuration. Offline reload preserves the journal;
  explicitly reopening the saved source restores the draft and leaves persisted
  bytes unchanged. The context is temporary; existing user projects/drafts are
  untouched.
- Assembled demo verification passes all managed hashes, paired gallery runtime
  bytes, native rendering/highlighting, saved/offline Studio builds and worker
  scope. All ten theme generators retain their CSS baseline; all 120 canonical
  playground endpoints and the live tree gallery pass.

- Final isolated complete Login recovery passes 2/2 worker/fallback cases,
  including three recoveries and twelve edits per runtime and two credential
  refusals per runtime. Entered passwords/one-time codes continue to block
  recovery until explicitly cleared. Automatic recovery remains disabled;
  4 MiB group budgets, 32,768-unit witnesses and all deadlines are unchanged.

## Scope

No native engine semantics or WASM ABI changed; the existing shared preview policy
was reused. The earlier native/WASM/catalog verification remains recorded in the
[owned binary command archive](todo-completed-owned-binary-commands-2026-10-28.md).
Multiline CDP insertion performance, version-specific module-map fixtures and
formatter artifact-reader trace expectations remain separate TODO items.
