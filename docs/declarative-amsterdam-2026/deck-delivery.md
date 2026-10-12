# Google Slides delivery

Created 2026-10-11 from the presentation, pitch, manifesto and evidence docs.

[Open the Google Slides presentation](https://docs.google.com/presentation/d/1v5JEclFycuXuYk5Mnpwb0alxdKIi30J6XgvLBmawxI4/edit)

The native deck is in **My Drive / ChatGPT**: **25 main slides and four reference
slides**, all with speaker notes. The main run follows the six public intervals
and reserves 54–60 minutes for questions. Narrative text and diagrams are
editable. Source images retain the approved formatting and colors. Discussion
#24 is the resource link and QR destination; Studio is marked in development.

Sasha's cloned opening now introduces [DNA for structured data](opening-teaser.md).
The teaser is light; the original title slide remains dark at position 2. The
deck-wide palette change and dark terminal samples are recorded in the checklist.
The native deck includes these edits and is authoritative over the initial PPTX.

## Artifacts

| Artifact | Location |
| --- | --- |
| Speaker script and timing | [slide-story.md](slide-story.md) |
| Initial PowerPoint, before the native opening edit | [cem-declarative-amsterdam-2026.pptx](../../dist/declarative-amsterdam-2026/slides/cem-declarative-amsterdam-2026.pptx) |
| Current native Google Slides PDF | [presentation.pdf](../../dist/declarative-amsterdam-2026/slides/render-teaser-final/presentation.pdf) |
| Source and command evidence | [evidence.md](../../dist/declarative-amsterdam-2026/slide-assets/evidence.md), [evidence.json](../../dist/declarative-amsterdam-2026/slide-assets/evidence.json) |
| Current slide PNGs | `dist/declarative-amsterdam-2026/slides/render-teaser-final/slide-01.png` through `slide-29.png` |
| Current native readback | `dist/declarative-amsterdam-2026/slides/raw-after-teaser-final.json` |
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
with presentation CSS; slide 18 labels it accordingly.

Initial native readback confirmed 28 slides and 28 speaker notes; its PDF had 28 pages.
Every slide was visually inspected. Contrast and appendix spacing were corrected
and the changed pages inspected again. The structural check has no geometry
errors; its single warning is an intentional lifecycle arrow at the beginning
of a wrapped line on slide 2.

The opening update has 29 slides and speaker-note pages. The new teaser is
native editable text and vectors, using canonical CEM palette values. Numbering
and the first three timestamps were updated; later agenda boundaries remain
unchanged. The full session still totals 60 minutes.
The final native PDF has 29 pages. All pages were visually checked; numbering,
opening timing and dark-slide contrast passed review.

Working-tree base: `33e18c9975784c5ba536217873ae8ac3a351cb65`; this task's changes
are uncommitted. Node 24.16.0; Chromium 148.0.7778.96.

| File | SHA-256 |
| --- | --- |
| Native CLI binary | `24454821fb196e9f0e01c5bff2110fe1def2157fc893bd7e0b5ef8863da26920` |
| Canonical login | `68f0487d911ed0a072366fbb19f9b52c3afce80cb0b404b3d6c345d07e71dbea` |
| Initial 28-slide PDF, 611700 bytes | `edfeaac5c7e2ac737aa14fd13e6f01c309fb0a04d641d37b3e822c6011c072da` |
| Current 29-slide PDF, 616632 bytes | `5b5d926163897714182c9689ae6ffa2e9ff461ec4e2bf7b8dc1c70e164ce0f4f` |

## Preparation still open

The public HTML-alternate-input checkpoint, login expression/template execution
into `<cem-element>`, edited interactive result and complete generated-result
transformation trace still need verification. Slides 17–19 explain those
contracts without fabricating an execution transcript. The participant handout,
clean-install instructions, website, organizer requirements and timed rehearsal
remain in [docs/todo.md](../todo.md).

The supplier pipeline is labelled a companion scenario. Six-layer insulation
is an architectural target with partial controls. Performance slides describe
mechanisms without invented benchmarks or measured savings. The catalog lists
32 **packages**, including internal contracts and module-map revisions.
