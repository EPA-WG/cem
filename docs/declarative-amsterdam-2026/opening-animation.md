# Opening animation

The [main native deck](https://docs.google.com/presentation/d/1v5JEclFycuXuYk5Mnpwb0alxdKIi30J6XgvLBmawxI4/edit)
already contains the integrated opening, including the current brain artwork,
automatic start and three staged reveals. Verified 2026-11-01 against native
revision `0yRVSFpJ02gOuw` and actual Google Slides playback in an isolated,
anonymous Chromium 148.0.7778.96 context. No click or sign-in was needed.
The [readback receipt](native-opening-readback.json) records both checks.

No slide was copied or overwritten during this verification. The earlier claim
that integration was still missing came from stale documentation: the Slides API
readback does not expose native animation settings. A fresh PowerPoint export
revealed the existing integration. The main deck, its 30-slide order, speaker
notes, palette and original dark title remain unchanged.

## Current native timeline

Keep the existing animation rather than replacing it with the older prepared
slide. The native export encodes the following times from slide entry:

| Interval | Visible action |
| --- | --- |
| 0–3 seconds | Thought fades in, including the current brain illustration. |
| 3–10.1 seconds | Hold Thought. |
| 10.1–13.1 seconds | DNA zooms in; Thought remains visible. |
| 13.1–23.1 seconds | Hold Thought and DNA. |
| 23.1–25.1 seconds | CEM fades in; earlier groups remain visible. |
| 25.1 seconds onward | All three stages remain visible. |

The parent starts on `onBegin` of the owned main sequence. All seven scheduled
effects use `afterEffect`: Thought fade, two background-colored timing shapes,
DNA resize, two more timing shapes, then CEM fade. DNA width and height share the
same 10,100 ms start and 3,000 ms duration, each growing from zero to its authored
size. The four timing shapes last 3,000, 4,100, 5,000 and 5,000 ms. Their generic
accessibility labels mentioning five seconds are not authoritative timing data.

The current sequence completes at **25.1 seconds**, rather than the historical
26-second reference. It fits the unchanged 45-second opening slide slot. This is
a newer authored sequence, not a claim that the older three-fade reference has
been copied exactly.

## Playback verification

The real Google Slides `/present?slide=id.p1` route was opened in a fresh headless
browser context, without loading a saved profile or entering credentials. Eight
frames sampled the running slideshow after DOMContentLoaded, with no clicks:

| Sample | Checked result |
| --- | --- |
| 0 seconds | Initial hidden/starting state. |
| 3.5 and 7.5 seconds | Thought visible; DNA and CEM absent. |
| 12.5 seconds | DNA entering; Thought retained; CEM absent. |
| 17.5 and 22.5 seconds | Thought and DNA visible; CEM absent. |
| 26 and 30 seconds | All three stages visible, with earlier content retained. |

Detached image analysis counts foreground pixels in fixed stage regions, and
the initial playback frames were also visually inspected. This detects missing
or prematurely shown stages without changing the presentation DOM. Captures
were within 120 ms of their scheduled sample times; no page errors occurred.
Exact 25.1-second timing comes from the native timing XML. Browser samples
confirm the intervals and zero-click behavior; they do not measure every effect
boundary to millisecond precision. Desktop presenter controls, physical-display
acceptance and the full timed session rehearsal remain separate checks.

The native revision and all 30 slide IDs were read back after playback and
matched the initial read. No edit, import, copy, deletion or storage cleanup was
needed. Entered passwords/OTP still block recovery until explicitly cleared;
no Studio session, saved project, draft or credential storage was accessed.

## Repeat the checks

```sh
yarn nx run @epa-wg/cem-studio:test:opening-animation
yarn nx run @epa-wg/cem-studio:verify:opening-playback
```

The first target runs nine offline ownership/timing/resize refusal tests.
The second opens the public slideshow in an isolated browser for approximately
30 seconds and writes an ignored report and frames under
`dist/reports/cem-studio/opening-playback/`. It requires network access and the
installed Playwright Chromium. It does not automate authentication or reuse a
personal browser profile. Unexpected redirects or missing stages fail the check.

For a fresh structural timing audit, first save the complete native presentation
read and a PowerPoint export using the connected Drive tools, then run:

```sh
python3 -B packages/cem-studio/scripts/presentation_opening.py \
  dist/reports/cem-studio/animation-main-before.json \
  dist/reports/cem-studio/animation-main-export.pptx \
  dist/reports/cem-studio/opening-native-audit.json
```

The read-only audit resolves the first slide through the package relationships,
binds native group/shape owners to exported IDs, and validates actual effect
starts, durations, triggers and DNA width/height orchestration. Full native
responses, PowerPoint exports and screenshots remain ignored. The committed
receipt is dated evidence, not a live connection or future playback guarantee.
The combined checks pass 41 tests plus Studio lint. Shared components and the
credential privacy policy are unchanged; no app-local JavaScript was added.

## Historical prepared reference

[Prepared timed-reveal slide](https://docs.google.com/presentation/d/1R4HkuelX7K69jpqgJDIJBBEL8D0wTkUnDSVi15jqJws/edit)
is retained as a reference, not the current opening. Its Thought group contains
13 older objects, whereas the main slide contains the newer three-object brain
group. Blindly copying it would regress the current artwork and speaker notes.

That reference has three two-second fades at 0, 12 and 24 seconds after one
starting click. Its import/export timing settings were checked previously, but
that does not describe the main deck's current automatic sequence. Historical
artifacts remain under `dist/declarative-amsterdam-2026/slides/`; they must not
replace the authoritative native deck or its later edits.
