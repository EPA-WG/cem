# Native dialog contract

`cem-dialog` and the compatibility alias `cem-dialog-shell` are canonical XHTML
CEM-ML declarations using the shared `native-surface` capability. Each produces
one stable native `dialog[part="surface"]`. It starts closed; the default is a
persistent nonmodal task. `mode="modal"` explicitly requests browser modality.

```html
<cem-dialog label="Edit profile" trigger="Edit" close-label="Done">
  <label>Name <input autofocus name="name"></label>
</cem-dialog>
```

| Authoring input | Meaning |
| --- | --- |
| `label`, `slot="label"` | Visible heading and default accessible name. Explicit `aria-labelledby` or `aria-label` overrides generated naming. |
| `trigger` | Text for a generated native launcher; `trigger-aria-label` supplies its optional accessible name and presence-only `trigger-disabled` disables it. |
| `slot="trigger"` | One native control or shared control provider replacing the generated launcher. |
| `trigger-for` | Typed CEM node reference, `#id`, bare ID or nearest-scope `@name` for an external launcher. Do not combine it with a local trigger. |
| `close-label`, `slot="close"` | Generated close button or one authored replacement. Close requests use the shared cancelable lifecycle. |
| Default slot | Eager task body. Native form controls retain their values, identity, focus, validation and events across ordinary rerenders. |
| `template[slot="body"]` | Inert body with `materialize="retain"` (default for a template), `dispose` or `eager`. A template body cannot be combined with an eager body. Use a direct inert outer template to author a nested body template in the host payload. |
| `mode` | `nonmodal` (default) uses `show()`; `modal` uses `showModal()`. Change it while closed. |
| `popover` | Optional `manual` or `auto` top-layer nonmodal presentation. Modal plus popover is rejected. |
| `default-open` | Presence requests the initial opening once per produced instance. It is not live open state. |
| `surface-id` | Optional ID on the native owner for native `commandfor` relationships. |
| `closedby` | Forwarded native close policy. |
| `focus-target`, `return-focus` | Shared typed reference or scoped string; `auto` uses native entry and the actual invoker; `none` suppresses shared focus work. Native browser focus behavior still applies. |
| `anchor`, `boundary`, `placement`, `fallback`, `overflow`, `anchor-lost` | Shared surface geometry. Dialogs default to centered placement and freeze established geometry on anchor loss. |
| `context-change` | `reject` (default), `replace`, or cancelable `request` for an already-open task invoked with different context. |

`cem-action` commands `--cem-show` and `--cem-hide`, native commands such as
`show-modal` and `request-close`, launcher activation and close controls all use
the shared surface lifecycle. Native `form method="dialog"` retains validation
and `returnValue`. The shared `cem-before-open` and `cem-before-close` events are
cancelable; `cem-open` and `cem-close` report transitions. The runtime owns
preparation, placement grants, invocation context, nested menu-to-task focus
relay and cleanup; the component contains no behavior script.

A heading ID and launcher ARIA references are shared runtime claims. Rerender
preserves these claims; disconnect/rebinding releases only owned values.
Keyboard focus enters an authored eligible target or the browser's dialog
fallback. Only the actual focused owner/control receives its own token outline;
forced colors use `Canvas`, `CanvasText` and `Highlight`.

## Migration from legacy dialogs

Remove `transient` and live `expanded`. Use explicit `mode="modal"` where modal
behavior is intended, a launcher/command for ongoing activation, and
`default-open` only for initial opening. Replace `cem-dismiss` consumers with
the shared lifecycle or native dialog events. Static `div[role="dialog"]`
wrappers are retired; use a region component for permanently inline content.
`cem-sheet` remains a separate canonical labeled region with its existing
`transient`/`expanded` API.

Colocated stories cover names, native forms, modal/nonmodal state, slots,
retention/disposal, external references, reconnect and native keyboard focus.
The property pages and five-theme galleries are included in source, release
bundle and installed-package verification through `verify-surfaces`.
