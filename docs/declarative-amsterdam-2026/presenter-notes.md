# Current presenter notes

Operational cues for the frozen Login/Supplier route, prepared 2026-11-01.
Use these with the [slide story](slide-story.md) and the
[recorded native slide order](native-slide-order.md). Stable IDs below identify
the live deck read back on 2026-11-01. Operational blocks were inserted into all
30 speaker-note fields by stable ID, above the preserved original notes. Palette
and animation remain delivery checks. A rehearsal has not yet measured these times.

## Before starting the clock

Use the [participant resource index](participant-resources.md), the supplied kit
and its [dated verification](participant-kit-verification.md). Start the local
server and warm the cache before speaking. Keep one editable Login copy, a
separate baseline copy, the [Login references](login-presenter/README.md), and the
[Supplier references](supplier-presenter/README.md) ready. Never reset existing
projects, drafts or site data. Keep the kit's hostname, port and browser profile.

Prepare an editor and preview side by side. Baseline heading is `Sign in`; the
template instruction is `Use your email and password.`; the local action is
`Sign in locally`. After each experiment, restore and save the original value
before comparing a baseline result. Keep unsaved changes visibly distinct from
persisted execution inputs. The project, resource and configuration IDs in
[presenter-resources.json](presenter-resources.json) are checked against the
ordinary catalog and frozen outputs.

Use no real credentials. Entered passwords/OTP block recovery until explicitly
cleared. Do not export credentials or include them in sources/screenshots.
Automatic whole-Studio recovery stays disabled; retain the shared 4 MiB aggregate
budget and five-second deadline. An empty password control needs no demonstration
secret. If a participant cannot clear a disposable value, keep recovery blocked
and continue with the saved reference output.

## Timed cues

Keep the six public intervals and final six-minute question period. Optional
references add no scheduled time. The table preserves the recorded physical
order, including Sasha's original title/security slides and interleaved references.
The resource IDs named as cues refer to the checked JSON run sheet below.

| Slide | Stable ID | Elapsed | Cue |
| --- | --- | --- | --- |
| 01 | `p1` | 00:00–00:45 | Explain the DNA metaphor. Show source and a useful result; no platform tour yet. |
| 02 | `h36c7c826c4eee100_0_0` | 00:45–01:10 | Introduce yourself and the session. This original title slide remains the planned dark contrast. |
| 03 | `p2` | 01:10–03:00 | Show baseline Login and name the sequence: edit, validate, inspect, transform, render. Open `readme`; cue `read`. |
| 04 | `p3` | 03:00–04:30 | Ask who maintains one rule in multiple delivery paths. Explain the integration cost in ordinary words. |
| 05 | `p4` | 04:30–07:00 | Trace sources, schemas, modules, packaging and operation. Identify the contracts kept through a change. |
| 06 | `p5` | 07:00–09:30 | Compare familiar HTML/XML names with CEM-ML notation. Do not claim universal interchangeability. |
| 07 | `p6` | 09:30–12:00 | Explain only namespace, node, attribute and content separator needed for this source. |
| 08 | `p7` | 12:00–15:00 | Cue `edit`: change the first heading to `Bienvenue 🍒`. Predict the effect; show Run before Save, then Save and validate. |
| 09 | `p8` | 15:00–17:00 | Explain pick/extend/create schemas. Distinguish a parser accepting syntax from a domain rule accepting an instance. |
| 10 | `p9` | 17:00–20:00 | Cue `fail`: validate the prepared syntax-valid form missing its method. Select `demo.login.method_required`; show the source span. |
| 11 | `p10` | 20:00–23:00 | Cue `repair`: add `@method=post` beside `@cem:form="sign-in"`, save and rerun. Zero errors; three reference warnings remain. |
| 12 | `p11` | 23:00–26:00 | Cue `inspect`: show native CEM-ML structure and original coordinates. Printed coordinates alone are not Provenance links. |
| 13 | `p12` | 26:00–29:00 | Cue `format`: pretty/tabular output and inert source-color export. Formatting does not overwrite the source; sandbox styles remain blocked. |
| 14 | `p13` | 29:00–33:00 | Cue `compare`: restore baseline heading; compare all 21 named values from HTML and CEM. Name wrappers, namespace mirrors and lexical differences outside the contract. |
| 15 | `p14` | 33:00–35:00 | Show Supplier's two-input/three-output diagram or saved baseline artifacts. Keep this a companion overview; defer live stock edits until required Login is complete. |
| 16 | `p15` | 35:00–37:00 | Cue `query`: compare prepared XPath and CEM-QL results. Cue `expression` returns `title=Sign in`, `required=2`. Show only the relevant expression. |
| 17 | `p16` | 37:00–39:00 | Cue `transform`: run the Login graph. Change the saved template instruction and rebuild; identify data and template as independent inputs. |
| 18 | `p17` | 39:00–41:00 | Follow one generated HTML range to `templates/view.cemt`. Computed output identifies authored rules; it does not promise universal per-character data lineage. |
| 19 | `p18` | 41:00–44:00 | Cue `live`: edit the local action label, save and rerun the script-free light-DOM view. Its data input is HTML, independently of the CEM graph. |
| 20 | `p19` | 44:00–47:00 | Inspect ordinary DOM and explain declarative data/events/templates plus the shared runtime. Light DOM is not an arbitrary-script security boundary. |
| 21 | `p20` | 47:00–49:00 | Map engine, schema packages, CEM-QL, hosts, theme/components, Studio and Site to the demonstrated route. Local kit is verified; public hosting is unverified. |
| 22 | `p21` | 49:00–50:00 | Explain native/WASM and terminal/browser/worker ownership. Public CLI installation and other OS claims require separate evidence. |
| 23 | `h209a9db50e1b8a77_0_0` | 50:00–50:30 | Preserve Sasha's DCE security image. State the scoped promises and their current limits. |
| 24 | `p22` | 50:30–52:30 | Explain vendor/scope boundaries as the architectural direction with the recorded enforcement profile. |
| 25 | `p27` | Optional reference | Skip in the timed run. Use for a relevant scope question only. |
| 26 | `p23` | 52:30–54:00 | Describe cache/reuse/recovery mechanisms without invented benchmarks. Keep credential refusal and automatic-recovery limits explicit. |
| 27 | `p28` | Optional reference | Reproducibility evidence if asked; frozen input/output hashes identify exact bytes. |
| 28 | `p25` | Optional reference | Package catalog breadth is not a claim of complete standards conformance. |
| 29 | `p26` | Optional reference | Compatibility profiles and extension boundaries if asked. |
| 30 | `p24` | 54:00–60:00 | Keep the resource hub visible for questions and adoption. Relevant Supplier questions may use cue `supplier`; do not schedule an extra tour. |

## Exact actions and fallback choices

The [Login walkthrough](login-walkthrough.md) supplies complete steps. Cue
`read` opens the original README; clone before editing. For `edit`, use
`login-cem` / `validate`. The prepared failure and repair both use `login-invalid`
/ `validation-repair`, with its project-owned schema package. Do not imply that
an unsaved repair changed the persisted validation result.

For comparison, use `semantics-html`, `semantics-cem` and their `-cemql` variants;
the same 21 named string values must match the saved contract. The expression
configurations use the saved standalone source, not an invented module wrapper.
The graph is `login-view-graph` / `render-graph`; its artifact is
`generated/login.html`. Follow actual owned graph provenance. Parse/Inspect's
printed source IDs and offsets cannot be promoted into links by the example.

For `live`, select `login-live-view` / `live-view`. Restore the original action
label before starting; change it to `Continue locally`, save and rerun. This view
reads `login-html`. Editing `login-cem` for the graph does not silently change it.
Native Parse/Validate keep 15 advisory attribute warnings for this template; the
recorded CLI/worker undeclared-variable difference remains a separate limitation.

For `supplier`, use an independent Supplier copy. `stock-source` shows saved CSV;
`availability` builds HTML/JSON/YAML. If a relevant question warrants live work
after Login, change `SKU-A,7` to `SKU-A,0`, save and build; then use `many` to
demonstrate domain-export refusal, restore `7`, save and rebuild. The zero and
restored outputs are frozen. No new artifacts belong to the failed run; prior
successful files on disk remain prior outputs. XML source/query/repair snapshots
are prepared references, not another scheduled presentation sequence.

At any failure, say which operation did not complete and open its saved output
or diagnostic from the run sheet. Explain the expected result without presenting
it as a successful live execution. A saved baseline is not proof that the most
recent edit completed. Preserve the current draft; cancel a navigation
guard or use the separate reference window. Do not clear projects, storage or
credentials on a participant's behalf.

If time slips, drop the live Supplier variant, optional references and catalog
detail first. Preserve Login edit, schema failure/repair, inspection, HTML/CEM
comparison, query/template/graph, visible light-DOM result, required platform map
and the final six-minute question period. Keep installation outside the clock.

## Closing and publication

Say: “Resources and updates are at discussion #24. The handout identifies the
tested local kit, exact inputs and results. Choose one bounded integration and
tell us which contract you want to preserve.” Keep the stable hub/QR visible.
The [resource block](discussion-24-resources.md) is prepared for review, not posted.
No public kit download, public Studio URL or untested installer is promised.

## Verification record

On 2026-11-01, `yarn nx run @epa-wg/cem-studio:verify:presentation-resources`
passed all 11 guard tests and checked 12 cues against current/frozen project
identities, native fallback bytes, 30 recorded slide IDs and 3600 planned seconds.
Studio lint passed. The source-link audit read the hub and three pinned GitHub
pages; routine local verification checks the corresponding committed blob hashes
and local links, without claiming a fresh external HTTP check. The generated
report is `dist/reports/cem-studio/presentation-resources.json`. Native-deck
readback is recorded in [the notes receipt](native-deck-notes-readback.json);
measured rehearsal remains a separate check.

## Native-deck notes readback — 2026-11-01

Thirty notes-only insertions used the current revision guard. Exact text readback
confirmed every new operational block, the three pinned resource URLs and all
original note text. The original narrative remains below the current block;
current execution cues explicitly supersede older preparation/status claims.
All slide IDs, order, visible text, geometry, styles and image source properties
were unchanged. Only revision metadata and temporary image download URLs were
excluded from structural comparison. No slide visuals were edited or rerendered.
An idempotence check planned zero further insertions. The combined resource and
notes suite passes 21 tests, including post-sync cue-drift refusal; Studio lint
passes. Shared components and credential policy are unchanged.

The receipt is dated evidence, not a claim that later deck edits are unchanged.
Full before/after connector responses stay in ignored local reports; the committed
receipt contains stable IDs, revision IDs and block hashes, without copying user
notes. The local verifier checks receipt bindings against current cue-table and
resource identities. Any intentional cue change requires review and a fresh native
notes readback. No discussion message was posted, public kit availability was not
changed, and no browser projects, drafts or credential fields were accessed.
