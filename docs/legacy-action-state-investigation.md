# Legacy action state investigation

2026-09-28. Status: contract decision needed before owner migration.

## Results

After rebuilding `cem-elements`, the unchanged legacy state file still reports
**3 passing / 18 failing**, matching the recorded state-suite baseline.
The first failure is `cem-icon-button`'s empty `disabled` attribute: its native
button remains enabled. Both legacy action templates bind the attribute value
rather than its presence.

A temporary, test-owned declaration probe replaced only the icon-button and
menu-item disabled expressions with the existing canonical expression:

```cem
@disabled={if seq:count(datadom.attributes.disabled) > 0 { true } else { null }}
```

Every original test and assertion remained in place. The probe reports
**5 passing / 16 failing**, with no added failing test names. These tests pass:

- `reflects action, loading, disabled, expanded, selected, and focus states on native controls`
- `applies shared native hover treatment without changing action geometry or semantics`

The active-state test advances beyond disabled reflection and fails later at
`expect(keyboardReleased.runtime).toBe(keyboardActive.runtime)`. This is a
previously masked failure, not an additional failing test name. The other failed
tests remain outside this bounded attribution.

The probe registered copies of the two legacy declarations before the ordinary
installer, replacing only their disabled expression. It changed no production
source. The temporary test edits and failure screenshots were removed after the
comparison; the committed legacy test file is unchanged.

## Contract conflict

The [active contract](../packages/cem-components/docs/action-active-contract.md#keyboard-activation)
says repeated keyboard activation is “intentionally idempotent in serialized
runtime data” after a pointer click already set `pressed` to `"click"`.
The assertion serializes `eventPayloads` as well as slices.

The recorded pointer click targets the icon `span`; the Space-release click
targets its native `button`. The payload also updates pointer coordinates to
zero and increments its revision from 1 to 2. The stored slice remains `"click"`.
The runtime is retaining the newest native event, so whole-snapshot equality
cannot describe this interaction accurately.

The new browser regression
[`click-payload.stories.ts`](../packages/cem-elements/src/lib/click-payload.stories.ts)
passes without changing runtime behavior. It checks:

- Two trusted native clicks: pointer on icon content, then Space release.
- No extra click or payload update while Space is held.
- Equal slice values after both clicks.
- Correct `target` and `currentTarget` for each event, and a refreshed payload.
- The same native button remains mounted.

## Decision and recommendation

**Recommended:** keep latest-event payload semantics. Revise the active contract
and its release assertion to distinguish unchanged slice values from refreshed
click metadata. Preserve full snapshot equality during the held interval, native
click counts, focus, geometry, styling and exact target assertions. This changes
an incorrect post-click invariant; it does not remove activation coverage.

The alternative is to require the complete serialized snapshot to remain
unchanged when the slice value repeats. That would require deciding which new
native event metadata to discard or retain elsewhere and changing shared runtime
behavior. No runtime behavior was changed during this investigation.

Execution stops for this decision under the user's instruction to stop at open
questions. The legacy component implementation remains frozen migration debt.
After approval, migrate `cem-icon-button` and `cem-menu-item` to canonical XHTML
with explicit disabled presence, colocated stories and the accepted companion
playground pattern; compare the complete legacy failure inventory afterward.

## Verification evidence

```sh
yarn nx run cem-elements:build
yarn nx run-many --targets=test --projects=@epa-wg/cem-components --excludeTaskDependencies --skipNxCache --args='src/lib/states.browser.spec.ts --run --reporter=json --outputFile=/tmp/cem-states-before.json'
yarn nx run cem-elements:test packages/cem-elements/src/lib/click-payload.stories.ts
```

Local diagnostic files: `/tmp/cem-states-before.json`,
`/tmp/cem-states-probe.json`, `/tmp/cem-states-payload-probe.json` and
`/tmp/cem-click-payload.log`. The full legacy state suite remains red; the new
runtime regression passes. The temporary probe's improvement is evidence for
migration, not a shipped fix or a green package gate.
