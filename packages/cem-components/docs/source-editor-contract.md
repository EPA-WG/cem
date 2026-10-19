# Source editor

`cem-source-editor` is a declarative document editor backed by the shared
`plaintext-editor` capability in `cem-elements`. It follows the existing
`cem-demo-element` playground's `contenteditable="plaintext-only"` convention.
Its editable `pre` is a named multiline textbox in the light DOM. Source enters
as text; HTML/CEM source is never executed or treated as editor markup. Syntax
highlighting and language services are separate future work.

Use `value` for initial/controlled source and `.value` for silent programmatic
replacement. Native `input` bubbles; declarative `$target.value` and host-command
bindings receive the admitted plain string, including native Enter/paste line
breaks. The editor preserves its DOM, caret, scroll position, native undo and
composition during unrelated renders. An accepted replacement changes the
source; a replacement received during composition waits until composition ends.
A retained value slice survives reconnection and serialized resumption.

Trailing source newlines remain exact. The browser's final caret placeholder
(a terminal BR, or a sentinel LF after a BR during paste) is excluded from the
admitted value. Unrelated renders compare that normalized value and leave the
native editing subtree intact, preserving caret and undo history.

`label` supplies visible fallback text and the default accessible name. The
`label` slot replaces visible fallback markup; `aria-label` names the textbox
separately. `describedby` links external help. `readonly` and `disabled` use
presence, including `="false"`; readonly retains keyboard text selection,
while disabled removes the tab stop. Neither permits native edits. This is a
document editor, not a form-associated field; use `cem-textarea` for form
submission, constraint validation and reset.

`fit="content|fill"` controls sizing. Fill needs a definite parent height; the
editor scrolls within that height. Native `class`, `dir` and `hidden` apply to
the host. Typography, surface, inset, boundary and focus use inherited canonical
CEM tokens. Forced colors use Canvas/CanvasText, GrayText and Highlight.

Source navigation uses `selection-request`, a new nonempty identity for each
request, plus nonnegative safe-integer `selection-start` / `selection-end`
UTF-16 offsets and optional `selection-direction="none|forward|backward"`.
Offsets clamp to the source length. Presence-only `selection-focus` explicitly
requests focus. Without it, an unfocused editor remembers the requested range
without stealing document focus. Requests received while disabled or composing
are consumed without applying. The host and owned control expose `.value`,
`.selectionStart`, `.selectionEnd`, `.selectionDirection` and
`.setSelectionRange(start, end, direction)` for generic adapters.

```cem
{cem-element @tag=cem-source-editor
 @src="@epa-wg/cem-components/components/cem-source-editor/cem-source-editor.xhtml#cem-source-editor"}
{cem-source-editor @label="templates/view.cemt" @value={datadom.slices.source}
 @slice=source @slice-event=input @slice-value="$target.value"}
```

The fragment inherits its source-owned `plaintext-editor` capability. Copy that
metadata when copying the template into a new declaration.

See the [property playground](../playgrounds/cem-source-editor.html),
[full examples and variation matrix](../playgrounds/cem-source-editor-gallery.html)
and [browser stories](../src/components/cem-source-editor/cem-source-editor.stories.ts).
