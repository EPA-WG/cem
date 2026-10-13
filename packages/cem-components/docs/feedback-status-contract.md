# Feedback and status declarations

The canonical owners are `cem-toast`, `cem-alert`, `cem-progress`,
`cem-progress-spinner` and `cem-skeleton`. Each ships XHTML, embedded scoped
styles, colocated asynchronous plays, a property page and a five-theme gallery.
Their former registry declarations and spinner behavior module are retired.

## Semantics and lifecycle

| Owner | Contract |
| --- | --- |
| Toast | A polite `role="status"` / `aria-live="polite"` message. Default payload replaces the `label` fallback (`Status`). The application controls its presence and lifetime. |
| Alert | Inline feedback. `tone` is bounded to `info`, `success`, `warning`, `danger`, with unsupported values using `info`. `role="alert"` explicitly requests urgency; absent or unsupported role uses `status`. Tone never silently changes announcement urgency. Default payload replaces `label` (`Alert`). |
| Linear progress | One native `<progress>` named by `label` (`Progress`). Missing `value` is indeterminate; present values, including the empty string, remain present. The browser normalizes value and maximum. Absent `max` is authored as 100; an invalid or nonpositive authored maximum follows the browser's maximum of 1. |
| Circular progress | One labeled progressbar and retained decorative SVG. Shared `progress-status` preparation normalizes finite values without rewriting host attributes. Its maximum fallback is 100, and values clamp to 0..max. Missing value alone means indeterminate. `describedby` references additional task context. |
| Skeleton | An `aria-hidden="true"`, inert placeholder. Default payload replaces `label` (`Loading`). Its content cannot create keyboard stops inside an inaccessible subtree. It has no live region or automatic motion. |

These owners do not infer task completion, loading or availability from external
work. They introduce no timers, dismissal events, focus transfer or keyboard
handlers. Progress has no content slots or live announcements; the task region
owns `aria-busy` and contextual status. Toasts and alerts retain native behavior
in authored descendants. Skeleton descendants are decorative.

Attribute updates and reconnect preserve the existing owner, projected payload
and circular SVG. Native host `hidden`, `class` and inherited `dir` remain
available. `hidden="false"` hides by presence; `hidden="until-found"` retains
native browser behavior.

## CEM principle audit

| Principle | Binding and evidence |
| --- | --- |
| Semantic intent | Status messages use palette pairs, boundary stroke, container inset and UI typography. Info/success/warning/danger map to trust/calm/enthusiasm/danger. They consume no action-state tokens. Native linear progress consumes the progress indicator accent; circular progress consumes the dedicated track and indicator pair. |
| Bounded variation | Alert tone and urgency have explicit fallbacks. Numeric progress clamps in its owning native or shared contract. Geometry and animation have no component-local public defaults. |
| Accessibility | Native progress and the circular progressbar have names. SVG is hidden and non-focusable; unknown work omits numeric ARIA. Toast/alert urgency is explicit. Decorative skeleton payload is inert. Spinner keyboard/pointer fixtures preserve event neutrality. |
| Cross-dimension harmony | Message inset, boundary and typography consume their respective theme endpoints. Skeleton geometry uses the established control height and bend endpoints. Circular diameter/track thickness remain D2c graphics, with D0 paint and D7 duration/easing. |
| Theme survivability | Colocated plays verify inherited overrides in all five theme modes. Source and packed browser gates exercise actual owners in forced colors and reduced motion. Spinner uses `GrayText` / `Highlight`; messages preserve system foreground/background; skeleton retains a system boundary. |
| Public ownership | All public tokens remain theme-owned and inherited. No new aliases, parallel token families or component defaults are introduced. Styles install once per declaration with public `part` hooks and native scope. |

Scoped circular animation uses a static `animation-name` plus token-valued
duration/easing longhands. This follows the supported scoped-CSS contract;
dynamic substitution inside animation shorthand cannot establish name ownership.
The compiler gives the keyframe a declaration-owned name. Reduced motion stops
rotation while retaining the same incomplete arc, geometry and ARIA. Linear
progress retains platform rendering and its native motion policy.

## Verification and limits

`cem-elements/src/lib/progress-status.stories.ts` exercises generic preparation
in worker and fallback runtimes, including large finite values, fractional
ranges, invalid inputs, reconnect and event neutrality. Colocated component
plays retain all former spinner assertions and cover native progress, message
projection, bounded alert states, inert placeholders and theme inheritance.

`@epa-wg/cem-components:verify-feedback` checks property controls, semantic
owners, five-theme galleries, retained identity, forced colors, reduced motion
and bundle metadata in source and isolated package archives. The dedicated
spinner forced-colors gate retains exact geometry, paint and input assertions.

Browser DOM checks do not establish spoken announcements or physical-device
acceptance. Those matrix runs remain explicit TODO items. Toast queueing,
auto-dismissal and overlays remain outside this contract.
