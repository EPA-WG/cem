# Virtual application proposal revision

Design-only completion, 2026-11-23. This revises the
[application](../../packages/cem-components/docs/application.md),
[command](../../packages/cem-components/docs/command.md) and
[localization](../../packages/cem-components/docs/localization.md) proposals;
it does not implement a component/runtime API or claim iframe security.

- [x] Define page-level and embedded virtual apps using the native CEM context,
      inherited/insulated providers, module maps/namespaces and downward-only
      container overrides of all or selected resource/context values.
- [x] Describe container/child command exports, parent service requests and
      MDI-style management, while leaving focus/active-app selection TBD.
- [x] Specify active/background priority, paused/offline admission and ancestor
      stops, latest-environment reconciliation and no event replay.
- [x] Specify immediate generated instance identity/name, save/load/clone/offload
      goals, private storage/logical URLs, fresh clone admission and existing
      privacy/checkpoint/shared-runtime constraints.
- [x] Align environment locale/theme matching, available defaults and app/container
      settings with semantic token ownership, accessibility and forced colors.

Reviewed against native context/resolver/namespace/grant, operation-control,
checkpoint and declarative UI contracts, with primary browser/MDI sources and
modern web scheduling guidance. Local links/anchors, preserved unfinished todo
inventory and whitespace were checked. No fixture, token, style or executable
behavior was changed; runtime/browser/device/AT acceptance remains future work.
