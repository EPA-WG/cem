# Completed graph parameter-map authoring — 2026-10-23

Six completed execution tasks are preserved from the [active TODO](../todo.md).
The 32 other original unfinished tasks remain there verbatim and in order.
Earlier delivery evidence remains in the
[compact Studio record](todo-completed-compact-studio-2026-10-23.md) and
[prior snapshot](todo-snapshot-2026-10-23.md).

## Completed tasks

- [x] Extend native graph authoring to node/edge creation and parameter maps;
  preserve authored IDs and resource references, validate each candidate and
  verify transactional persistence/history/recovery/offline execution. Add
  fixtures before implementation and shared controls before Studio composition.

  - [x] Add native fixtures and parameter-map authoring, then verify Save,
    history, recovery and explicit offline Run for the expanded graph editor.

  - [x] Add native parameter add/update/remove contracts before implementation:
    preserve unrelated graph nodes, IDs, resource references and source-only
    expressions; distinguish empty text from removal; reject ambiguous targets,
    mixed operations, unsafe serialization and source/node limit violations.

  - [x] Compose parameter-map forms from shared declarative controls and native
    descriptors; verify stale/repeated/unexpected fields, pending disabling,
    native form reset, reversible drafts and unchanged repository revisions until Save.

  - [x] Verify equal-valued parameter and owner switches discard unsubmitted
    staging without persisting it. If needed, extend the shared `value-request`
    convention for text controls, including consumed-token checkpoint/resume,
    repeat-token edit preservation and unchanged credential refusal.

  - [x] Verify parameter-map Save/reload, bounded history, draft recovery and an
    explicit offline native graph Run with the authored parameter value; retain
    credential recovery refusal and the existing 4 MiB checkpoint budget.

## Adopted contracts

The [native graph authoring boundary](../cem-studio.md#bounded-graph-configuration-authoring)
accepts one mutually exclusive `parameter: { node, name, value }` mutation;
`value: null` removes one entry and `value: ''` keeps empty text. Native CEM AST
parsing, whole-graph validation and preservation checks own the transformation.
Template caller parameters must be public globals or local to the selected
entrypoint. Descriptions expose literal entries and `canEditParameters`; source-only
expressions, explicit element namespace identities and removal of parameters
with extra authored metadata remain outside the structured editor. Authored
IDs, aliases, resource references and unrelated graph edges/parameters survive.
Limits remain 100 nodes / 1 MiB source and 20 history snapshots / 8 MiB.

Studio composes shared declarative fields, multiline text, selects, table,
fieldset and actions in XHTML/CEM-ML. Services stage native edits and guarded
repository transactions. A shared `cem-elements` text-control `value-request`
extension resets unsubmitted staging when parameter/owner identity changes,
including equal-valued selections; repeating an already consumed token retains
live edits. An explicit request with no authored `value` resets to the empty
default rather than replaying a prior value. Tokens carry identity rather than entered values. Consumed-token
checkpoint/resume prevents an authored-default reset on a fresh runtime, and
late request changes invalidate activation without clearing entered values or
manufacturing input/change events. No Studio-local UI JavaScript is introduced.

Apply, Reset, Undo and Redo do not Save or Run. Explicit Save commits accepted
source; explicit Run uses the persisted graph. Portable draft recovery retains
unsaved parameter changes independently of repository revisions. Entered
passwords or one-time codes still block recovery under the
[credential privacy policy](../committed-checkpoints.md#credential-privacy-policy-for-live-login-recovery).
The aggregate 4 MiB checkpoint budget remains unchanged and automatic
whole-Studio recovery stays disabled.

## Verification evidence

| Gate | Recorded result |
| --- | --- |
| Native graph authoring | **21/21 passed**: add/update/remove, empty literals, no-ops, preservation and refusal contracts. |
| Native folder/build regression | **15/15 passed** with graph-authoring request compatibility retained. |
| Native checkpoint runtime state | **40/40 passed**, including optional consumed-token DTO round trips and malformed-value refusal. |
| Shared text-control browser contract | **5/5 browser stories passed (3.85 s)**: controlled requests, repeat/removal/reintroduction, empty/native reset, omitted-value default reset, focus and retained metadata/currentness. |
| Shared runtime unit regression | **782/782 passed across 79 files (4.95 s)**. |
| Shared runtime and Studio typecheck/lint | **Passed with 0 errors**; pre-existing warnings remain warnings. |
| Studio parameter-form acceptance | **4/4 passed (20.84 s)** after the rebuilt native query/runtime delivery; includes equal-valued parameter/owner switches, reset/history and guarded native submissions. |
| Shared fresh-runtime form recovery | **3/3 browser stories passed (13.28 s)**: fresh worker/fallback and retained consumed-token recovery, plus late request refusal without losing live edits. |
| Complete Studio browser cohort | **137/137 passed across 3 files (662.21 s elapsed; 660.63 s tests)**: 111 workbench + 18 navigation + 8 recovery. Existing layout, native editing, credential refusal and 4 MiB budget assertions/deadlines remain intact. |
| Deployed workspace lifecycle | **Passed** with native parameter/node/edge authoring, Save/reload, draft backup/recovery and explicit offline Run. The four-artifact build retains its SVG outputs and emits exact authored HTML `<p>Authored café 🍒</p>`. |
| Aggregate serial release | **53 Nx check tasks passed (42 from valid cache)**. Includes deterministic delivery, deployed native parameter output and offline lifecycle, package, clean consumer and synchronized release evidence; generated CLI schema preparation was removed afterward. |

The [native authoring fixtures](../../packages/cem_ml/tests/transform_config_authoring.rs),
[checkpoint DTO fixtures](../../packages/cem_ml/tests/checkpoint_runtime_state.rs),
[shared form stories](../../packages/cem-elements/src/lib/form-control.stories.ts),
[fresh-runtime form stories](../../packages/cem-elements/src/lib/checkpoint-publication.stories.ts),
[Studio fixtures](../../packages/cem-studio/src/workbench.browser.spec.js) and
[deployed lifecycle verifier](../../packages/cem-studio/scripts/verify-static-worker.mjs)
retain executable assertions. Generated screenshots and Vitest attachments stay
excluded from Git. Automated results do not establish physical-device or spoken
assistive-technology acceptance; those original TODOs remain open.
