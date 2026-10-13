# Opening animation

Prepared native slide: [CEM opening — timed reveal (one-click start)](https://docs.google.com/presentation/d/1R4HkuelX7K69jpqgJDIJBBEL8D0wTkUnDSVi15jqJws/edit?usp=drivesdk).

The slide contains three editable groups: Thought, DNA, and CEM. Importing an
animated PowerPoint slide into Google Slides retained all three native fades
and their timing. The original 30-slide presentation remains unchanged.

## Timeline

The exported animation data encodes this sequence, measured from the first
click. Earlier sections remain visible as the next section appears.

| Time | Action |
| --- | --- |
| 0–2 seconds | Thought fades in. |
| 2–12 seconds | Hold. |
| 12–14 seconds | DNA fades in. |
| 14–24 seconds | Hold. |
| 24–26 seconds | CEM fades in. |

The three effects share one parallel parent. Their start delays are 0, 12,000,
and 24,000 milliseconds, and each fade lasts 2,000 milliseconds. These delays
are relative to the same parent, so the sequence completes at 26 seconds.
Microsoft documents this relationship in
[ChildTimeNodeList](https://learn.microsoft.com/en-us/dotnet/api/documentformat.openxml.presentation.childtimenodelist?view=openxml-3.0.1).

## Add it to the presentation

1. Open the prepared slide and copy its slide thumbnail.
2. Paste it into the main presentation beside the current first slide. Use the
   original theme so its CEM light colors remain intact.
3. Start the slideshow and click once. Check the fades and both ten-second
   pauses. Keep the current opening until this playback check passes, then
   replace it with the animated slide.
4. To try starting on slide entry, open **View → Motion**, change the first
   effect to **After previous**, and preview again. Confirm the pauses remain
   intact before using that setting.

Google documents **On click**, **After previous**, and **With previous** in its
[animation controls](https://support.google.com/docs/answer/1689475?hl=en).
Automatic start on slide entry is still unverified. Both tested imports
converted the authored first effect (`withEffect` and `afterEffect`) to
`clickEffect`, with an enclosing `delay="indefinite"` start condition.

## Verification and artifacts

The tested route was PowerPoint timing → native Google Slides import →
PowerPoint export → inspection of the exported timing XML. The native readback
also retained three editable groups. The Slides API cannot transfer these
native animations between presentations, so the final copy uses the Slides UI.

Artifacts are under `dist/declarative-amsterdam-2026/slides/`:

- `teaser-animation-auto-test.pptx` — authored import source.
- `raw-animation-auto.json` — native slide readback.
- `animation-auto-roundtrip.pptx` — export from the prepared native slide.
- `animation-auto-roundtrip.timing.xml` — retained timing data.
- `render-animation/presentation.pdf` — static layout preview.

The XML verifies retained settings; the PDF can verify layout only. Actual
slideshow playback, slide-entry behavior, and timing after copying into the
main presentation still require a presenter-mode check. Execution tasks remain
in [the presentation checklist](../todo.md).
