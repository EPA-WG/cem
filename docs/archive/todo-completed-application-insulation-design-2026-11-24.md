# Insulation-first application proposal revision

Design-only completion, 2026-11-24. This updates the
[application](../../packages/cem-components/docs/application.md),
[command](../../packages/cem-components/docs/command.md) and
[localization](../../packages/cem-components/docs/localization.md) proposals.
It supersedes the earlier inherited-by-default context composition and the
suggestion that CEM-only apps require separate browser realms for virtualization.
No executable capability or security acceptance is claimed.

- [x] Make each app private by default, reusing native scope ownership, typed
      properties and implicit inheritance inside the selected environment.
      Use explicit admitted bindings for parent/group sharing; add no generic
      inheritance flag or whole-app context-mode switch.
- [x] Define `cem-elements` as the guarded browser-service boundary, with no
      direct unrestricted app browser API, native API wrappers or manually
      prefixed app storage keys required by the declarative app contract.
- [x] Retain EPA-WG/embed-page commit
      `dde611e450ef7d901df6054a6545250e0862af49` and its local/session storage
      tests as the concrete insulation-parity reference. Specify complete
      prefix/key/event/session behavior, stable identities, named sharing,
      lifecycle fencing and save/load/clone rules for later CEM implementation.
- [x] Require equivalent logical APIs for iframe and embedded hosts; ordinary
      same-origin frames retain the CEM layer, and cross-origin hybrid frames
      add browser enforcement. Preserve native permissions/activation limits.
- [x] Keep immutable resource reuse, named context bindings, execution groups
      and mutable providers distinct. Preserve downward container authority,
      theme ownership/accessibility and explicit focus/active-app TBD status.

Verification: local document links/anchors, proposal consistency, unchanged
unfinished todo inventory and whitespace checks. Reference source/tests were
inspected; no new browser/native fixture or runtime test was added or run.
No component, theme token, stylesheet or executable runtime behavior changed.
