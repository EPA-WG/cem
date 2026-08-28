# CEM Light-DOM CSS Scope Contract

**Status:** Normative accepted target; implementation pending.
**Primary use case:** declaration-owned CSS for no-shadow-DOM CEM components.
**Migration authority:** [`docs/todo.md`](./todo.md), under **Native CSS `@scope` migration**.

This document defines the CSS ownership, scope, projection, specificity, and
diagnostic contract for `cem-elements`, `cem-components`, CEM Studio, and CEM
Site. The words **MUST**, **MUST NOT**, **SHOULD**, and **SHOULD NOT** are
normative. The [declarative UI principle](./declarative-ui-principle.md) defines
where UI is authored; this document defines how that UI enters the light-DOM
cascade.

The target requires native CSS `@scope`, which is a
[Baseline 2026 feature](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@scope).
There is no compatibility rewrite for browsers without `@scope`.

> **Migration status:** the current runtime implements the completed legacy
> `:where(...)`, `data-cem-scope`, and `data-cem-instance-scope` baseline. Those
> selectors are migration evidence, not the target authoring API. Components
> MUST NOT depend on this target until the migration checklist and its runtime
> gates pass. Only then may this status change to “accepted and implemented.”

## 1. Principles

CEM components render into the light DOM. Their rules therefore participate in
the ordinary document cascade: CEM does not emulate a shadow root and does not
promise impenetrable CSS.

The contract provides three levels of ownership:

| CSS kind | Authored source | Public root | Lifetime |
| --- | --- | --- | --- |
| private declaration CSS | static bare `<style>` in a component declaration | produced custom-element tag | one managed artifact per effective declaration |
| named shared CSS | declaration/style pair with one matching `scope` name | `scope="<name>"` on produced DCE hosts | one managed artifact per effective declaration |
| instance CSS | `<style>` inside an explicit inert instance payload template | the style element's parent DCE | one managed artifact per instance |

The generated render identity `data-cem-render-scope` remains internal to
rendering, hydration, resources, and diagnostics. It is never a CSS hook.
`template[data-cem-island="instance"]` remains the internal inert-data boundary
and nested-host marker. Neither marker is a public styling interface.

UUIDs provide stable identity and collision resistance. They do not provide CSS
encapsulation or a security boundary.

## 2. Declaration-Owned CSS

Component CSS MUST be a static CEM-ML style in the component's XHTML
declaration:

    <cem-element tag="cem-select">
      <template id="cem-select" type="text/cem-ml">
        {module |
          {body |
            {style @type="text/css" | ```
              :host {
                display: inline-block;
              }

              [part~="control"] {
                inline-size: 100%;
              }
            ```}
            ...
          }
        }
      </template>
    </cem-element>

`cem-elements` statically extracts each declaration style, removes it from the
instance render plan, and installs it once beside the effective declaration.
Removing the declaration removes its managed styles. A component MUST NOT clone
static CSS into instances, inject CSS from component JavaScript, add selectors
to package-global CSS, or own a standalone `<cem-tag>.css` file.

### 2.1 Private compilation

For a declaration producing `cem-select`, private CSS compiles to:

```css
@scope (cem-select) to (
  :scope :has(> template[data-cem-island="instance"]) > *,
  [slot] > *
) {
  :where(:scope) {
    /* authored :host */
  }

  [part~="control"] {
    /* component-owned internals */
  }

  [slot="label"] {
    /* documented projected root */
  }
}
```

Compilation MUST apply these rules:

- `:host` becomes `:where(:scope)`.
- `:host(...)` becomes `:where(:scope)...`.
- The compiler MUST NOT translate `:host` to `&`; nesting and scoped selectors
  have different specificity behavior.
- The generated scope root contributes no selector specificity to contained
  rules. Authored selectors keep their ordinary specificity.
- The first lower limit excludes the children and internals of descendant DCE
  hosts identified by their direct instance data island. The nested host itself
  remains styleable by its parent component.
- The second lower limit leaves a projected root styleable but excludes its
  consumer-owned descendants.
- Scope limits do not block inherited properties or custom properties.

The limits intentionally start with a descendant combinator. The current scope
root is therefore not mistaken for a nested DCE merely because it owns its own
instance data island.

### 2.2 Static artifacts and contained at-rules

Declaration stylesheet content and its `scope` attribute MUST be statically
extractable. Runtime expressions, conditionals, or loops that generate
declaration styles diagnose as
`cem.ql.template.stylesheet_dynamic_unsupported` and produce no fallback style.

Declaration-local keyframes receive a deterministic stylesheet suffix and all
local animation references are rewritten. `@import` and document-global
constructs such as `@font-face`, `@property`, `@counter-style`,
`@font-palette-values`, `@page`, and `@namespace` are suppressed with the
diagnostics in section 8. Authored outer `@scope` is also unsupported because
the runtime owns the component boundary.

Dynamic visual state MUST use static selectors over host, state, and ARIA
attributes plus CEM custom properties.

## 3. Named Shared Scope

A declaration may opt into one public shared scope:

```html
<cem-element tag="cem-select" scope="form-controls">
  <template id="cem-select" type="text/cem-ml">...</template>
</cem-element>
```

Produced hosts reflect the declaration-owned name:

```html
<cem-select scope="form-controls">...</cem-select>
```

An explicit shared style uses the same static name:

    {style @scope="form-controls" @type="text/css" | ```
      :host {
        font-family: var(--cem-typography-ui-font-family);
      }
    ```}

Shared rules compile with a CEM-instance-qualified root and the same lower
limits as private rules:

```css
@scope (
  [scope="form-controls"]:has(> template[data-cem-island="instance"])
) to (
  :scope :has(> template[data-cem-island="instance"]) > *,
  [slot] > *
) {
  :where(:scope) {
    font-family: var(--cem-typography-ui-font-family);
  }
}
```

The `:has(...)` qualification prevents the public name from capturing unrelated
HTML such as `<th scope="row">`. It does not make the name private.

### 3.1 Resolution matrix

The declaration/style resolution matrix remains:

| Declaration | Styles present | Bare `<style>` | `<style scope="form-controls">` |
| --- | --- | --- | --- |
| no `scope` | bare only | private tag scope | invalid |
| `scope="form-controls"` | bare only | shared named scope | — |
| `scope="form-controls"` | scoped only | — | shared named scope |
| `scope="form-controls"` | both kinds | private tag scope | shared named scope |

The bare-only row is a shorthand. Once a valid matching explicit scoped style
exists, bare styles remain private and matching explicit styles are shared. An
invalid or mismatched scoped style is rejected and does not alter valid bare
style resolution.

### 3.2 Name and ownership rules

- A declaration accepts exactly one non-empty static CSS identifier as `scope`.
  Whitespace-separated, dynamic, or invalid values fail closed.
- An explicit style scope MUST exactly match the declaration scope.
- The declaration owns the value reflected to each produced host. Adding a value
  to an unscoped instance, or changing/removing a declared value, diagnoses and
  restores the declaration state.
- Multiple declarations MAY deliberately contribute to the same public name;
  ordinary cascade order applies between their contributions.
- Consumers MAY deliberately select `[scope="form-controls"]`. A scope name is
  a documented public styling surface, not insulation from a determined author.
- Libraries SHOULD use package-qualified names when a generic name could collide.

Using `scope` keeps the API short and makes shared membership visible in markup.
Its costs are equally deliberate: the name is public, inheritance crosses the
boundary, independent libraries can choose the same name, and HTML already uses
`scope` semantically on `th`. The DCE qualification also relies on `:has()`, so
it shares the modern-browser requirement and selector-matching cost of that
pseudo-class. CEM mitigates those costs with declaration ownership, identifier
validation, package naming guidance, direct-child `:has()` tests, and
DCE-qualified generated roots—not with obscurity.

## 4. Projection and Stable Styling Hooks

### 4.1 Projected roots

`slot="name"` is the public projected-root marker. A named projection preserves
or stamps its documented name on each projected element root:

```html
<span slot="label">Account</span>
```

Default projected element roots use `slot=""`. The marker remains on the
projected root after CEM projection; no literal `<slot>` wrapper survives.

- `[slot="label"]` may style the documented projected root.
- `[slot] > *` is a scope lower limit, so the component does not style deeper
  consumer-owned structure.
- Text roots cannot carry attributes and inherit from the component-owned
  insertion container.
- Component fallback content is component-owned and MUST NOT receive a
  projection marker.
- A projection site that must expose a styleable public root MUST have a stable
  CEM-ML slot name.

### 4.2 Component-owned internals

`part="name"` is the stable hook for component-generated internals:

```html
<button part="control">...</button>
```

Component and consumer CSS select it with ordinary light-DOM selectors such as
`[part~="control"]` or `cem-select [part~="control"]`. CEM MUST NOT describe
this as native `::part()`, whose semantics belong to shadow trees.

Projection MUST NOT stamp `part` onto consumer-owned payload. A component MAY
place a documented part on its own insertion container.

## 5. Instance CSS

Per-instance CSS requires an explicit inert payload envelope:

```html
<cem-select>
  <template>
    <style>
      :host {
        inline-size: 20rem;
      }

      [part~="control"] {
        min-block-size: 3rem;
      }
    </style>

    <span slot="label">Account</span>
  </template>
</cem-select>
```

A bare `<style>` directly inside an uninitialized DCE is forbidden because the
browser applies it globally before CEM can establish a boundary. The direct
`template` is reserved as the explicit instance payload envelope, so its source
is inert before upgrade. The runtime adopts it as the instance data island
instead of creating another wrapper.

- Mixing the direct payload template with non-whitespace siblings diagnoses and
  fails closed.
- A literal template intended as projected content MUST be nested inside the
  outer payload template.
- Instance source styles remain inert in the data island.
- The runtime creates one active managed style per source style directly under
  the produced host. No `data-cem-instance-style` marker is required.

An inline stylesheet with an omitted `@scope` prelude uses its parent element as
the scope root. The managed instance style therefore compiles to:

```css
@scope to (
  :scope :has(> template[data-cem-island="instance"]) > *,
  [slot] > *
) {
  :scope {
    /* authored :host */
  }

  :scope [part~="control"] {
    /* authored instance rule */
  }
}
```

For instance CSS, `:host` becomes `:scope`, `:host(...)` becomes `:scope...`,
and every other authored selector receives a `:scope` descendant prefix. That
prefix adds one class-level specificity component, so an otherwise equivalent
instance rule overrides a declaration rule without a generated UUID selector.

`data-cem-instance-scope` is removed and has no replacement. Instance isolation
comes from the native implicit scope root and the existing data-island lower
limit.

## 6. Specificity and Cascade

CEM follows the native cascade, including
[scoping proximity](https://www.w3.org/TR/css-cascade-6/#scope-atrule), after
origin, importance, encapsulation context, style attributes, cascade layers, and
specificity are considered.

- A generic page rule and an equally specific component rule are resolved by
  scope proximity inside the component. For example, a declaration-owned
  `button` rule beats an otherwise equal page-level `button` rule.
- A consumer can intentionally override a component with a more specific public
  selector such as `cem-select [part~="control"]`.
- Declaration and shared-scope root selectors contribute no specificity.
- The generated instance `:scope` prefix intentionally contributes `0-1-0`.
- Authored declaration and shared selectors MUST NOT exceed `0-2-1` before any
  generated instance prefix.
- IDs, selector duplication used to manufacture specificity, inline
  presentation styles, and `!important` in library rules are forbidden.
- CEM generates no cascade layer. Applications remain free to organize their
  own non-CEM styles into layers.
- Custom properties and inherited properties are the intended cross-boundary
  theming mechanism.

Scope proximity does not override a selector with greater specificity and does
not create complete insulation. Consumers who know a public tag, `scope`,
`slot`, or `part` hook can style it declaratively.

## 7. Render Identity, Hydration, and Anonymous Declarations

`uid-seed` continues to control stable internal identities. Resolution order is:

1. explicit declaration `uid-seed`, including an explicit blank value;
2. a host-provided seed;
3. a source hash in deterministic build or SSR mode;
4. an ephemeral runtime fallback.

The generated value appears as `data-cem-render-scope` on the host and relevant
render roots. It namespaces render resources, instance IDs, hydration, patching,
diagnostics, and keyframe suffixes. It MUST NOT appear in generated CSS selectors
or authored application/component CSS.

SSR serializes render identity and the public declaration-owned `scope` value.
Hydration reuses them, restores an altered public scope, and MUST NOT reinterpret
render identity as CSS identity.

An anonymous `<cem-element>` still derives and registers a deterministic
UUID-shaped custom-element tag, writes it to the declaration, and creates one
adjacent instance. That generated tag is its private `@scope` root. Anonymous
declarations otherwise follow the same shared, projection, instance, and render
identity rules.

## 8. Diagnostics and Fail-Closed Rules

| Condition | Required result | Diagnostic |
| --- | --- | --- |
| declaration style content or `scope` is dynamic | omit the style artifact | `cem.ql.template.stylesheet_dynamic_unsupported` |
| declaration `scope` is empty, multiple, or not one CSS identifier | expose no named shared surface | `cem-element.stylesheet_scope_invalid` |
| explicit style scope does not match the declaration | do not install that style | `cem-element.stylesheet_scope_mismatch` |
| produced-host `scope` is added, changed, or removed contrary to its declaration | diagnose and restore declaration state | `cem-element.scope_mutation_restored` |
| direct payload template is mixed with non-whitespace siblings | leave payload inert and render no mixed payload | `cem-element.instance_payload_mixed` |
| bare instance `<style>` is found outside the payload envelope | do not adopt it as instance CSS | `cem-element.instance_style_unenveloped` |
| authored outer `@scope` appears in managed CSS | suppress the authored scope | `cem.scoped_css.authored_scope_unsupported` |
| `@import` appears in managed CSS | suppress the import | `cem.scoped_css.import_unsupported` |
| a document-global at-rule appears | suppress the construct | `cem.scoped_css.global_construct_unsupported` |
| `:global`, `:global(...)`, or `:root` appears | contain it as a host alias | `cem.scoped_css.global_alias` |

A missing concise declarative capability is a hard stop. Add the reusable
capability to `cem-elements` rather than bypassing the CSS contract with
component or application JavaScript.

## 9. Author Checklist

Before accepting component CSS:

- keep component-owned rules in static embedded CEM-ML styles;
- use `:host`, documented `part` values, slot-root markers, host/state/ARIA
  attributes, and CEM custom properties;
- use `scope` only for an intentionally public cross-component rule set;
- keep private and shared artifacts distinct when both are needed;
- use the inert payload envelope for any instance stylesheet;
- never select data-island or render-identity markers;
- stay within the specificity ceiling and avoid `!important`;
- verify that multiple instances share declaration styles; and
- keep component CSS out of standalone and package-global stylesheets.

## 10. Migration Verification

The migration MUST add executable evidence for:

- private, named-shared, and instance compilation output;
- generic page CSS versus equal-specificity component CSS;
- intentional public overrides and instance specificity;
- nested same-tag and different-tag DCE containment;
- named/default projected roots, text payload, and fallback content;
- inheritance and custom properties crossing scope limits;
- `<th scope="row">` collision resistance;
- invalid, dynamic, multiple, mismatched, and mutated scope values;
- inert payload safety, mixed-sibling rejection, and literal nested templates;
- complete removal of `data-cem-scope` and `data-cem-instance-scope` from the
  target styling path;
- SSR/hydration reuse without CSS use of render identity;
- once-per-declaration ownership and declaration removal;
- anonymous declaration scope roots;
- no generated layers or library `!important`; and
- computed specificity, scoping proximity, and source-order behavior.

The primary implementation gates remain:

```bash
yarn nx run cem_ql:test
yarn nx run cem-elements:test:unit
yarn nx run cem-elements:test
yarn nx run cem-elements:verify-demo-fixtures
yarn nx run @epa-wg/cem-components:verify
```

## 11. Standards Basis

- [CSS Cascading and Inheritance Level 6 — scoped styles](https://www.w3.org/TR/css-cascade-6/#scope-atrule)
- [MDN: `@scope`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@scope)
- [WHATWG HTML: custom elements](https://html.spec.whatwg.org/dev/custom-elements.html)
- [WHATWG HTML attribute index: `scope`](https://html.spec.whatwg.org/multipage/indices.html#attributes-3)
- [CSS Shadow Parts](https://www.w3.org/TR/css-shadow-parts-1/)
