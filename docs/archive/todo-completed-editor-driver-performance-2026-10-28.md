# Multiline editor-driver performance — 2026-10-28

## Completed investigation

- [x] Reproduce the original multiline projection through bare native, isolated
  shared CEM and live Studio editors; record native events, editable nodes,
  publications, source equality, focus/selection and Chromium CPU profiles.
- [x] Isolate the Studio DOM without runtime listeners, then disable styles in a
  separate static snapshot to attribute browser/context cost independently.
- [x] Add the shared native bulk-paste regression and reproducible diagnostic;
  retain native history, selection, observable synthetic input, original
  deadlines and the credential privacy policy.

The dominant delay reproduces without CEM runtime listeners. Chromium's CDP
`fill` creates 1,987 editable nodes and emits 1,987 trusted input notifications
for the 24,585 UTF-16-unit source, while fresh native clipboard paste creates one
text node and one input. Shared CEM admission already publishes the final value
once on both paths. This investigation does not claim a production driver-speed
fix: the browser's fragmented edit remains slower, especially with Studio styles.
No runtime DOM replacement, local UI JavaScript, CSS containment workaround,
theme rule removal, or increased timeout was applied.

## Reproducible measurements

After `yarn copy:demo` and `yarn serve:demo`:

```bash
CEM_EDITOR_INPUT_PROFILE=1 node packages/cem-studio/scripts/verify-editor-input.mjs http://127.0.0.1:8081/
```

The diagnostic uses temporary browser contexts and cloned public example projects,
leaving existing user projects/drafts untouched. Each input operation retains the
30-second deadline. Reports/profiles are ignored build artifacts under
`dist/reports/cem-studio/editor-input/`; the report pins browser version, fixture
hash/size and all six cases. Node/event counts and latency are observations;
they are not assertions requiring Chromium's fragmentation defect to persist.
The actual input source is never assigned or rebuilt through DOM APIs.
Static snapshot/style changes are attribution setup only.

Profiled run on Chromium **148.0.7778.96**, with the original 24,586-byte,
994-line native AST JSON fixture:

| Case | Wall time | Native input events | Editable nodes | CEM publications |
| --- | ---: | ---: | ---: | ---: |
| Bare native fill | 2,489 ms | 1,987 | 1,987 | n/a |
| Isolated shared CEM fill | 4,813 ms | 1,987 | 1,987 | 1 |
| Static Studio fill, no runtime listeners | 21,234 ms | 1,987 | 1,987 | n/a |
| Same static DOM, styles disabled | 3,310 ms | 1,987 | 1,987 | n/a |
| Live Studio fill | 23,851 ms | 1,987 | 1,987 | 1 |
| Fresh live Studio paste | 38 ms | 1 | 1 | 1 |

Fixture SHA-256:
`774a1f3f9133d0f469e7028bf280d67be602d7a4b9b9585abab71bb5f14a5441`.
The styled static and live DOMs have 2,788/2,789 elements and 35 stylesheets.
The static CPU profile attributes 99.7% of sampled time to V8's `(program)`;
the live profile reports 94.1%, with about 916 ms in `readControl`. These profiles
and the listener-free reproduction establish browser editing/style work as the
main cost; they do not identify a particular CSS rule as faulty. Initial probes
also reproduced 26.9-second live fill and 24.4-second static fill. Replacing an
already fragmented buffer adds substantial cost, so the permanent diagnostic
uses a fresh editor for paste. Timings are hardware/browser dependent and include
optional profiling overhead.

## Verification

- All ten `cem-source-editor` stories pass. New `NativeBulkPaste` coverage pastes
  994 lines with Unicode and HTML-like literal text, requires one publication,
  retains the same focused native control and backward Unicode selection through
  status rendering, and verifies native undo/redo. Existing CDP bulk-publication,
  synthetic-input, composition, readonly and trailing-newline coverage passes.
- The six-case deployed diagnostic passes exact source, focus/control identity,
  shared publication, selection and native paste undo/redo assertions.
- Authored Studio source/scripts lint passes. Component story lint has zero
  errors and four existing non-null assertion warnings in unchanged code.
- Demo build/copy and complete verification pass: 898 managed files, 333 Studio
  files, four WASMs below 25 MiB, native gallery rendering/highlighting,
  saved/offline Studio builds, all ten unchanged theme generators and all 120
  playground endpoints plus the live tree gallery. Generated profiles, reports,
  screenshots and Vitest attachments remain excluded from Git.

- Final isolated complete Login recovery passes 2/2 worker/fallback cases,
  with three recoveries and twelve edits per runtime and two credential
  refusals per runtime. Entered passwords/one-time codes still block recovery
  until explicitly cleared; automatic recovery remains disabled. The 4 MiB
  group budget, 32,768-unit witness limit and all deadlines are unchanged.

No native/WASM semantics or production editor behavior changed. Future browser
updates can be measured with the same diagnostic; bulk-paste acceptance remains
separate from CDP fill. Version-specific module-map fixtures and formatter reader
trace expectations are the next unfinished native work.
