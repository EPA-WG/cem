# Semantic Timing and Motion Tokens (CEM) — Canonical Spec

**Status:** Canonical (v1.0)

**Last updated:** December 21, 2025

**Audience:** Design Systems, Product Design, Front-End Engineering

**Applies to:** UI animation timing (durations, easing curves, and optional spring presets)

**Companion specs:**
- **D0. Color (Emotional Palette)** ([`cem-colors.md`](./cem-colors.md)) — color transition timing
- **D1. Space & Rhythm** ([`cem-dimension.md`](./cem-dimension.md)) — spacing animation timing
- **D2. Coupling & Compactness** ([`cem-coupling.md`](./cem-coupling.md)) — density transition timing
- **D3. Shape — Bend** ([`cem-shape.md`](./cem-shape.md)) — bend/morph animation timing
- **D4. Layering** ([`cem-layering.md`](./cem-layering.md)) — lift/drop transitions, reduced-motion behavior
- **D5. Stroke & Separation** ([`cem-stroke.md`](./cem-stroke.md)) — focus/selection transition timing

---

## 1. Purpose and scope

This spec defines the **CEM contract surface** for motion timing:

- **Duration** tokens (how long a change takes)
- **Easing** tokens (how the motion accelerates/decelerates)
- **Spring** presets (optional; for platforms that support spring physics)

The goal is to make motion choices **intent-driven** (what the user experiences), while allowing implementations/adapters
(e.g., Material, custom, platform-native) to supply exact numeric curves/physics.

---

## 2. Taxonomy placement

- **D7. Timing & Motion** (this spec)
  - time durations
  - easing curves
  - spring presets (optional)

Related dimensions:

- **D4. Layering & Elevation**: overlays/dialogs often use longer durations
- **D2. Coupling & Compactness**: compact density should not force overly fast or illegible motion
- **D5. Stroke & Separation**: focus/selection indicators may animate; duration must not impede operability

---

## 3. Consumer vocabulary

CEM duration names reflect how users perceive timing:

- **instant** — “blink; do not interrupt”
- **noticeable** — “registerable, but not slow”
- **lingering** — “large change; allow the user to track it”

Easing names reflect intent:

- **smooth** — everyday, unobtrusive
- **highlighted** — draws attention (more pronounced)
- **uniform** — neutral/mechanical (constant speed)
- **classic** — legacy compatibility

---

## 4. Canonical token surface

### 4.1 Duration tokens (required)

```css
:root {
  /* Core buckets (contract-level) */
  --cem-duration-instant:    50ms;
  --cem-duration-noticeable: 250ms;
  --cem-duration-lingering:  300ms;

  /* Optional semantic aliases (adapter may override) */
  --cem-duration-action: var(--cem-duration-noticeable);
  --cem-duration-overlay: var(--cem-duration-lingering);
}
```

Normative rules:

- Ordering MUST remain: `instant < noticeable < lingering`.
- Reduced-motion modes may shorten durations (including to `0ms`), but MUST preserve relative ordering.

### 4.2 Easing tokens (required)

Easing tokens are expressed as CSS timing functions.

```css
:root {
  /* Everyday motion */
  --cem-easing-smooth:        ease-in-out;
  --cem-easing-start-smooth:  ease-in;
  --cem-easing-end-smooth:    ease-out;

  /* Attention-drawing motion (adapter SHOULD provide a more pronounced curve) */
  --cem-easing-highlighted:        var(--cem-easing-smooth);
  --cem-easing-highlighted-start:  var(--cem-easing-start-smooth);
  --cem-easing-highlighted-end:    var(--cem-easing-end-smooth);

  /* Neutral/legacy */
  --cem-easing-uniform: linear;
  --cem-easing-classic: ease;
}
```

Normative rules:

- `smooth` MUST be suitable as the default.
- `highlighted*` MUST be *visibly more pronounced* than the corresponding `smooth*` curves **in the same implementation**.
  (The above defaults provide placeholders; adapters are expected to supply real emphasized curves.)

### 4.3 Spring tokens (optional extension)

Spring tokens are **optional** because spring rendering is implementation-specific (WAAPI polyfills, native toolkits,
framework animation engines, etc.).

If springs are supported, reserve the following semantic naming grid:

- **Focus**: `reposition | highlight`
- **Feel**: `functional | delight`
- **Speed**: `instant | noticeable | lingering`

Recommended token shape (implementation-defined value encoding):

```css
:root {
  /* Example encoding as a string. Implementations may choose a different encoding. */
  --cem-spring-reposition-functional-instant:  "stiffness=… damping=…";
  --cem-spring-reposition-functional-noticeable: "stiffness=… damping=…";
  --cem-spring-reposition-functional-lingering:  "stiffness=… damping=…";

  --cem-spring-highlight-delight-lingering: "stiffness=… damping=…";
}
```

Normative rules (if springs are implemented):

- `instant | noticeable | lingering` MUST preserve perceived ordering from fastest to slowest.
- `delight` MAY overshoot/bounce; `functional` SHOULD minimize overshoot.

---

## 5. Usage guidance

- Use **instant** for micro-interactions that must not interrupt flow (e.g., tiny state toggles).
- Use **noticeable** for most component transitions (expand/collapse, standard page UI changes).
- Use **lingering** for overlays and large spatial changes where users must track context.

Easing selection:

- Use **smooth** for default transitions.
- Use **start-smooth** for entrances; **end-smooth** for dismissals.
- Use **highlighted** variants when motion’s purpose is to direct attention.
- Use **uniform** for mechanical/neutral motion (e.g., indeterminate progress).
- Use **classic** only for legacy compatibility.

---

## 6. Governance and versioning

Treat D7 tokens as **contract-level** once canonical.

Breaking (MAJOR):

- Renaming/removing a required token
- Changing the semantic meaning of a token
- Violating the ordering rule (`instant < noticeable < lingering`)

Non-breaking (MINOR/PATCH):

- Adjusting numeric values while preserving semantics and ordering
- Adding optional aliases or additional optional spring presets
- Documentation clarifications

---

## 7. Canonical token summary (contract surface)

| Token | Category | Required |
|---|---|---|
| `--cem-duration-instant` | Duration | Yes |
| `--cem-duration-noticeable` | Duration | Yes |
| `--cem-duration-lingering` | Duration | Yes |
| `--cem-easing-smooth` | Easing | Yes |
| `--cem-easing-start-smooth` | Easing | Yes |
| `--cem-easing-end-smooth` | Easing | Yes |
| `--cem-easing-highlighted` | Easing | Yes |
| `--cem-easing-highlighted-start` | Easing | Yes |
| `--cem-easing-highlighted-end` | Easing | Yes |
| `--cem-easing-uniform` | Easing | Yes |
| `--cem-easing-classic` | Easing | Yes |
| `--cem-duration-action` | Duration alias | No |
| `--cem-duration-overlay` | Duration alias | No |
| `--cem-spring-*` | Spring presets | No |
