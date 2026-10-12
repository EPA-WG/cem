# Declarative Amsterdam 2026 — presentation handoff

Started 2026-10-10 before clearing chat context; updated 2026-10-11 after agenda
alignment. The user requested a temporary switch from implementation to preparing
the CEM presentation for Declarative Amsterdam 2026. The current priority order
is presentation, CEM Studio, then the preserved implementation queue.

## Resume after clearing context

Read the repository's [CLAUDE.md](../../CLAUDE.md), then the current presentation
section in [docs/todo.md](../todo.md#current-work-declarative-amsterdam-2026-cem-presentation).
That file remains the sole execution checklist. Work on its first unfinished
presentation action; do not start datepicker migration while this set is active.

The user requested an adoption-focused platform pitch on October 10 and
alignment with the published agenda on October 11. Read the
[presentation minutes](../declarative-amsterdam-2026/presentation.md#presentation-minutes)
for the current run: preserve the public login walkthrough and all six proposal
blocks, using the pitch to explain their value. Read the
[presentation plan](../declarative-amsterdam-2026/presentation-plan.md)
for ideas, preliminary design, and adoption decisions, the
[platform pitch](../declarative-amsterdam-2026/cem-platform-pitch.md) for the
audience message, the [manifesto](../declarative-amsterdam-2026/cem-manifesto.md)
for principles, and the [evidence record](../declarative-amsterdam-2026/platform-evidence.md)
for capability support. The [native Slides deck and PDF](../declarative-amsterdam-2026/deck-delivery.md)
were created on October 11 and now contain 30 physical slides: 26 timed slides
and four optional references interleaved in Sasha's order, all with speaker
notes. The [opening DNA metaphor](../declarative-amsterdam-2026/opening-teaser.md)
starts the session; “Browser vs DCE security” is slide 23 and the closing resource
slide is 30. References 25 and 27–29 add no scheduled time. The session retains
the six public intervals and 54–60-minute question period.
The native deck includes Sasha's edits; do not replace it with the initial PPTX.
The website, complete runnable demonstration, public release checks,
benchmarks and conference acceptance checks remain open.

Studio is not ready yet. The slides use
[discussion #24](https://github.com/EPA-WG/cem/discussions/24) as the stable
resource destination for slide links, QR codes, handouts, and the later website.
Exercise, release, and Studio links will be added there as verified and
available. Keep Studio in the platform overview; the session's participant
route does not depend on it. Verify the login demonstration and prepare its
presenter-led fallback independently.

Keep durable decisions, artifact locations, tested commands/versions and the next
step in these repository documents as presentation work proceeds. The chat and
temporary files are not the only record needed to continue.

## Starting material

- [Presentation plan](../declarative-amsterdam-2026/presentation-plan.md): internal
  audience strategy, narrative, demonstration design, adoption criteria and
  preparation requirements. The login document is the required demonstration;
  a supplier pipeline across formats and hosts is a companion for XML/XSLT,
  data, integration, and UI developers.
- [CEM manifesto](../declarative-amsterdam-2026/cem-manifesto.md) and
  [platform pitch](../declarative-amsterdam-2026/cem-platform-pitch.md): principles
  and audience-facing message. Public wording stays in the pitch; presentation
  instructions, proof selection and pilot measurements stay in the plan.
- [Pitch evidence](../declarative-amsterdam-2026/platform-evidence.md): audit of
  DCE slide 6, the six resource layers, all 32 schema packages, runtime boundaries
  and proposed demonstrations. Complete vendor containment, progressive end-to-end
  consumption and bundler parity must not be presented as finished capabilities.
- [Session proposal](../declarative.amsterdam.2026.md): **CEM-ML: Evolutionary
  Architecture for the Declarative Web Stack**. It proposes a 60-minute
  mini-tutorial for newcomers, following one small document through validation,
  inspection, conversion, transformation and rendering. It includes a timed
  journey and a presenter-led fallback for participant setup problems.
- [Conference summary draft](../summary.md): introductory wording, platform
  framing and a proposed pipeline diagram. Some deployment/runtime descriptions
  are historical; reconcile them with current implementation before reuse.
- [Schema presentation](cem-ml-schema-presentation.md) and its adjacent `assets/`:
  secondary background and illustrations, explicitly a 2026-05-21 snapshot.
  Its own notice makes current acceptance criteria and designs authoritative.
- [CEM-ML syntax](../cem-ml-syntax.md), [CLI contract](../cem-ml-cli-contract.md),
  [deployment contract](../cem-ml-deployment-contract.md) and
  [canonical examples](../../examples/cem-ml/README.md): starting references for
  verifying the tutorial and its commands.
- [Runtime README](../../packages/cem-elements/README.md),
  [components README](../../packages/cem-components/README.md) and
  [roadmap](../../roadmap.md): current implementation and platform context.

The [working presentation document](../declarative-amsterdam-2026/presentation.md)
records the 2026-10-11 program/discussion review, topic coverage, and current
minute allocations. The [published program](https://declarative.amsterdam/program)
lists Sasha Firsov's **CEM-ML declarative document lifecycle engine** at 13:45
on November 12, with a break at 14:45 CET and default room L016 / Hypatia.
Use that title; the architecture title remains the proposal's framing.
The [discussion timeline](https://github.com/EPA-WG/cem/discussions/24) supplies
the six intervals retained in the minutes. The organizing committee's archived
CFP gives an October 12 materials deadline; correspondence must still establish
any later or session-specific requirements. Slides and a native-deck PDF are
delivered; runnable tutorial completion, delivery requirements, CLI release
verification and rehearsal evidence remain open. Studio's readiness is tracked
separately. The proposal's planned
release is not evidence that a release was published.

## Implementation checkpoint to preserve

The presentation preparation checkpoint was committed and pushed to
`origin/develop` as `33e18c99` before the October 11 agenda revisions. It saves
the pitch, manifesto, evidence, and earlier planning; the current minutes add
public-agenda alignment after that checkpoint.

The switch began from clean `develop`, synchronized with `origin/develop` at
`9de4302a43df71a0b1db4f8231453e05f6b494b8`:
`feat: complete datatype contracts and native component migrations`.
This commit includes the preceding accumulated work and the TODO cleanup. Its
`docs/todo.md` is the comparison checkpoint for the 16 implementation actions;
do not reset the working tree to restore it or overwrite later progress.

The [2026-10-10 archive](../archive/todo-snapshot-2026-10-10.md) preserves completed
topics, decisions and verification evidence. Its checkbox states are historical;
the active TODO holds the remaining work. At the switch:

- Native datatype/function composition, typed preludes, authorized datatype
  overrides and rebinding, pretyped provenance, sealed preparation evidence,
  whole-list inheritance and related serialization/replacement work are
  implemented, with their contracts and fixture evidence preserved in the archive.
- Shared native-surface body preparation, launcher/close controls and nested
  menu-to-task focus relay are implemented. Dialog, dialog-shell, tooltip, sheet,
  checkbox, radio, switch and autocomplete have canonical declarations and
  shared runtime capabilities. Required-marker accessible names are corrected.
- Component inventory is **20 canonical / 33 legacy / 53 legacy authored code
  files**. This is a dated checkpoint, not a count to repeat without checking
  after further implementation.
- The latest autocomplete checks passed 11 component/shared browser plays,
  22 workflow/primitive regressions, and source/packed/bundle/forced-color checks.
  Build/package and static verification evidence is in the archive. The cleanup
  also checked 141 links/anchors and passed the declarative gate from Nx cache.
- Physical touch/pen/IME, native picker, platform Back/gesture and browser/AT
  acceptance remain open. Historical aggregate browser failures also remain
  open; passing focused tests do not establish a green full legacy suite.

These boundaries matter when selecting conference claims and demonstrations.
Use the implemented subset for the tutorial without treating the conference
deadline as completion of the remaining component migration or device coverage.

## Return to implementation

When the presentation, supporting demo/setup files and rehearsal evidence are
ready, record their paths and status here, close/archive that presentation set
in the TODO, and continue with [CEM Studio](../todo.md#next-priority-cem-studio).
Preserve any implementation changes actually made during preparation and update
their evidence individually. The earlier queue's 16 actions remain open until
their own acceptance requirements are met.

After the Studio goals, resume at
[cem-datepicker](../todo.md#remaining-declarative-ui-migration), starting with
its recorded template-brace defect and a native fixture. Continue with
timepicker/slider, navigation and interactive content owners, remaining legacy
components and Studio/Site as listed in the TODO. Accessibility/device acceptance,
historical readiness investigations and deferred roadmap sequencing keep their
existing boundaries; conditional design work still requires its explicit request.

Component work continues under the [declarative UI principle](../declarative-ui-principle.md)
and [component development pattern](../component-development-pattern.md): canonical
XHTML/CEM-ML, colocated CSF Next plays, scoped token CSS and shared runtime
capabilities, with source/packed/bundle galleries. Add fixture actions to the TODO
before implementation and preserve meaningful assertions.

Use known Nx targets through the workspace package manager. The working Node
installation at this checkpoint is `/home/suns/.nvm/versions/node/v24.16.0/bin`;
prefix commands with that path if the next shell lacks Node/Yarn. Rebuild
`cem-elements` before comparing legacy browser suites, which consume its packaged
output, and avoid rebuilding component outputs during running browser gates.
