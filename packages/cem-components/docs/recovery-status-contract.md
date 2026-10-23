# Recovery status

`cem-recovery-status` renders metadata supplied by the shared
`CemRecoveryStatusObserver` in `cem-elements`. Studio consumes the same declaration;
its JavaScript only connects the shared service to its declarative shell model.
The observer and component do not enable automatic recovery.

## Observations and authority

`runtime.observeRecoveryStatus()` returns a host-owned observer for that runtime's
publication coordinator and recovery scope. Other hosts can construct
`new CemRecoveryStatusObserver({ publication, recoveryScope })` explicitly.
`snapshot()` reads only existing inspection metadata. `subscribe(callback)` emits
immediately (unless suspended) and polls once per second while subscribed; identical snapshots are
coalesced. `refresh()` allows an explicit metadata refresh. Unsubscribe or dispose
with the view. Subscriber exceptions cannot disrupt the observer or its peers.

The frozen `cem-recovery-status/1` DTO contains only `state`, `reason`, six counts,
`automaticRecovery: false`, and `credentialPolicy: "block-entered"`. It drops
owner IDs/tags, runtime IDs, source/field bytes, arbitrary errors and policy stamps.
The scan includes nested preview publication coordinators, with a hard 256-domain
ceiling. Runtime counts describe registrations, not distinct physical WASM heaps.
A missing publication coordinator reports `unconfigured` even when runtime
registrations exist. Removed/disposed members are observed on the next refresh.

| State | Meaning |
| --- | --- |
| `unconfigured` | No checkpoint publication domain is configured. |
| `observing` | All inspected owners have a publication and adapter; eligibility remains unchecked. |
| `busy` | Preparation, capture or admitted work was observed. |
| `blocked` | Unknown outcomes, missing publications/adapters, or observation bounds prevent an affirmative observation. |
| `unavailable` | A scope/domain was disposed or inspection is unavailable. |

There is deliberately no `ready`, `recoverable` or `current checkpoint` state.
Observation never calls owner-current hooks, native telemetry, exports, capture,
preparation, policy callbacks or action replay. A publication receipt is not a
live-property fingerprint or complete physical-runtime inventory. Full privacy,
consent, admission, source, byte-budget, ownership and freshness checks still run
through the original recovery transaction, including combined fallback coverage.

Status updates are host inputs. Before draining/capturing/retiring a generation,
the host calls `const release = observer.suspend()`. Nested suspensions are
idempotently released. Polling and subscriber publication stay paused until the
last release; explicit `snapshot()` remains read-only. Release only after all
fresh owners and publications settle. The latest metadata is then emitted and
polling resumes. This prevents the observer from invalidating its own checkpoint
barrier. Suspension grants no capture, retirement or resume authority.

## Declarative surface

Bind `state`, `reason`, `runtime-count`, `owner-count`, `acknowledged-count`,
`admission-count`, `unknown-operation-count`, and `unsupported-owner-count` from
the named DTO fields. Counts are nonnegative integers (default `0`). `state`
defaults to `unconfigured`; `reason` defaults to `not-configured`. Supported
reasons are the observer's fixed code set. Unknown states/reasons use an
unavailable fallback; arbitrary error strings are never presented.

`label` defaults to **Recovery** and supplies visible fallback text. The default
slot replaces it with projected markup. `aria-label` optionally names the native
summary separately. Standard `dir` and presence-based `hidden` remain available.
The polite atomic live region announces the short status; details contain counts
and policy, without an action that could enable recovery. The native disclosure's
open/closed state is retained through the declarative `toggle` / `@newState`
binding, including ordinary status updates. No component-specific JavaScript or
additional capability is required.

Styles consume inherited CEM palette, typography, gap, inset, coupling and stroke
endpoints. The native summary retains its operable target and keyboard outline;
forced colors use system colors. No theme defaults or storage keys are introduced.

## Credential policy

Entered passwords and one-time codes **block recovery**. Only the user clears
them. The observer does not inspect, clear, redact, persist or infer credentials;
a property-only password-manager edit can therefore leave observed metadata
unchanged. The display explicitly says eligibility is unchecked. The original
capture-time credential classifier and complete-owner checks remain mandatory,
including refusal of stale private slices/events after a DOM value is cleared.
Do not label this observation as credential safety, checkpoint availability or
permission to restart. See the [committed checkpoint policy](../../../docs/committed-checkpoints.md).

Try the [property playground](../playgrounds/cem-recovery-status.html),
[full examples and variation matrix](../playgrounds/cem-recovery-status-gallery.html),
and [automated stories](../src/components/cem-recovery-status/cem-recovery-status.stories.ts).
