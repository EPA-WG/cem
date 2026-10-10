# Declarative composite menu capability

A declaration requests `capability="composite-menu"` and produces one direct
`part="composite"` container. Its direct links/buttons and direct child hosts'
`part="control"` controls form the collection. A control host may project a
direct child with `slot="submenu"` that uses this same capability. No component
tag names are required. `keyboard="menu"` on the root enables the composite;
`direction`, `dir` and `justify` retain declaration ownership.

The browser-independent navigation/position contracts have Node unit tests.
This addition performs no native AST transformation; existing native capability
registration transports its name and version. Browser lifecycle adapters own
focus, trusted event handling, roving tabindex, roles and popup relationships.
IDs and ARIA attributes are preserved across desired-DOM merges only at the
capability ownership boundary. Observers reconcile projected item changes;
listeners and observers are removed on disconnect and inert documents are
ignored. Native mode also suppresses disabled projected link activation.

`action-control` provides a `hasSubmenu` slice from the retained named payload,
blocks disabled native actions and lets the declaration choose a button owner
or a leaf anchor. Neither capability renders component markup.


The `popup` capability consumes direct `part="base"` and `part="popup"`
containers. A native button/link or tabindex trigger in the base owns activation;
`open="false"` closes. Its controller shares geometry and panel visibility with
composite-menu. Composite Escape stops at the nearest open submenu; an unhandled
root Escape bubbles to popup. A leaf activation emits `cem-popup-dismiss`, which
popup ancestors use to dismiss the chain. No separate document key handler is
installed, so independent popup/menu siblings keep their own focus.

Task-opening leaf activations participate in the shared `native-surface` relay.
The registered menu chain and outer popup launcher are captured before hiding
ancestors, including independently linked submenus. While task preparation is
pending or rejected, ordinary leaf dismissal is suppressed. Successful preparation
dismisses the captured transient chain without restoring menu focus, then enters
the task once. Eligible task exits return to the stable launcher; outside and Tab
destinations are preserved. Source-chain changes invalidate pending handoff.
Popup dismissal treats independently linked submenu controls as part of its
interaction region. Relay registrations are released on disconnect.
