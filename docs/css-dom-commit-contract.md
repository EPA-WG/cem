# CSS and DOM publication contract

Status: accepted on 2026-09-27 through the user's instruction to continue with
the recommended contract. Implementation is tracked in [todo.md](todo.md), under
atomic native CSS replacement in streamed Edge updates.

## Publication boundary

The prepared CSS and DOM APIs can reject invalid work before publication. They
cannot make several DOM mutations invisible to synchronous custom-element
callbacks. Changing an observed host context marker can invoke its attribute callback;
inserting a custom element invokes its connection callback. A callback can
inspect or mutate the same host before the surrounding JavaScript call returns.

The existing [patch renderer](../packages/cem-elements/src/lib/projection.ts)
describes atomicity as validating a complete
transaction and its targets before mutation. Combined publication follows that
admission boundary and permits intermediate observations from synchronous
custom-element callbacks, with explicit recovery after publication failures.

## Reproducible evidence

The [browser fixture](../packages/cem-elements/src/lib/css-dom-commit-order.stories.ts)
exercises actual native CSS output,
`DeclarationStyleOwnership.commitGroup`, and `preparePatchFramesForRange` in
worker and forced main-thread fallback modes. The host observes its context
marker; a child observes connection. Both orders end with the requested CSS and
DOM, but synchronous observers can see these intermediate combinations:

| Publication order | Callback | Observed DOM | Observed CSS |
| --- | --- | --- | --- |
| CSS, then DOM | Host context-marker change | Before | New |
| DOM, then CSS | New child's connection | After | Old |

This is a diagnostic fixture demonstrating why a sequential composition alone
cannot establish observer isolation. It is not an accepted coordinator or a
change to production behavior.

The fixture host is a synthetic custom element with `attributeChangedCallback`.
Generated CEM hosts instead observe attributes using `MutationObserver`; they
do not have that synchronous attribute callback. Native child connection
callbacks and explicitly invoked cancellation/release handlers remain relevant
to publication. The recommendation does not require isolating mutation observer
callbacks or removing CEM's observation of external DOM changes. See the
[CSS lifecycle audit](scoped-css-module-maps.md#css-lifecycle-and-mutation-observation).

Verification on 2026-09-27: 297 browser cases pass in both default and retained-CSS
lanes; typecheck and lint pass with two existing warnings.

## Accepted contract: atomic admission and coherent completion

1. Load and compile all CSS, snapshot the patch, and validate every ownership
   lease, context marker, DOM target, range and current request revision before
   the first publication mutation. Rejection here preserves the active state.
2. Publish CSS and DOM synchronously, with no asynchronous gap. Prefer CSS first
   so newly connected children can read their new styles. Arbitrary synchronous
   custom-element callbacks may observe intermediate state.
3. Defer CEM-owned old-load releases and cancellation notifications until both
   CSS and DOM publication finish. Queue managed reentrant update requests until
   that point. This does not suppress browser callbacks or author DOM mutations.
4. Once publication starts, distinguish failure requiring recovery from rejection
   before publication. If a callback changes a target or publication fails, report
   the need for an authoritative render/resume and do not claim rollback. Cloning
   old markup cannot restore arbitrary callback effects, native control state or
   listener ownership.
5. Test both callback orders, mutation during publication, cancellation before
   admission, supersession during publication and recovery before enabling
   changed-CSS Edge updates. Extend the response/state contract with the chosen
   completion and recovery semantics in the same integration work.

This follows the current patch renderer's admission boundary and preserves the
light-DOM model and retained node identities. It adds explicit handling for the
failure window instead of interpreting every failed commit as no mutation.

## Alternative: require observer isolation

Keep changed-CSS Edge updates guarded while defining stronger lifecycle
restrictions or a different publication architecture. Merely reversing the
publication order does not satisfy this requirement. Replacing a larger tree
would also need a separate identity, focus, native state and lifecycle contract;
it is not a drop-in implementation of the current ownership rules.

## Implementation status

Proceed with the admission/completion contract above. The synchronous stylesheet
notification boundary is the first implementation step: it defers ownership
release/abort notifications until the surrounding publication returns or throws.
Nested boundaries share a queue, and all queued cleanup runs even after failure.
`DeclarationStyleOwnership.commitGroupWithPatch` now jointly admits declaration
CSS and a prepared patch, checks that they belong to the same host, and reports
`applied`, `rejected` or `recovery-required`. It rechecks the revision and live
ownership after publication and cleanup. Nested joint calls are rejected so a
call cannot report success before an outer notification boundary finishes.
Prepared native declaration loads can now transfer their outputs once through
`takeCommit()` into joint publication. Admission and completion retain the
preparation's host/cancellation/release validity guard, and disposal still owns
native cleanup.
Prepared instance CSS, managed update queuing, authoritative recovery and the
Edge response/state contract still need integration. Changed-CSS Edge updates
remain guarded until those requirements are verified.

Joint publication verification: 575 unit tests and 298 browser cases in each default and
retained-CSS lane pass; typecheck and lint pass with two existing warnings. The
ordering fixture now also verifies deferred release/abort callbacks against real
native CSS and a prepared DOM patch in worker and fallback modes. The joint
fixture covers twelve scenarios including pre-publication rejection, cancellation,
target mutation, revision changes, cleanup failure and host disconnection.
