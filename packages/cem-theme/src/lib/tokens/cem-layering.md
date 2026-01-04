# CEM D4 Layering — Signed Depth, Planes, and Appearance Shifts

**Status:** Canonical  
**Last updated:** 2025-12-21  
**Taxonomy placement:** D4. Layering (treats *depth + elevation* as one consumer concept)

CEM uses **layering** as the umbrella term for:

- **Depth (signed):** behind vs in front of the base plane (*recess* vs *lift*).
- **Elevation (positive):** steps above the base plane (*raised*, *floating*, *overlay*, *command*).
- **Planes (rendering):** global stacking rules for overlays/modals, independent of local nesting.

**Companion specs:**
- **D0. Color (Emotional Palette)** ([`cem-colors.md`](./cem-colors.md)) — tonal "dig down" vs "lift up"
- **D1. Space & Rhythm** ([`cem-dimension.md`](./cem-dimension.md)) — breathing room coupling
- **D3. Shape — Bend** ([`cem-shape.md`](./cem-shape.md)) — bend roles for overlays/modals
- **D5. Stroke & Separation** ([`cem-stroke.md`](./cem-stroke.md)) — contour reinforcement, especially in forced-colors
- **D7. Time & Motion** ([`cem-timing.md`](./cem-timing.md)) — lift/drop transitions and reduced-motion behavior
- **Action states** ([discussion #14](https://github.com/EPA-WG/custom-element-dist/discussions/14)) — visual priority order and interaction escalation semantics

## Contents

1. [What layering means to users](#1-what-layering-means-to-users)
2. [Consumer layering vocabulary](#2-consumer-layering-vocabulary)
3. [User-flow journey through layering](#3-user-flow-journey-through-layering)
4. [Design goals](#4-design-goals)
5. [Signed ladder and planes](#5-signed-ladder-and-planes)
6. [Token model](#6-token-model-basis--endpoints--adapters)
7. [Basis tokens and appearance-change principle](#7-basis-tokens-the-canonical-signed-ladder)
8. [Semantic endpoints contract](#8-semantic-endpoints-product-facing-contract)
9. [Representation channels](#9-representation-channels)
10. [Scrim](#10-scrim)
11. [Action states and the color ladder](#11-action-states-must-align-with-layering-and-the-color-ladder)
12. [Cross-dimension coupling](#12-cross-dimension-coupling)
13. [Accessibility and forced-colors requirements](#13-accessibility-and-forced-colors-requirements)
14. [Component mapping matrix](#14-component-mapping-matrix)
15. [Implementation guidance](#15-implementation-guidance)
16. [Canonical token summary](#16-canonical-token-summary)
17. [Governance and versioning](#17-governance-and-versioning)
18. [Migration notes](#18-migration-notes)
19. [References](#19-references)

---

## 1. What layering means to users

Layering communicates **relationship and intent** before interaction:

- **Where am I?** (shell vs work)
- **What belongs together?** (grouping vs separation)
- **What can I act on?** (affordance prominence)
- **What requires my decision?** (modal commitment)

In consumer language, layering typically reads as:

- **Back layer:** context and scaffolding (navigation, filters).
- **Work layer:** primary reading/doing.
- **Interruption/command:** transient overlays and modal decisions.

Layering exists to support three outcomes:

1) **Orientation** — know what is “infrastructure” vs “work.”
2) **Separability** — see boundaries without reading.
3) **Commitment clarity** — distinguish “context” from “decision.”

---

## 2. Consumer layering vocabulary

### 2.1 Common terms

- **Layer / plane:** a named region with stable intent (back, work, overlay).
- **Stack / z-order:** physical draw order.
- **Recessed / inset:** visually “dug down” behind the base.
- **Raised / floating:** visually “lifted” above the base.
- **Overlay / popover:** transient contextual layer above other content.
- **Command / modal:** must-respond decision layer.

### 2.2 Semantic tiers are not a z-index ladder

The tier number communicates **priority and meaning** (user psychology), not guaranteed draw order. Physical stacking may differ because of clipping, portals, and platform APIs. That physical mismatch **does not** change the semantic contract.

**Normative rules**

- **MUST NOT** use `--cem-elevation-*` as a z-index value.
- **MUST NOT** infer semantic tier from physical draw order.
- **MUST** preserve semantic meaning even when physical stacking differs.
- **MUST** treat **Command** as the owner of modality (scrim, focus trap, blocked interaction), even if an Overlay renders above it.
- **MUST** apply dismissal precedence: dismiss the topmost Overlay first, then the Command layer (if present).

**Discriminator rules: Overlay vs Floating**

Use these rules when physical appearance could be misread (notably when overlays appear over dialogs).

- Choose **Overlay** when the UI is a **separate transient layer artifact** with its own dismissal semantics (appears/disappears without becoming part of layout).  
  Typical: menus, popovers, autocomplete lists, date pickers, **tooltips**.

- Choose **Floating** when the UI is the **same object** as in the work context, temporarily lifted for affordance while remaining part of the current task.  
  Typical: hover lift, “picked-up” cards, **dragged items**.

**Examples (required interpretation)**

- **Tooltip over a dialog:** Tooltip remains **Overlay** (contextual helper). Physical topness does not outrank the dialog’s “must respond” semantics; it dismisses first.
- **Dragging while a dialog is present:** The dragged object remains **Floating** (same work object lifted). The drag ghost may portal above for clipping, but it **MUST NOT** be reclassified as Overlay.

### 2.3 7-tier canonical ladder

CEM defines exactly **7 canonical tiers**. The ladder is signed: two recess rungs, one base rung, and four lift rungs.

> The ladder is designed to be perceptually distinct and implementable across light/dark/contrast themes.

| Signed tier   | Token               | Canonical name    | Intended meaning                    | Typical examples                        |
|--------------:|---------------------|-------------------|-------------------------------------|-----------------------------------------|
|        **−2** | `--cem-recess-2`    | **Deep recessed** | infrastructure far behind attention | deep shell wells, backstage panels      |
|        **−1** | `--cem-recess-1`    | **Recessed**      | back context behind work            | side navigation rail, filter plane      |
|         **0** | `--cem-elevation-0` | **Base**          | ground / canvas                     | app background, page canvas             |
|        **+1** | `--cem-elevation-1` | **Raised**        | work region separation              | cards, primary panels, editors          |
|        **+2** | `--cem-elevation-2` | **Floating**      | interactive lift / grouped focus    | hover-lift regions, dragged items       |
|        **+3** | `--cem-elevation-3` | **Overlay**       | contextual transient layer          | menus, tooltips, popovers, select lists |
|        **+4** | `--cem-elevation-4` | **Command**       | must respond / modal decision       | dialogs, blocking confirmations         |

### 2.4 Terminology disclaimer

1) **One consumer term:** “layering” is the shared vocabulary for elevation and recess.
2) **Signed meaning:** both “dig down” and “lift up” are first-class.
3) **Multiple channels:** tone, contour, shadow, blur/material, space, and motion are coordinated.
4) **Theme resilience:** the ladder works in light/dark/forced-colors.
5) **Avoid false importance:** overlays can render above commands physically, but they must not steal semantic priority.

---

## 3. User-flow journey through layering

Layering is experienced as the user moves through a interaction flow.

### 3.1 Landing (orient)

- The user identifies **Back vs Work** immediately.
- Recess is used to make scaffolding feel “behind.”
- Back layer gives orientation without competing for attention.
- Base establishes the canvas and reading comfort.

```
[ Back layer ]   |  [ Work layer ]
(nav/context)    |  (primary content)
```

### 3.2 Scanning (compare)

- The user needs separability at speed.
- Work layer regions should be visually grouped (Raised).
- Recess reduces noise in scaffolding regions.

### 3.3 Interacting (act)

- Floating lift signals “this is the active object.”
- Focus/hover are reinforced without changing the underlying layer role.

### 3.4 Contextualizing (choose)

- Overlay provides contextual options near the trigger, above the work.
- Overlay must remain dismissible and non-modal.
- If an overlay blocks reading or causes ambiguity, it has become a Command.

### 3.5 Deciding (commit)

- Command is the highest semantic priority.
- Command forces a decision with scrim and focus trap.
- Any Overlay displayed above a Command remains subordinate and dismisses first.

---

## 4. Design goals

- **Semantic clarity:** layers communicate intent (shell vs work vs command).
- **Minimal ladder:** seven tiers only; no micro-tiers as defaults.
- **Theme resilience:** semantics stay constant across light/dark/contrast and density.
- **Accessibility:** layering must remain readable in forced-colors (contour substitutes for shadow/tone).
- **Portability:** the model maps to web, native, and component libraries.

---

## 5. Signed ladder and planes

Layering is a composition of:

- **Semantic tier** (the signed ladder rung)
- **Plane** (the global rendering order / portal)

### 5.1 Planes

CEM defines five named planes (semantic layer roles). They are typically implemented with portals.

- **Back plane** — recessed context.
- **Base plane** — canvas.
- **Work plane** — primary doing/reading.
- **Overlay plane** — contextual transient UI.
- **Command plane** — modal decisions.

**Normative rules**

- Overlays and commands **MUST** render in global planes to avoid clipping by scroll containers.
- A component’s **semantic tier** is independent of its **rendering plane**.

### 5.2 Physical stacking exceptions are allowed

A tooltip can render above a dialog while remaining an **Overlay-tier** element.

This is not a contradiction; it reflects two truths:

- **Physical topness** is about rendering constraints (portals, clipping, platform APIs).
- **Semantic topness** is about attention and commitment (what the user must respond to).

---

## 6. Token model (basis → endpoints → adapters)

CEM D4 defines three layers of contract.

### 6.1 Basis tokens (rungs)

Basis tokens are the minimal numeric “rungs.” They encode signed tier meaning.

- `--cem-recess-2`
- `--cem-recess-1`
- `--cem-elevation-0`
- `--cem-elevation-1`
- `--cem-elevation-2`
- `--cem-elevation-3`
- `--cem-elevation-4`

### 6.2 Semantic endpoints (product-facing)

Semantic endpoints are what components use. They map to basis rungs.

- `--cem-layer-back`
- `--cem-layer-base`
- `--cem-layer-work`
- `--cem-layer-overlay`
- `--cem-layer-command`

### 6.3 Adapters (appearance channels)

Adapters describe how the rung is rendered:

- tonal shift (color ladder)
- shadow recipe
- contour/stroke reinforcement
- material/blur recipe
- spacing adjustments
- motion/transition recipe

Adapters can vary per theme/density without changing semantics.

---

## 7. Basis tokens: the canonical signed ladder

### 7.1 Required rung tokens

The 7-tier ladder rungs are required and must remain stable.

### 7.2 Appearance-change principle

When moving by one rung, **at least one** of the following must change in a perceivable way (and ideally two in high-density UI):

- **Tone** (surface/container tone, chroma shift)
- **Contour** (stroke/outline, especially in forced-colors)
- **Shadow** (soft separation)
- **Material** (blur/opacity)
- **Space** (inset/gap or breathing room)
- **Motion** (lift/drop transition)

Recess should primarily use **tone and contour** (shadows are often imperceptible “into” the page).

Lift should primarily use **tone + shadow**, with contour as a fallback.

---

## 8. Semantic endpoints: product-facing contract

Semantic endpoints allow components to express layering without knowing numeric rungs.

### 8.1 Canonical endpoints

| Endpoint              | Maps to rung        | Meaning                         |
|-----------------------|---------------------|---------------------------------|
| `--cem-layer-back`    | `--cem-recess-1`    | back layer: context behind work |
| `--cem-layer-base`    | `--cem-elevation-0` | canvas/base                     |
| `--cem-layer-work`    | `--cem-elevation-1` | primary work regions            |
| `--cem-layer-overlay` | `--cem-elevation-3` | contextual transient UI         |
| `--cem-layer-command` | `--cem-elevation-4` | must respond: modal decision    |

> Note: `--cem-elevation-2` (Floating) is most often an **internal component choice** within the Work layer (e.g., a temporary lifted region), not a global plane.

### 8.2 Optional endpoints

Optional endpoints are allowed only if they remain stable and scoped, 
and if they map cleanly to an existing rung without creating new “micro tiers”.

Examples:

- `--cem-layer-back-deep` → `--cem-layer-back` + `--cem-recess-2`
- `--cem-layer-work-floating` → `--cem-layer-work` + `--cem-elevation-2`

---

## 9. Representation channels

Layering cues are multi-channel. No single channel is reliable across all themes.

### 9.1 Tone and the color ladder

The CEM color ladder is the primary driver of “dig down” vs “lift up.”

**Rules**

- Recess **SHOULD** move toward neutral/gray (less chroma/brand energy).
- Lift **SHOULD** move toward brighter/branded tones (more chroma/energy).
- Tone changes must preserve text contrast; do not “flip” text color unless the rung truly becomes Overlay/Command.

### 9.2 Shadow

- Shadows **MAY** be reduced in dark themes; tonal separation must still carry the meaning.
- Recess relies less on shadow; “negative shadow” is generally not readable.
- Shadow should never be the only separator.

### 9.3 Contour / stroke

- Back and Base **SHOULD** rely on contour in high-density layouts and forced-colors.
- Back and Base layers **SHOULD** have minimal contour.
- Work and above **SHOULD** provide optional contour reinforcement for dense layouts.
- Overlay and Command **MUST** remain legible if shadows are removed.

### 9.4 Material / blur

Where supported, material/blur can reinforce overlay separation, but **MUST NOT** replace tone/contrast requirements.

### 9.5 Space (inset and gap)

Layers with higher semantic priority **SHOULD** “earn” breathing room (inset and/or gap) in comfort density modes.

- Recess can be communicated by reduced padding/visual weight, but never by reducing hit targets.

### 9.6 Motion

Lift/drop transitions **MAY** reinforce the ladder, but the meaning must remain clear in reduced-motion settings.

- Lift/drop transitions are allowed for state changes (hover, focus) but must not change semantic tier.

---

## 10. Scrim

Scrim communicates modality and commitment.

### 10.1 Scrim usage matrix

| Layer role         | Typical components              | Scrim           | Notes                                                       |
|--------------------|---------------------------------|-----------------|-------------------------------------------------------------|
| Back / Base / Work | nav, content                    | none            | scrim would reduce usability                                |
| Overlay            | menus, popovers, tooltips       | optional (rare) | only if overlay must strongly detach from busy content      |
| Command            | dialogs, blocking confirmations | **required**    | command owns modality, focus trap, and dismissal precedence |

### 10.2 Command owns modality

- Command **MUST** trap focus (or equivalent modality) and own the scrim.
- Overlay **MUST NOT** break modality; it may appear above physically, but remains subordinate in semantics and dismissal precedence.
- Dismissal precedence: **dismiss the topmost Overlay first, then Command**.

---

## 11. Action states must align with layering and the color ladder

### 11.1 Alignment rule: one direction, two dimensions

Layering (signed depth) and action states (visual priority order) must not contradict each other.

- “More urgent / more interactive” states **SHOULD NOT** be rendered with recess cues.
- “Less available / less interactive” states **SHOULD NOT** be rendered with lift cues.

### 11.2 State escalation should not change layer role

State changes (hover, focus, pressed, selected, disabled, readonly) **MUST NOT** change the layer endpoint. They may adjust **rung rendering** (tone/contour/shadow) within a bounded range.

Action states must align with layering cues and color ladder direction.

A button becoming hovered does not become an Overlay. State changes should be expressed as **micro-shifts** within the component’s assigned layer role.


### 11.3 Recommended state → signed shift mapping (default guidance)

This guidance assumes the component’s base endpoint is correct.

- **Default:** no signed shift change.
- **Hover / focus:** +0 → +1 *appearance reinforcement* (often contour and/or slightly increased tonal separation).
- **Pressed:** may “press toward base” visually *without* reducing salience (tactile cue, not hierarchy reversal).
- **Selected:** treat as a *meaning* change (often tone), not necessarily as a rung change.
- **Readonly / disabled:** allow a **recess cue** (typically −1 via tone/contour) without moving planes.

### 11.4 Active/pressed tactile rule

Pressed feedback may reduce shadow (a “press in”), but **MUST** preserve the state’s prominence via tone/contour so the user does not read it as “disabled or background.”

---

## 12. Cross-dimension coupling

Layering relies on other dimensions for robustness:

- **D0 Color ladder:** supplies tonal recipes for recess and lift.
- **D5 Stroke:** provides contour when tone/shadow are insufficient or unavailable.
- **D1 Space:** higher-priority layers may gain breathing room in comfort density.
- **D7 Motion:** transitions can reinforce, never replace, the ladder.

Layering **MUST NOT** rename or duplicate these dimensions; it only defines coupling invariants.

---

## 13. Accessibility and forced-colors requirements

### 13.1 Depth is not the only signal

Layer meaning must remain readable without shadows and with limited color (e.g., high-contrast themes). Therefore:

- At least one non-shadow signal (tone or contour) **MUST** remain.
- Focus indicators **MUST** remain visible across all tiers.

### 13.2 Forced-colors

In forced-colors:

- Tier meaning **SHOULD** be expressed primarily via contour and spacing.
- Do not rely on subtle tonal deltas.

---

## 14. Component mapping matrix

This matrix maps common component families to **layer endpoints** and their **default rung**.

| Component family                   | Layer endpoint        | Default rung        | Notes                                              |
|------------------------------------|-----------------------|---------------------|----------------------------------------------------|
| App shell (chrome)                 | `--cem-layer-back`    | `--cem-recess-1`    | recessed context; low brand energy                 |
| Backstage wells (rare)             | `--cem-layer-back`    | `--cem-recess-2`    | use sparingly; avoid turning the UI into “tunnels” |
| Page canvas                        | `--cem-layer-base`    | `--cem-elevation-0` | ground / canvas                                    |
| Main content panels                | `--cem-layer-work`    | `--cem-elevation-1` | default work separation                            |
| Floating affordances (within work) | `--cem-layer-work`    | `--cem-elevation-2` | same object lifted (hover/drag)                    |
| Tooltip                            | `--cem-layer-overlay` | `--cem-elevation-3` | may portal above dialogs; remains Overlay          |
| Menu / popover / select list       | `--cem-layer-overlay` | `--cem-elevation-3` | portal/top-layer; dismiss precedence above dialog  |
| Sheet (non-blocking)               | `--cem-layer-overlay` | `--cem-elevation-3` | optional scrim only if required                    |
| Dialog / confirmation              | `--cem-layer-command` | `--cem-elevation-4` | owns modality, scrim, focus trap                   |
| Toast / global banner              | `--cem-layer-overlay` | `--cem-elevation-3` | if it blocks progress, treat as Command            |

---

## 15. Implementation guidance

### 15.1 Keep semantics separate from z-index

- Use layer endpoints and rung tokens for **meaning**.
- Use a dedicated z-index policy (framework-specific) for **stacking mechanics**.

### 15.2 Example CSS pattern (illustrative)

The following demonstrates the mapping shape; it is not a required implementation.

```css
/* Semantic endpoints select a rung; adapters render the rung. */

.cem-layer--work      { --cem-rung: var(--cem-elevation-1); }
.cem-layer--overlay   { --cem-rung: var(--cem-elevation-3); }
.cem-layer--command   { --cem-rung: var(--cem-elevation-4); }

/* Adapter hooks (tone/shadow/contour) are theme-specific. */
.cem-layer {
  background: var(--cem-layer-tone, Canvas);
  box-shadow: var(--cem-layer-shadow, none);
  outline: var(--cem-layer-contour, none);
}
```

### 15.3 Dismissal and focus precedence

- Escape closes the topmost Overlay first.
- If no overlays remain, escape closes the Command (if allowed).

---

## 16. Canonical token summary

### 16.1 Required rung tokens

- `--cem-recess-2`
- `--cem-recess-1`
- `--cem-elevation-0`
- `--cem-elevation-1`
- `--cem-elevation-2`
- `--cem-elevation-3`
- `--cem-elevation-4`

### 16.2 Required semantic endpoints

- `--cem-layer-back`
- `--cem-layer-base`
- `--cem-layer-work`
- `--cem-layer-overlay`
- `--cem-layer-command`

### 16.3 Optional semantic endpoints (allowed)

- `--cem-layer-back-deep` → `--cem-layer-back` + `--cem-recess-2`
- `--cem-layer-work-floating` → `--cem-layer-work` + `--cem-elevation-2`

---

## 17. Governance and versioning

### 17.1 Breaking (major)

Treat as breaking if you:

- Rename or remove any canonical rung token or endpoint.
- Change the semantic meaning of a rung or endpoint.
- Change the meaning of “Command owns modality” or dismissal precedence.

### 17.2 Non-breaking (minor/patch)

Treat as minor/patch if you:

- Adjust adapter recipes (tone/shadow/contour) per theme/density while preserving semantics.
- Add optional endpoints with stable consumer meaning (and clear mapping to existing rungs).
- Clarify documentation or add mapping guidance.

---

## 18. Migration notes

This spec is the canonical D4 contract for layering (signed depth).

- Legacy elevation tokens may be kept as **internal aliases**.
- Prefer `--cem-elevation-*`, `--cem-recess-*`, and `--cem-layer-*` as the public CEM contract going forward.

---

## 19. References

### Internal

- Action states in visual priority order (EPA-WG `custom-element-dist` discussion #14): https://github.com/EPA-WG/custom-element-dist/discussions/14

### External

- Material Design Backdrop (back layer / front layer): https://m2.material.io/components/backdrop
- Material Design 3 — Elevation: https://m3.material.io/styles/elevation
- Android Developers — Material 3 elevation guidance (tonal + shadow): https://developer.android.com/develop/ui/compose/designsystems/material3#elevation
- Fluent 2 Design System — Elevation: https://fluent2.microsoft.design/elevation
- Windows apps — Depth, z-depth, and shadow: https://learn.microsoft.com/en-us/windows/apps/design/layout/depth-shadow
- Apple Human Interface Guidelines — Materials: https://developer.apple.com/design/human-interface-guidelines/materials
- Apple Human Interface Guidelines — Popovers: https://developer.apple.com/design/human-interface-guidelines/popovers
- Atlassian Design System — Elevation (includes z-index guidance): https://atlassian.design/foundations/elevation
- Atlassian Design System — Portal (stacking context): https://atlassian.design/components/portal
