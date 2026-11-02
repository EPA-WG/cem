# Google Slides delivery

Created 2026-10-11 from the presentation, pitch, manifesto and evidence docs.

[Open the Google Slides presentation](https://docs.google.com/presentation/d/1v5JEclFycuXuYk5Mnpwb0alxdKIi30J6XgvLBmawxI4/edit)

The native deck is in **My Drive / ChatGPT**: **30 physical slides, including
26 timed slides and four optional references**, all with speaker notes.
The [native slide order](native-slide-order.md) preserves Sasha's additions and
reordering. References remain at positions 25 and 27–29; closing/Q&A is slide 30.
The main run follows the six public intervals
and reserves 54–60 minutes for questions. CEM narrative text and diagrams are
editable; the supplied DCE diagram remains an image. Source images retain the
approved formatting and colors. Discussion
#24 is the resource link and QR destination; Studio is marked in development.

Sasha's cloned opening now introduces [DNA for structured data](opening-teaser.md).
The teaser is light; the original title slide remains dark at position 2. The
deck-wide palette and dark diagnostic panel are now verified in the
[palette readback](native-deck-palette-readback.json).
The native deck includes these edits and is authoritative over the initial PPTX.
The later update preserves the added “Browser vs DCE security” image and the
revised embedding quotations. The DCE promise takes 50:00–50:30, within the
existing platform block; the deployment overview now takes 49:00–50:00.

## Artifacts

| Artifact | Location |
| --- | --- |
| Speaker script and timing | [slide-story.md](slide-story.md) |
| Current physical order and stable slide IDs | [native-slide-order.md](native-slide-order.md) |
| Initial PowerPoint, before the native opening edit | [cem-declarative-amsterdam-2026.pptx](../../dist/declarative-amsterdam-2026/slides/cem-declarative-amsterdam-2026.pptx) |
| Current native Google Slides PDF | [presentation.pdf](../../dist/reports/cem-studio/palette-final-render/presentation.pdf) |
| Source and command evidence | [evidence.md](../../dist/declarative-amsterdam-2026/slide-assets/evidence.md), [evidence.json](../../dist/declarative-amsterdam-2026/slide-assets/evidence.json) |
| Current slide PNGs | `dist/reports/cem-studio/palette-final-render/slide-01.png` through `slide-30.png` |
| Current native readback | `dist/reports/cem-studio/palette-final.json` |
| Current numbering/timing patch and checks | `dist/declarative-amsterdam-2026/slides/user-sync-requests.json`, `user-sync-issues.json` |
| Initial-deck QA and repairs | `dist/declarative-amsterdam-2026/slides/output-issues-final.json`, `native-repairs.json` |

Generated `dist/` files are ignored. These commands reproduce the **initial**
PowerPoint and its code assets, not subsequent native edits:

```sh
node docs/declarative-amsterdam-2026/generate-code-preview.mjs
node docs/declarative-amsterdam-2026/generate-slide-assets.mjs
python3 docs/declarative-amsterdam-2026/generate-resource-qr.py
node docs/declarative-amsterdam-2026/generate-slides.cjs
```

Authoring requires the built native CLI, WASM/theme/demo packages, Playwright
Chromium, PptxGenJS 4.0.1 and ReportLab's pure Python QR encoder. Set
`CEM_SLIDE_NODE_MODULES` and `CEM_QR_ENCODER` for different dependency locations.
Import the PowerPoint as native Slides. Google may override hyperlink colors;
set the closing slide's link to white against its dark background. These are
authoring steps, not participant installation instructions.

Preserve the live native deck when editing. The opening update is reproduced
by [update-opening-slide.cjs](update-opening-slide.cjs) from the saved
`raw-before-teaser.json` snapshot. Its revision guard prevents applying it to
a changed deck without review; never reimport the initial PPTX over Sasha's work.
The later [order synchronization](sync-slide-order.cjs) uses
`raw-before-user-sync.json`, preserves user content, and guards the reviewed
30-slide order and native revision. It changes numbering and allocates the
new diagram's time without reordering or replacing slides.

## Verified content

The asset run executed 17 native commands and generated 14 source/view images.
CEM-ML uses native tabular formatting and the existing CEM WASM HTML colorer;
XML markup uses its HTML path. CEM-QL uses native tabular formatting and native
HTML color output. XPath stays plain because its colorer is unsupported.
Browser checks confirm exact source text, no overflow and no page errors.

Removing `@method=post` violates the teaching schema: exit 1, one error,
`demo.login.method_required`. Restoring it returns exit 0 with zero errors.
Both runs retain three baseline unresolved-reference warnings. The diagnostic
header reports 1:1; its retained source frame locates the form at line 8.

The verified route exports CEM to XML, explicitly restores the source namespace
declarations, then imports XML. XPath and CEM-QL select the same two input nodes
and source spans. The inspected node has source `login.xml`, byte offset 252
and length 78. The sign-in screenshot uses the existing static HTML counterpart
with presentation CSS; current slide 19 labels it accordingly.

Initial native readback confirmed 28 slides and 28 speaker notes; its PDF had 28 pages.
Every slide was visually inspected. Contrast and appendix spacing were corrected
and the changed pages inspected again. The structural check has no geometry
errors; its single warning is an intentional lifecycle arrow at the beginning
of a wrapped line on slide 2.

The opening update produced 29 slides and speaker-note pages. The teaser is
native editable text and vectors, using canonical CEM palette values. Numbering
and the first three timestamps were updated; later agenda boundaries remain
unchanged. The full session still totals 60 minutes.
That native PDF had 29 pages. All pages were visually checked; numbering,
opening timing and dark-slide contrast passed review.

The current synchronization readback and PDF contain 30 slides. The new image,
user-edited text and chosen order are preserved. Numbering runs 01–30; 26 timed
slides span 00:00–60:00 continuously. The four reference slides retain their
REFERENCE footers. The issue check reports no errors; its single warning is
the intentional lifecycle arrow on current slide 3.
All 30 rendered pages passed visual review for numbering, legibility and
clipping. A full object comparison confirms that the patch preserved Sasha's
image, embedding text, styling and slide order.

The initial deck and opening work were committed and pushed as `ca8f6ef4` on
`origin/develop` before this synchronization. Generation used Node 24.16.0 and
Chromium 148.0.7778.96; the native PDF was rendered with `pdftoppm` at 120 dpi.

| File | SHA-256 |
| --- | --- |
| Native CLI binary | `24454821fb196e9f0e01c5bff2110fe1def2157fc893bd7e0b5ef8863da26920` |
| Canonical login | `68f0487d911ed0a072366fbb19f9b52c3afce80cb0b404b3d6c345d07e71dbea` |
| Initial 28-slide PDF, 611700 bytes | `edfeaac5c7e2ac737aa14fd13e6f01c309fb0a04d641d37b3e822c6011c072da` |
| Opening-update 29-slide PDF, 616632 bytes | `5b5d926163897714182c9689ae6ffa2e9ff461ec4e2bf7b8dc1c70e164ce0f4f` |
| October 11 30-slide PDF, 846673 bytes | `c7dafc51e4050a9922449e953fe8dbb75834e7cf459c44955845303feca5e913` |

## Preparation still open

The original slide export on October 11 covered a smaller CLI/static route.
The [frozen Login](login-presenter/README.md), [Supplier](supplier-presenter/README.md)
and [local participant kit](participant-kit-verification.md) now supply verified
source/command/output evidence, including the live view and graph provenance.
The [participant resource index](participant-resources.md),
[reviewable discussion block](discussion-24-resources.md) and
[current presenter notes](presenter-notes.md) are ready in the repository.
The new notes preserve the recorded 30 slide IDs and 60-minute schedule.

Native-deck operational notes were synchronized and read back on 2026-11-01:
30 revision-guarded insertions preserved original notes and visible slide content.
The [readback receipt](native-deck-notes-readback.json) binds stable IDs and hashes
to the reviewed cues and links. Current blocks supersede older execution-status
claims while preserving the original narrative beneath them. Static images were
not updated. Public release/host verification,
organizer requirements and measured rehearsal remain in [docs/todo.md](../todo.md).

The supplier pipeline is labelled a companion scenario. Six-layer insulation
is an architectural target with partial controls. Performance slides describe
mechanisms without invented benchmarks or measured savings. The catalog lists
32 **packages**, including internal contracts and module-map revisions.

## Canonical palette — 2026-11-01

The native deck now uses CEM comfort canvas/text, creativity emphasis, neutral
cards, trust containers/links and calm contours from the canonical
[cem-colors source](../../packages/cem-theme/src/lib/tokens/cem-colors.md).
The original title at physical slide 2 is preserved in dark mode; slide 30 is
light. The diagnostic excerpt on slide 10 has a CEM dark comfort canvas, warm
error emphasis and light message text. This remains a labelled diagnostic
excerpt, not a fabricated terminal transcript. Source screenshots, the QR and
Sasha's security image are unchanged; raster artwork is not recolored.

Revision-guarded color updates preserve slide IDs/order, wording, styled runs,
fonts, links, geometry and every speaker note. Exact readback is normalized only
for temporary image download URLs, revision IDs, 8-bit color precision/omitted
zero channels, equivalent run splits/nonpainting newline colors and subpixel
numeric precision. The preserved title is excluded from color updates.

All 30 output pages were rendered at 90 dpi and visually inspected. One
consolidated repair changed both resource-link colors to the CEM trust endpoint,
then the final 30-page export was inspected again. The structural issue checker
has zero errors and the existing intentional lifecycle-arrow warning. No wording,
image or layout repair was necessary. The final PDF is 889089 bytes, SHA-256
`5bacacc1cd6f936a9d498abef8fb2eb7b6bef26975bb98173cee2cf63f31e703`.
The [committed receipt](native-deck-palette-readback.json) records the native
revision, canonical token bindings and 21 used palette contrast pairs.

Every checked narrative text pairing exceeds 4.5:1. Calm text is used on canvas,
neutral or trust surfaces; it is not used as small text on creativity surfaces.
Nine approved local source-image HTML documents retain their semantic syntax
colors, with minimum measured text contrast 5.837:1. This checks those local
source documents, not OCR or a new native colorizer run. Before/after native
images are structurally preserved. A repeat update plan contains zero requests.

Thirty-two resource/notes/palette guard tests and Studio lint pass. The guarded
planner/readback tool is `packages/cem-studio/scripts/presentation-palette.mjs`;
it accepts saved full native reads, writes an ignored review plan, and never
mutates Drive itself. Submit only the reviewed structured requests with the
recorded revision guard; keep full responses and rendered files in ignored
`dist/reports/cem-studio/`. Shared components, project storage and the credential
privacy policy are unchanged. The subsequent [opening playback check](opening-animation.md) verifies the
existing automatic native sequence; measured rehearsal remains open. This PDF
review alone is not a Slideshow timing test.

## Integrated opening playback — 2026-11-01

Fresh native timing export and anonymous Google Slides playback confirm the
opening is already integrated. Preserve the newer brain artwork and current
25.1-second automatic Thought → DNA → CEM sequence; do not copy the older
three-fade reference over it. The [timeline and repeated-check instructions](opening-animation.md)
and [readback receipt](native-opening-readback.json) record zero-click start,
eight stage samples, retained content at 30 seconds and unchanged native revision.
The 45-second opening slot and 60-minute agenda remain unchanged. Nine timing/
ownership tests, 32 resource/notes/palette tests and Studio lint pass. Full timed
rehearsal and physical presenter controls remain separate.

## Manifesto/platform story mapping — 2026-11-01

The [completed story map](story-map.md) covers all 14 manifesto principles and
seven pitch sections against existing native slide IDs, current presenter cues
and six website story sections. No slides or notes need rewriting: the current
narrative already carries those messages. The authored Site route is
`/guides/declarative-amsterdam-2026/`; it uses the native Markdown/HTML/DOM/CEMT
pipeline and existing shared page layout, without app UI JavaScript. The final
six-minute question period, optional-reference rules and credential policy remain.
Native build and focused browser acceptance pass, alongside 40 guard tests and
Studio lint. The whole-site CLI-guide relative-link policy failure is recorded in
the map and TODO; public hosting and full-session rehearsal are still open.
