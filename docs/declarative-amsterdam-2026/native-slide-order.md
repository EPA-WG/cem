# Native slide order

Reviewed 2026-10-11 after Sasha's additions and reordering. Preserve this native
order; renumber labels without replacing user content with the initial PPTX.
The [slide story](slide-story.md) contains the corresponding speaking notes.

There are **30 physical slides: 26 timed and four optional references**.
References 25 and 27–29 remain in Sasha's chosen positions and add no scheduled
time. Skip them during the rehearsed run or use them for a relevant question.
The closing slide remains visible during 54:00–60:00.

| Slide | Stable native ID | Native title | Elapsed time |
| --- | --- | --- | --- |
| 01 | `p1` | CEM — DNA for structured data | 00:00–00:45 |
| 02 | `h36c7c826c4eee100_0_0` | CEM-ML — Declarative document lifecycle engine | 00:45–01:10 |
| 03 | `p2` | Change the source. See the result. | 01:10–03:00 |
| 04 | `p3` | Make capabilities reusable across systems | 03:00–04:30 |
| 05 | `p4` | Keep useful rules as systems change | 04:30–07:00 |
| 06 | `p5` | Keep your markup knowledge | 07:00–09:30 |
| 07 | `p6` | A small vocabulary for the whole journey | 09:30–12:00 |
| 08 | `p7` | Make one change | 12:00–15:00 |
| 09 | `p8` | Pick, extend, or create your own schemas | 15:00–17:00 |
| 10 | `p9` | Catch the mistake at its source | 17:00–20:00 |
| 11 | `p10` | Repair the cause. Run the check again. | 20:00–23:00 |
| 12 | `p11` | Inspect the meaning behind the syntax | 23:00–26:00 |
| 13 | `p12` | Make structure readable | 26:00–29:00 |
| 14 | `p13` | Change representation. Preserve agreed meaning. | 29:00–33:00 |
| 15 | `p14` | Multiple inputs. Multiple useful outputs. | 33:00–35:00 |
| 16 | `p15` | Ask a familiar question | 35:00–37:00 |
| 17 | `p16` | Declare the transformation | 37:00–39:00 |
| 18 | `p17` | Follow the result back to its source | 39:00–41:00 |
| 19 | `p18` | Render into the document you can inspect | 41:00–44:00 |
| 20 | `p19` | Embedding applications | 44:00–47:00 |
| 21 | `p20` | Find your place in the platform | 47:00–49:00 |
| 22 | `p21` | Put processing where the work happens | 49:00–50:00 |
| 23 | `h209a9db50e1b8a77_0_0` | Browser vs DCE security | 50:00–50:30 |
| 24 | `p22` | Insulate each vendor’s contribution | 50:30–52:30 |
| 25 | `p27` | Six layers: foundations and boundaries | Optional reference |
| 26 | `p23` | Make performance a lifecycle property | 52:30–54:00 |
| 27 | `p28` | Follow the source and the evidence | Optional reference |
| 28 | `p25` | 32 schema packages | Optional reference |
| 29 | `p26` | Reuse expertise within explicit profiles | Optional reference |
| 30 | `p24` | Bring one useful integration | 54:00–60:00 |

The new slide 23 preserves Sasha's image and original speaker note. Its badge
and 50:00–50:30 footer are editable native text. Slide 22 now takes 49:00–50:00;
all later scheduled boundaries remain unchanged. Slide 20 retains the revised
embedding copy and combined quotation panel.

Readback: `dist/declarative-amsterdam-2026/slides/raw-after-user-sync.json`.
The guarded [synchronization script](sync-slide-order.cjs) records this patch;
its explicit order and revision checks require review before a later rerun.
