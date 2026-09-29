# Field control migration: form contract decision

Status: decision needed before migrating `cem-field` and `cem-text-field`.
This note records investigation evidence; it does not change runtime semantics.

## Evidence

Temporary canonical XHTML prototypes used the existing value binding:

~~~cem-ml
{input @name={datadom.attributes.name}
       @value={datadom.slices.value ?? datadom.attributes.value}
       @slice=value @slice-event=input @slice-value="$target.value"}
~~~

Browser reproduction: render a field inside a form with `value="initial"`,
type `edited`, then click a native reset button. Expected: `initial`.
Actual for both fields: `edited`.

The focused Storybook run had eight tests across the two prototypes: four
passed (attributes/boolean presence and editing/busy retention), two reset tests
failed as above, and two indicator-paint tests failed. Paint requires further
investigation when component work resumes. The incomplete prototypes were
removed; production fields remain on the existing implementation.

Command used (runnable again once the migration fixtures are reintroduced):

~~~sh
yarn nx run cem-elements:test packages/cem-components/src/components/cem-field/cem-field.stories.ts packages/cem-components/src/components/cem-text-field/cem-text-field.stories.ts
~~~

## Why this needs a decision

[Component conventions](../packages/cem-components/docs/conventions.md#4-form-participation)
require form-associated hosts, validity forwarding and reset to the declared
default. Native child inputs alone do not fulfill the whole host contract.

The existing runtime
[input-value tests](../packages/cem-elements/src/lib/input-value.stories.ts)
explicitly assert that committing a bound value changes `input.defaultValue`.
Changing generic value patching would change that tested behavior.

The [declarative UI rule](declarative-ui-principle.md#hard-stop-capability-rule)
says: “When CEM-ML cannot express required behavior, or the expression would require
verbose repetition that obscures the component contract, STOP work on the
component or application UI.”

The runtime already exposes `formDisabled`, `formReset`, and `formStateRestore`
hooks. The shared `choice-select` capability demonstrates their use. Field
components cannot add local JavaScript to implement those hooks.

## Recommended decision

Approve an opt-in shared form-control capability in `cem-elements` that:

- keeps the declared reset default separate from the live value slice;
- handles reset, disabled fieldsets, validity and form-state restoration;
- defines one submission owner so host and native child do not submit twice;
- lets the component remain an XHTML/CEM-ML declaration.

Define and test this capability before resuming the field migration. Preserve
the current generic input-value patching contract.

The alternative is to revise the component form contract to accept native-child
ownership and current rendered reset defaults. That weakens the documented
field behavior and is not recommended.

After the capability passes, resume both field declarations, scoped indicator
paint, complete attribute stories and source/package playgrounds; then compare
the remaining legacy state failures.
