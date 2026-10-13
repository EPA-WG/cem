# Shared project navigation workflow

The [declarative example](../playgrounds/project-navigation.xhtml) composes
production CEM controls before Studio adopts project navigation. It is exported
as `@epa-wg/cem-components/playgrounds/project-navigation.xhtml`; load its
`#project-navigation-example` declaration after the component declarations and
mount it through `mountCemView`.

This is a bounded composition example, not a project repository or a new widget.
Its colocated [tree stories](../src/components/cem-tree/cem-tree.stories.ts) supply
scalar service stubs. Studio remains responsible for durable identities,
transactions, draft authority, stale completions, routing and persistence.

| Shared control | Responsibility |
| --- | --- |
| Passive `cem-list` with `cem-action` | Explicit project actions; no action-list or listbox claim. |
| `cem-tree` | Hierarchy, expansion and focus. Leaf activation sends an identity through the shared host bridge; the service decides when selection changes. |
| `cem-tabs` | Manual local pane activation, retained panels and native editor selection. |
| `cem-dialog` | Modal preparation, accessible naming, native close policy and focus return. |
| `cem-textarea`, `cem-alert`, `cem-surface`, `cem-stack` | Native editing, semantic status and layout using inherited theme tokens. |

The service projects `selected`, `draft`, `saved`, `dirty`, `pending`, `saving`,
`failed` and `status` slices. It provides `navigate`, `open-alpha`, `open-beta`,
`edit`, `save`, `discard` and `cancel` commands. The two named actions are fixture
resource identities, not a proposed Studio command API.

Dirty navigation retains the selected resource and opens a conditional dialog.
Cancel or native Escape preserves the draft. Save keeps the dialog mounted and
uses `closedby="none"` while pending; failure retains the draft and reports an
inline error. Success or Discard admits the new selection and removes the dialog.
The shared native-surface capability returns focus to the entry control when it
remains available and the dialog still owned focus. Application services never
query controls or call focus, show or close methods.

The named resource region isolates changing loading/empty/error content from
retained workbench panes. Source and installed-package navigation verification
exercise the composition at a compact viewport across all five themes, inherited
spacing/focus overrides and forced colors. Worker and fallback stories cover
keyboard/pointer activation, failed and delayed Save, Discard, Cancel, Escape,
retained editor identity/caret and explicit status states. Browser/device and
assistive-technology acceptance beyond the automated Chromium lane remains open.

CEM review: Save uses primary intent, Discard destructive intent, and Cancel
contextual intent. The declaration adds no CSS, token defaults or visual variants.
Native owners retain keyboard, modality and focus responsibilities; the service
owns navigation admission. The pinned Material tree, tabs, dialog and passive-list
subset boundaries remain unchanged.
