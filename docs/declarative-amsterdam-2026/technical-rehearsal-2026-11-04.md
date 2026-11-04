# Technical rehearsal — 2026-11-04

Current update: the affected public links on slides 18/30 now pass in fresh
HTTP/2 browser contexts at **27.480 / 25.873 seconds**, under the original
30-second gate. The [stable-connection readback](public-affected-links-stable-readback.json)
retains the exact selections, zero network/page errors and unchanged critical
release bytes. Earlier failed runs below remain historical evidence; this is
not a fresh single-batch pass of all 14 links.

The presenter reports the spoken/projector rehearsal completed and will provide
results. Actual timing, readability/controls and offline-fallback outcomes remain
pending in the acceptance record. Automated execution time does not measure
explanations, audience edits or questions.

## Delivery set

| Material | Current location / identity |
| --- | --- |
| Native deck | https://docs.google.com/presentation/d/1v5JEclFycuXuYk5Mnpwb0alxdKIi30J6XgvLBmawxI4/edit |
| Audience Site | https://cem-amsterdam-2026.trans-acf.workers.dev/guides/declarative-amsterdam-2026/ |
| Studio | https://cem-amsterdam-2026.trans-acf.workers.dev/studio/ |
| All slide resources | https://github.com/EPA-WG/cem/discussions/24#discussioncomment-18867103 |
| Accepted Site version | `928c385e-79cb-4ef9-844a-ce64967a6243` |
| Public acceptance | [Exact public receipt](site-public-928c385e-readback.json), 1,206 published files; explicit HTTP/2 browser profile |
| Speaker run sheet | [Presenter notes](presenter-notes.md), 30 physical slides / 26 timed / four optional; 3,600 planned seconds |
| Native notes readback | [Receipt](native-deck-notes-readback.json), revision `DuWnTfroxCPn2A` |
| Current static fallback PDF | `dist/reports/cem-studio/rehearsal-2026-11-04/render/presentation.pdf`, 906,565 bytes, SHA-256 `ca903c3e4076ee4285dd3be7640b167ef5f6602d670af5d1beee12d520b23140` |
| Rehearsal transfer | `dist/releases/cem-amsterdam-2026-rehearsal-public-recheck.tgz`; PDF, offline kit, run sheet and current evidence, with a companion `.sha256` file |
| Local transfer fallback | `dist/packages/cem-studio-login-kit.tgz`; exact bytes and fresh extracted-consumer result in the rehearsal receipt |
| Frozen commands/results | [Login 2.8.1](login-presenter/README.md) and [Supplier 1.1.1](supplier-presenter/README.md), manifests/checkpoint argv/diagnostics |

The native deck is authoritative. The earlier editable PPTX and November 2 PDF
precede the presenter's opening-slide changes. This fresh 30-page PDF includes
those changes, including the opening resource link/QR. The PDF is a static
fallback: it does not reproduce the native opening animation or live Studio.

## Executable checks

The starting checkout was `82daec226b42f30b403ca54b40ef4933de0c52b4`.
Node `v24.16.0`, Linux x64 and Playwright Chromium `148.0.7778.96` were used.
All targets ran through Yarn/Nx with the existing built dependencies, excluding
upstream tasks and skipping cache. This is a prepared-machine rehearsal, not a
clean toolchain installation or a benchmark.

```sh
NX_DAEMON=false yarn nx run-many \
  -t verify:login-presenter,verify:supplier-presenter,verify:presentation-resources \
  -p @epa-wg/cem-studio --excludeTaskDependencies --skip-nx-cache --parallel=1
NX_DAEMON=false yarn nx run @epa-wg/cem-studio:verify:participant-kit \
  --excludeTaskDependencies --skip-nx-cache
NX_DAEMON=false yarn nx run-many \
  -t test:presentation-resources,verify:presentation-resources,verify:presentation-story,lint \
  -p @epa-wg/cem-studio --excludeTaskDependencies --skip-nx-cache --parallel=1
```

Native replay and resource verification reported **55.70 seconds wall time**:
21 Login checkpoints, ten Supplier checkpoints, 12 cue bindings, 30 slide IDs
and the planned timing map. Output/diagnostic bytes, exact argv and exit codes
matched, including the intentional failure and saved repair. No frozen inputs
were rewritten. The native CLI here is the workspace build; the public release
CLI's known checkpoint mismatch remains outside the participant exercise.

Speaker notes were refreshed using the prior receipt's exact owned-block hashes
and a live revision guard. All 30 updates passed exact readback and an idempotent
zero-change plan. Original note tails, other note elements and every visible
slide property were preserved; temporary image download URLs/revision metadata
were excluded from structural comparison. An unreviewed block still refuses
replacement. Existing pinned source references remain available.

The built-guide verifier now serves the prepared same-origin publication and
checks the two actual source-only `cem-demo-element` viewers. It checks exact
trimmed/colorized source, no rendered demo children, the explicit import map and
shared module, semantic table, all local destinations and privacy statements.
Its former script-free `<pre>` expectation predates the requested snippet change.

The extracted consumer kit passed all 23 selected offline configurations, Login
and Supplier README routes, independent copies, original diagnostic selection,
Save/reload graph provenance, light-DOM interaction and unsaved draft recovery.
Its inventory was unchanged after use. The archive is 16,126,594 bytes, SHA-256
`2b8cec8f3cebe74c3a705313f9fc8b815056e614a1bd707a336c9b550ccfdf53`,
with 417 inventoried files. All 66 presentation guards, guide browser verification
and Studio lint pass.

The host clock advanced approximately eight hours during browser verification.
The kit command's reported total of 29,295.84 seconds is therefore discarded as
a rehearsal duration. The technical pass remains valid; no spoken timing is
inferred from it. Public-link attempt/retry results are retained separately in
[the rehearsal receipt](technical-rehearsal-2026-11-04-readback.json).

The live GitHub resource comment was read again: it contains the presentation
host and no `green-morning` origin. All 14 frozen deep links were matched against
that current comment before opening fresh browser contexts. The initial run
passed 11 and timed out at slides 18 (provenance), 19 (live view) and 30 (Supplier).
Its provenance duration crosses the clock discontinuity and is not a usable
startup measurement. Other completed cold starts reported 25.309–28.480 seconds.
Affected-link retries retain the original 30-second application-readiness gate;
any extra diagnostic wait is recorded separately and cannot establish a pass.
The isolated live-view retry passed at 28.977 seconds; provenance and Supplier
still timed out. The original 14-link run and the retry remain failed overall.
Three critical Studio startup files (index, main module and service worker)
remain byte-identical to the accepted prepared release. Cold public startup
remains a delivery limitation; prewarm and retain the verified offline kit.
The earlier provenance trace records `ERR_NETWORK_CHANGED` on the main module and
both stylesheets; the document itself returned 200, with no JavaScript page
errors. A separate extra 30-second diagnostic wait also failed and is not an
acceptance pass. The subsequent stable-connection recheck above closes the
affected-link gate; the earlier trace does not establish an application regression.

## Readability and live-versus-saved cues

A fresh export of the current native deck rendered all 30 pages. All pages were
reviewed in three contact sheets; code-dense slides 8, 12, 16 and the supplied
security artwork on 23 were also inspected at full render size. No clipping or
blocking layout defect was found. The structural checker reports zero errors;
its one warning is the intentional lifecycle arrow at the start of the second
line on slide 3, not a list item. Keep the original dark title/security artwork.
Physical projection contrast and room-distance readability remain unmeasured.

Slide 8 illustrates `Welcome back`; the live exercise uses `Bienvenue 🍒` to
exercise Unicode input. Say so before the edit. Restore `Sign in` before the
21-value comparison. Slide 19's saved image is a static preview; use the live
view configuration to demonstrate interaction. The saved live-view Parse result
is not a recording of that interaction. Parse source coordinates are not
clickable graph provenance, and formatting must not overwrite the input.

## Presenter run and fallback

1. Before the clock, open the current native deck, this PDF, the Site and kit.
   Extract a fresh kit; enter its folder and run `node start.mjs`. Open the
   printed loopback URL, retain its hostname/port/profile, and warm both ordinary
   Amsterdam samples and their service-worker cache. Keep baseline and editable
   copies independent. First-time cache population requires preparation.
2. Start the opening animation and the clock together. Keep the six intervals:
   00–07 motivation, 07–17 syntax/edit/schema, 17–35 diagnose/repair/inspect/
   compare, 35–47 query/transform/render, 47–54 platform, 54–60 questions.
   Installation stays outside the hour.
3. Exercise saved-versus-unsaved edits, select the original diagnostic span,
   repair/save/rerun, restore baseline before comparing, then build the graph
   and inspect template provenance. The live view reads its HTML input
   independently of the graph's CEM input.
4. If public loading fails, switch to the prewarmed local kit. If execution
   fails, preserve the draft and open the cue's saved output/diagnostic in a
   separate window. Identify it as a reference; do not describe it as a newly
   successful execution. If the browser fails, use the current static PDF and
   frozen HTML/checkpoints. PDF playback needs no network.
5. If late, omit Supplier live edits and optional slides 25, 27–29 first. Retain
   the required Login sequence and final six minutes. Supplier is a two-input,
   three-output companion overview unless a question warrants the saved variant.
6. On the presenter's machine, speak the complete run with the actual display,
   clicker and offline fallback. Record actual transition times, room-distance
   legibility and which optional material was skipped. Complete the delivery
   TODO only after this evidence exists.

Use disposable input only. Password/OTP values block recovery until explicitly
cleared; automatic whole-Studio recovery stays disabled. The 4 MiB aggregate
budget and five-second handoff deadline remain. Never reset participant projects,
drafts or site data to recover the presentation.

## Limits retained

The accepted public browser run used HTTP/2 (`--disable-quic`); default Chromium
had earlier transport/startup failures. Cold public startup is preparation time,
not a promised instant transition. The local kit is transferred separately and
has no public download promise. Other OS/assistive-technology environments,
clean Node installation, public registry CLI parity and actual projector controls
are not established by these checks. The unrelated Playground native plaintext
editing failure remains deferred in the main TODO.
