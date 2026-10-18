# Declarative Amsterdam 2026 — presentation handoff

Started 2026-10-10 before clearing chat context; updated 2026-10-11 after agenda
alignment. The user requested a temporary switch from implementation to preparing
the CEM presentation for Declarative Amsterdam 2026. The user changed priority
on 2026-10-11: resume CEM components now, starting
with cem-datepicker. Presentation and Studio work are deferred with their
materials and unfinished actions preserved.

Priority update 2026-10-17: Studio graph-edge editing and validation are complete.
Prioritize Studio functionality and the Amsterdam sample set
over unrelated device acceptance and helper work. The existing required tutorial
checkpoints and login-demo plan define sample scope; implement what the
presentation will show first. Track implementation in the
[active sample checklist](../todo.md#amsterdam-presentation-samples-active-studio-priority).
Shared declarative component principles remain mandatory. This reactivates sample
functionality; it does not establish that the complete tutorial or participant
Studio route is ready.

### Studio sample audit 2026-10-17

Product correction: Studio serves ordinary example projects. Predefined and
user-created/customized projects share one workspace, resources, configurations,
commands, diagnostics and persistence. Amsterdam is catalog metadata. There are
no dedicated presentation/example screens or numbered tutorial configurations.

Home offers **Login** and **Schema examples** under **Example projects**.
**Create project from example** makes an independent editable copy, then
**Projects → Open project** enters the common workspace. Existing projects keep
their saved contents. Startup restores the last selection; a fresh profile opens
Home without auto-installing a seed. Login's README resource carries instructions. Version 2.5.2 has thirteen
configurations, including XPath and **(CEM-QL)** variants of **Login semantics
from HTML** and **Login semantics from CEM-ML**. Their queries select the XHTML `main` subtree; document
wrappers and lexical formatting remain outside the comparison.

| Required presentation content | Available project content | Remaining work |
| --- | --- | --- |
| Read/edit/validate/inspect | Login CEM-ML source with shared-source parse, validation and inspection configurations. | Preserve existing three reference warnings in expected results. |
| HTML import, conversion and semantic comparison | Login HTML resource, HTML-to-CEM-ML conversion, and two ordinary query configurations share a 21-field semantic contract with the CEM source. Native import retains decoded values and provenance; the query explicitly maps namespace/mirror differences. | Preserve the bounded comparison contract and expected values when adding later examples. |
| Intentional failure and repair | Separate syntax-valid source omits the form method; owned teaching schema reports `demo.login.method_required`. | Continue to demonstrate source selection, saved repair/reload and repeat failure through the shared editor/diagnostics. |
| Equivalent queries | Login has native XPath and CEM-QL module resources with the same complete 21-field contract for both source formats. Ready namespaces and bounded navigation are explicit; query modules can be edited, validated and rerun. | Preserve source ownership, prefix independence, Boolean presence and offline copy isolation. |
| Template output | Two Login render configurations share an editable native CEMT view and complete expected HTML. Both sources support saved heading/label/method edits and template instruction edits. Results uses the shared sandboxed document preview and exact HTML copy/download. | Preserve exact HTML and saved source/template edits in direct and graph execution. |
| Graph and source provenance | Login graph imports the saved CEM resource, applies the shared view and exports HTML. Existing graph controls edit its output destination; native URI/range rows open the template with draft/revision guards. Offline copies preserve independent graph/template bytes. | Constructed output maps to template instructions/expressions; arbitrary computed data lineage is not claimed. |
| Live light-DOM view | **Login live view** opens the saved CEM template and HTML data in the shared script-free preview. Shared fields/action/alert remain interactive; theme and transient state stay separate from Studio. | Saved template/data changes, offline reload and independent copies are covered by the deployed gate. |

Native source/diagnostic contracts remain unchanged. Login archive updates and
collection metadata never overwrite existing saved projects. Continue from the
active TODO; implement presentation content as project data and missing general
capabilities in the shared libraries. Device/AT acceptance and rehearsal remain
independently open.

The unified-project correction passes 3 native Login fixtures, 116 Studio
browser tests and the full 51-task delivery gate. Deployed evidence includes
README access, all five unnumbered Login configurations, failure/repair, independent
copies and offline recovery through the ordinary workspace. Automated AX/reflow,
five themes/forced colors, deterministic packaging and clean consumer installation
also pass. Generated screenshots and Vitest attachments are excluded from Git.

The Login graph/source-provenance increment passes 155 selected native tests and
all 129 Studio browser cases through the full serial 51-task delivery gate.
The deployed workflow edits the export destination, follows generated ranges to
the template, preserves saved source/template/graph edits across offline reload,
repeats schema repair and verifies independent copies and their source URIs.
Draft/revision, UTF-8, EOF-caret and disposal guards pass. The clean-build pair
matches across 264 files with SHA-256
`f79ce8f431d16eb4ce9d95289c8b76609bf1a1da6876a28c2723725cffb71387`.
Theme/forced-color, automated accessibility, packaging and clean consumer checks
pass. UI stays declarative; native URI boundaries retain original source owners.
Constructed/computed output maps to template instructions/expressions, without a
claim of arbitrary per-character input lineage. Generated test artifacts remain
excluded. Native CEM-QL source Parse/Inspect is available from the shared source
controls: select `queries/semantics.cemql`, then parse its AST/events or inspect
its structure, diagnostics and source offsets without executing it. Both module
and standalone-expression identities retain original source coordinates.
Standalone-expression formatting is also available through an ordinary Convert
configuration with matching expression input/output identity. It uses the shared
native token formatter without module wrappers; invalid source retains original
diagnostic coordinates. Remaining presentation-needed Login resources and
configurations come next.

The Login live-view increment adds a shared `live-preview` capability in
`cem-elements`, declarative `cem-media-preview mode="live"`, and an ordinary
Login template/configuration. Five native sample fixtures, 130 Studio browser
cases and 16 focused shared plays pass. Deployed evidence covers real typing and
clicking, saved-versus-unsaved template changes, saved HTML data, offline reload,
independent copies, theme isolation and normal/forced-color paint. No Studio UI
JavaScript is added. Resource bodies remain original text until native import;
script-free frames own separate declarations, styles and transient form state.
Scripts, navigation, ambient storage and network fallback remain disabled.
The complete serial 52-task Studio release gate passes, including clean consumer
installation and synchronized evidence with `projectLiveView`. Two clean builds
match across 268 files with SHA-256
`9f3f1e21b5f4a5c49a09754d29b13ddfc747cd844ac0a17c11af2ae95cb12eee`.
Generated screenshots and Vitest attachments remain excluded from Git.

## Resume after clearing context

Read the repository's [CLAUDE.md](../../CLAUDE.md), then [docs/todo.md](../todo.md).
That file remains the sole execution checklist. Its Studio presentation-sample
queue is active under the October 17 priority update. The
[deferred presentation actions](../todo.md#deferred-work-declarative-amsterdam-2026-cem-presentation)
retain the remaining deck, tutorial, release and rehearsal work.

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

Priority update 2026-10-11: the user has resumed component implementation before
presentation completion. The earlier presentation → Studio → components sequence
below is historical; use the active TODO order and preserve unfinished work.

When the presentation, supporting demo/setup files and rehearsal evidence are
ready, record their paths and status here, close/archive that presentation set
in the TODO, and continue with [CEM Studio](../todo.md#deferred-priority-cem-studio).
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
