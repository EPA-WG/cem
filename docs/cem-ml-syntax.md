# CEM-ML Syntax

> Status: chosen CEM-native canonical surface direction.
>
> This document records the canonical CEM-ML curly-brace surface and compares it
> with equivalent XML-convention forms. XML syntax remains a secondary parity
> surface and should be developed alongside each feature. The examples are
> syntax sketches, not a final grammar specification.

## Goals

CEM-ML should keep XML's useful separation between structure, attributes,
content, namespaces, and parser scopes while reducing XML delimiter noise.

The current base shape is:

```cem
{name @attributes | content...}
```

The equivalent XML convention is:

```xml

<name attributes="...">content...</name>
```

Core goals:

- Keep structure visually marked by `{...}`.
- Keep attributes visually marked by `@`.
- Keep content visually marked by `|`.
- Keep content-type switches explicit and schema-owned.
- Preserve source-map and scope frames for every syntax and content-type layer.
- Treat XML, HTML, SVG, CSS, JavaScript, JSON, and other formats as scoped
  content types, not opaque blobs unless the active schema says so.

## Visual Planes

| Plane                | CEM-ML proposal              | XML convention                                         | Meaning                                                   |
|----------------------|------------------------------|--------------------------------------------------------|-----------------------------------------------------------|
| Node start           | `{screen`                    | `<cem:screen`                                          | Opens a structured node.                                  |
| Node end             | `}`                          | `</cem:screen>`                                        | Closes the node.                                          |
| Attribute            | `@state=pending`             | `state="pending"`                                      | Metadata on the current node.                             |
| Namespaced node      | `{html:aside ...}`           | `<html:aside ...>`                                     | Node owned by a namespace.                                |
| Namespaced attribute | `@html:class="summary"`      | `html:class="summary"`                                 | Attribute owned by a namespace.                           |
| Content boundary     | `\|`                         | `>` after the start tag                                | Switches from node header to content.                     |
| Text content         | `\| Secure checkout`         | `Secure checkout`                                      | Character content.                                        |
| Raw content          | `` ```...``` ``              | `<![CDATA[...]]>` or raw-text element rules            | Content with reduced escaping.                            |
| Typed scope          | `{@type="text/html" \| ...}` | `<cem:scope type="text/html">...</cem:scope>`          | Parser/schema content-type scope without a semantic node. |
| Expression scope     | `{$ \| ...}`                 | `<cem:expr>...</cem:expr>`                             | Reserved cem-ql expression node.                          |
| Directive            | `@ns`, `@default`, `@schema` | `xmlns`, `xsi:schemaLocation`, processing instructions | Parser/schema control.                                    |

## ASCII And Unicode Profiles

CEM-ML may allow ASCII and Unicode surface symbols as equivalent lexical
choices. This is similar to allowing both single-quoted and double-quoted string
values: authors can choose the delimiter that causes the least escaping for the
local content. Canonical output should still pick one profile for stable diffs.

The Unicode profile uses white braces `⦃⦄` as the structural delimiters.
The heavier `【...】` pair is alternative syntax for rich content allowing to skip encoding the backticks and
double brackets.

The comment profile uses `※` for line comments and `⸨...⸩` for ordinary ignored block comments.
The starred parenthesis `﴾...﴿` pair is alternative syntax for block comments allowing to skip encoding other comment
characters.

| Role                                | ASCII           | Unicode               | Example                                                  |
|-------------------------------------|-----------------|-----------------------|----------------------------------------------------------|
| Markup beginning / node-scope begin | `{`             | `⦃` U+2983            | `{badge ...}` / `⦃badge ...⦄`                            |
| Markup end / node-scope end         | `}`             | `⦄` U+2984            | `{badge}` / `⦃badge⦄`                                    |
| Attribute marker                    | `@`             | `@`                   | `@tone=info`                                             |
| Reserved expression node            | `$`             | `$`                   | `{$ \| .items}` / `⦃$ ▷ .items⦄`                         |
| Content/context marker              | `\|`            | `▷` U+25B7            | `{badge \| text}` / `⦃badge ▷ text⦄`                     |
| Namespace separator                 | `:`             | `:`                   | `html:aside`                                             |
| Assignment                          | `=`             | `=`                   | `@type="text/html"`                                      |
| Double-quoted string                | `"..."`         | `"..."`               | `@label="Secure checkout"`                               |
| Single-quoted string                | `'...'`         | `'...'`               | `@label='Secure checkout'`                               |
| Rich content enclosure 1,2          | `` ```...``` `` | `⟦...⟧` U+27E6/U+27E7 | `` ```<code>`x`</code>``` ``  / `⟦<code>```x```</code>⟧` |
| Rich content enclosure 3            |                 | `【...】` U+3014/U+3015 | `【 ⟦<code>```x```</code>⟧ 】`                             |
| Block comment 1,2                   | `/* ... */`     | `⸨...⸩` U+2E28/U+2E29 | `⸨ ignored block ⸩`                                      |
| Block comment 3                     |                 | `﴾...﴿` U+FD3E/U+FD3F | `﴾ ⸨ignored⸩ block ﴿`                                    |
| Line comment                        | `// ...`        | `※ ...` U+203B        | `※ ignored to line end`                                  |

ASCII and Unicode forms are semantically equivalent:

```cem
{badge @tone="info" | Secure checkout}
```

```cem
⦃badge @tone='info' ▷ Secure checkout⦄
```

Rich/native content may choose any listed enclosure:

````cem
{@type="text/html" |
  ```
  <code>Use `cem parse` here.</code>
  ```
}
````

```cem
⦃@type='text/html' ▷
  ⟦
  <code>Use ``` fences here without escaping.</code>
  ⟧
⦄
```

```cem
⦃@type='text/html' ▷
  【
  <code>Use ⟦ rich content brackets ⟧ here without escaping.</code>
  】
⦄
```

Profile rules:

- ASCII and Unicode delimiter forms may be mixed only where the delimiters are
  locally balanced. For example, a node opened with `⦃` must close with `⦄`.
- String quote style is local: `"..."` and `'...'` have the same semantic value
  after unescaping.
- Rich/raw enclosure style is local: triple backticks, `⟦...⟧`, and `【...】`
  have the same semantic role after unescaping.
- Canonical CEM-ML should select one output profile, likely ASCII, unless a
  Unicode profile is explicitly requested.

## Core Rules

### Nodes

A named node starts with `{name` and ends with `}`.

CEM-ML:

```cem
{badge @tone=info |Secure checkout}
```

XML convention:

```xml

<cem:badge tone="info">Secure checkout</cem:badge>
```

### Attributes

Attributes are prefixed with `@`. A bare attribute is boolean-like and is
interpreted by the active schema.

CEM-ML:

```cem
{field @name=email @required @maxLength=120}
```

XML convention:

```xml

<cem:field name="email" required="required" maxLength="120"/>
```

Attribute values may be unquoted when they are simple identifiers or numbers.
Quoted string values may use either `"..."` or `'...'` as equivalent local
delimiter choices. Canonical serialization may quote all string values with one
chosen quote style.

Attribute values may also contain cem-ql expression spans. In attribute-value
mode, `{...}` is not CEM-ML structural syntax and cannot open a CEM-ML node; it
is part of the attribute value and is scanned by the cem-ql/AVT layer when the
active schema marks the attribute as template-aware.

CEM-ML:

```cem
{button @disabled={.busy} @label="Hello {.name}" | Save}
```

XML convention:

```xml

<cem:button disabled="{.busy}" label="Hello {.name}">Save</cem:button>
```

For template-aware attribute values, literal braces escape as `{{` and `}}`.

### Content Runs

The `|` token starts the content plane explicitly. Its Unicode equivalent is
`▷`. The marker is optional when the content boundary can be inferred from the
first non-attribute token. After the content plane starts, bare words are
content, not attributes. Nested `{name ...}` forms remain structured CEM-ML
nodes.

CEM-ML:

```cem
{action @id=pay @intent=primary |
  {icon @name=lock}
  Pay now
}
```

Equivalent relaxed form:

```cem
{action @id=pay @intent=primary
  {icon @name=lock}
  Pay now
}
```

XML convention:

```xml

<cem:action id="pay" intent="primary">
    <cem:icon name="lock"/>
    Pay now
</cem:action>
```

Relaxed content-boundary rules:

- `|` / `▷` may be used to mark the content boundary explicitly.
- `|` / `▷` may be omitted when the first non-attribute token starts content.
- A normal node header is `{` node-name attributes*. Attributes must start with
  `@`.
- The first non-attribute token after the node name and attributes starts
  CEM-ML content.
- In the content plane, `{name ...}` opens a nested CEM-ML node.
- In the content plane, native syntax may be used when the active content type
  says it owns that region.
- Canonical CEM-ML should include `|` / `▷` for clarity even when the parser
  accepts the relaxed form.
- Source-preserving AST nodes retain original content segments even when
  canonical semantic text is merged.

### Reserved cem-ql Node

`$` is a reserved node name. `{$ | ...}` opens a cem-ql expression scope rather
than a normal CEM semantic node. The node name selects the cem-ql content type;
the expression body is parsed by the cem-ql parser, not by the CEM-ML structural
parser. Braces inside the expression body therefore belong to cem-ql. The
content marker is optional here too: `{$ expr}` and `{$ | expr}` are equivalent.

CEM-ML:

```cem
{$ | .items.filter(|item| item.active).count()}
```

Equivalent relaxed form:

```cem
{$ .items.filter(|item| item.active).count()}
```

CEM-ML with a Rust-like cem-ql block expression:

```cem
{$ |
  {
    let active := .items.filter(|item| item.active);
    active.count()
  }
}
```

Attribute values can use direct cem-ql spans without the `$` node because the
attribute-value scanner already owns `{...}`:

```cem
{button @disabled={.busy} | Save}
```

Use the `$` node when an expression is itself a content item or when a full
expression scope is clearer than an inline AVT span.

### Template Embedding

Text templates use CEM-ML child nodes. A text template must use `{node ...}` for
structure or `{$ ...}` / `{$ | ...}` for cem-ql expressions. Bare `{...}`
cem-ql interpolation in text content is not permitted.

Default structural content:

```cem
{p Hello {em world}}
```

Template text with cem-ql expression nodes:

```cem
{p Hello {$ .name}, {$ count(.items)} items}
```

Template-embedding rules:

- Default content-plane parsing treats `{name ...}` as CEM-ML structure.
- `{$ ...}` or `{$ | ...}` is always an explicit cem-ql expression node and does
  not require the surrounding text position to be template-aware.
- Bare `{.name}` and `{count(.items)}` forms are not valid text interpolation in
  CEM-ML content. Write `{$ .name}` and `{$ count(.items)}` instead.
- Literal braces in text content are ordinary text unless they start a valid
  CEM-ML child node or `$` expression node; use a rich/raw enclosure when exact
  brace preservation would otherwise be unclear.
- Attribute values do not need the `$` node because the attribute-value scanner
  already owns `{...}` spans.

## Document Format Directive

Top-level canonical CEM-ML documents begin with a required document-format
directive:

```cem
@doc cem-ml 1
```

The form is `@doc cem-ml <version>`. The version accepts the SemVer constraint
forms defined by `cem-ml-ac.md` AC-F-8: `MAJOR`, `MAJOR.MINOR`, or full
`MAJOR.MINOR.PATCH` with optional prerelease/build metadata. Tier A defines the
canonical `cem-ml` format at embedded version `1.0.0`, so `@doc cem-ml 1`,
`@doc cem-ml 1.0`, and `@doc cem-ml 1.0.0` select the same Tier A parser
profile.

`@doc` must appear before any non-trivia top-level directive or item in a
persisted `.cem` document. Embedded CEM-ML fragments inherit the parent
document-format identity unless the host API supplies an explicit fragment
format. XML and HTML parity documents do not use `@doc`; their format identity
comes from the selected parser/content-type profile.

### Anonymous Typed Scopes

If the first item after `{` is an attribute or directive instead of a node name,
the block is an anonymous scope. Anonymous scopes are useful when only a parser
or content-type boundary is needed.

CEM-ML:

```cem
{@type="text/html" |
  <label>
    <svg viewBox="0 0 16 16" aria-hidden="true">
      <path d="M2 8h12"/>
    </svg>
    name: <input name="name"/>
  </label>
}
```

XML convention:

```xml

<cem:scope type="text/html">
    <label>
        <svg viewBox="0 0 16 16" aria-hidden="true">
            <path d="M2 8h12"/>
        </svg>
        name:
        <input name="name"/>
    </label>
</cem:scope>
```

The anonymous scope is not a semantic CEM node by default. It creates parser,
schema, policy, diagnostic, and source-map scope frames. A transform may choose
to materialize a node, but that is transform behavior, not syntax behavior.

### Content-Type Handoffs Stay Schema-Owned

`@type` selects the active child content type. CEM-ML remains in charge of the
scope stack, source maps, diagnostics, validation, and nested handoffs. The child
parser is not opaque unless the active schema marks the region as raw or
passthrough.

CEM-ML:

````cem
{@type="text/html" |
  ```
  <label>
    <script type="module">
      const input = document.querySelector("input[name='name']");
      input?.addEventListener("change", (event) => {
        console.log(event.target.value);
      });
    </script>
  </label>
  ```
}
````

XML convention:

```xml
<cem:scope type="text/html">
  <label>
    <script type="module">
      const input = document.querySelector("input[name='name']");
      input?.addEventListener("change", (event) => {
        console.log(event.target.value);
      });
    </script>
  </label>
</cem:scope>
```

The nested scope model is:

```text
CEM-ML scope
  anonymous typed scope: text/html
    HTML schema tokenizer/parser
      label element
      script element -> JavaScript module child scope
```

### Named Host Nodes

Use a named node when the host element has document semantics, runtime policy,
or transform behavior. `style` and `script` are usually named host nodes when
they are first-class CEM document nodes.

CEM-ML:

````cem
{style @type="text/css" |
  ```
  label {
    display: inline-flex;
    gap: .5rem;
  }
  ```
}
````

XML convention:

```xml

<cem:style type="text/css"><![CDATA[
  label {
    display: inline-flex;
    gap: .5rem;
  }
]]></cem:style>
```

CEM-ML:

````cem
{script @type="module" |
  ```
  const input = document.querySelector("input[name='name']");
  ```
}
````

XML convention:

```xml

<cem:script type="module"><![CDATA[
  const input = document.querySelector("input[name='name']");
]]></cem:script>
```

Use anonymous typed scopes when the wrapper would only select a parser. Use
named nodes when the name itself has semantic or runtime meaning.

## Namespaces

CEM-ML uses prefix declarations with `@ns`. Prefixes are local aliases. The
canonical identity is `{ namespaceUri, localName }`.

CEM-ML:

```cem
@ns cem = "https://cem.dev/ns/core/1"
@ns html = "http://www.w3.org/1999/xhtml"
@default cem

{screen @id=checkout |
  {html:aside @class="summary" |
    {badge @tone=info |Secure checkout}
  }
}
```

XML convention:

```xml

<cem:screen
        xmlns:cem="https://cem.dev/ns/core/1"
        xmlns:html="http://www.w3.org/1999/xhtml"
        id="checkout">
    <html:aside class="summary">
        <cem:badge tone="info">Secure checkout</cem:badge>
    </html:aside>
</cem:screen>
```

Namespace rules:

- `@ns prefix = "uri"` declares a namespace prefix in the current scope.
- `@default prefix` sets the default namespace for unprefixed node names.
- The same namespace binding name may be declared more than once in a scope.
  This includes the blank/default binding selected by `@default`.
- Prefix declarations are scoped to the containing block and descendants.
- A later declaration wins from its source position forward; earlier nodes keep
  the expanded namespace identity that was active where they appeared.
- Unprefixed attributes are interpreted by the current node schema.
- Prefixed attributes, such as `@html:class`, are explicitly namespace-owned.
- Rendered XML/HTML uses normal `xmlns` declarations.

### Default Namespace Rebinding

The blank/default namespace can be rebound so different schemas can be used
without prefixes at different source positions. This is useful for common HTML
with inline SVG, where both languages usually read better unprefixed.

CEM-ML:

```cem
@ns html = "http://www.w3.org/1999/xhtml"
@ns svg = "http://www.w3.org/2000/svg"
@default html

{label |
  @default svg
  {svg @viewBox="0 0 16 16" @aria-hidden=true |
    {path @d="M2 8h12"}
  }

  @default html
  name:
  {input @name=name}
}
```

XML convention:

```xml
<label xmlns="http://www.w3.org/1999/xhtml">
    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" aria-hidden="true">
        <path d="M2 8h12"/>
    </svg>

    name:
    <input name="name"/>
</label>
```

In the example, `{label}` and `{input}` resolve under the HTML namespace, while
`{svg}` and `{path}` resolve under the SVG namespace. The same lexical default
binding name is used for both schemas. The source-map namespace frame records
which default binding was active at each source position.

## Schema Scoping

CEM-ML keeps schema scoping explicit. The CEM-ML syntax differs from XML, but
the scope behavior maps to the same parser/schema model as AC-F-2 and
`cem-ml-stack-design.md` §13.1. Schema declarations and switches open schema
scopes. They do not mutate an ancestor scope.

### Form Matrix

| Role                         | CEM-ML canonical form                                     | XML convention                                                                                   |
|------------------------------|-----------------------------------------------------------|--------------------------------------------------------------------------------------------------|
| Document/block shorthand     | `@schema src="./schema.cem-schema"`                       | `<cem:schema src="./schema.cem-schema"/>` at the same sibling position, or host `cem:schema-src` |
| Inline schema declaration    | `{cem:schema @cem:name="badge" \| ...}`                   | `<cem:schema cem:name="badge">...</cem:schema>`                                                  |
| Mid-document sibling switch  | `{cem:schema @src="./schema.cem-schema"}`                 | `<cem:schema src="./schema.cem-schema"/>`                                                        |
| Wrapping schema switch       | `{cem:schema @src="./schema.cem-schema" \| ...}`          | `<cem:schema src="./schema.cem-schema">...</cem:schema>`                                         |
| Query-resolved schema switch | `{cem:schema @select='schema("badge")'}` or wrapping form | `<cem:schema select='schema("badge")'/>` or wrapping form                                        |
| Host-node URI switch         | `{section @cem:schema-src="./schema.cem-schema" \| ...}`  | `<cem:section cem:schema-src="./schema.cem-schema">...</cem:section>`                            |
| Host-node query switch       | `{section @cem:schema-select='schema("badge")' \| ...}`   | `<cem:section cem:schema-select='schema("badge")'>...</cem:section>`                             |

`@schema src="..."` is a document- or block-prelude shorthand. It lowers to a
sibling-position schema switch at the same point in the event stream. Use
`{cem:schema ...}` when the schema switch itself needs source identity, a query
form, wrapping content, or parity with XML examples.

### Inline Declaration

```cem
@ns cem = "https://cem.dev/ns/core/1"
@default cem

{cem:schema @cem:name="badge" |
  {define @name=badge |
    {attribute @name=tone}
    {text}
  }
}

{section @cem:schema-select='schema("badge")' |
  {badge @tone=info | Secure checkout}
}
```

XML convention:

```xml
<cem:schema xmlns:cem="https://cem.dev/ns/core/1" cem:name="badge">
    <define name="badge">
        <attribute name="tone"/>
        <text/>
    </define>
</cem:schema>

<cem:section cem:schema-select='schema("badge")'>
    <cem:badge tone="info">Secure checkout</cem:badge>
</cem:section>
```

Inline schema declarations bind an addressable name in the scope chain. The
declaration does not switch the active schema of the parent scope. Descendant
schema selections resolve the innermost matching `cem:name`.

### Mid-Document Switch

CEM-ML:

```cem
@ns cem = "https://cem.dev/ns/core/1"
@default cem

{cem:schema @src="./schemas/admin.cem-schema"}

{section |
  {action @intent=danger |Delete account}
}
```

XML convention:

```xml

<cem:schema xmlns:cem="https://cem.dev/ns/core/1" src="./schemas/admin.cem-schema"/>

<cem:section>
    <cem:action intent="danger">Delete account</cem:action>
</cem:section>
```

The self-closing switch opens a sibling-position scope. The loaded schema
applies to itself and subsequent siblings until the end of the parent scope.

### Wrapping Switch

CEM-ML:

```cem
{cem:schema @src="./schemas/admin.cem-schema" |
  {section |
    {action @intent=danger |Delete account}
  }
}
```

XML convention:

```xml
<cem:schema xmlns:cem="https://cem.dev/ns/core/1" src="./schemas/admin.cem-schema">
    <cem:section>
        <cem:action intent="danger">Delete account</cem:action>
    </cem:section>
</cem:schema>
```

The wrapping switch opens a schema scope for its own content only. The parent
scope is unchanged after the wrapper closes.

### Host-Node Switch

CEM-ML:

```cem
{section @cem:schema-src="./schemas/admin.cem-schema" |
  {action @intent=danger |Delete account}
}
```

XML convention:

```xml
<cem:section xmlns:cem="https://cem.dev/ns/core/1" cem:schema-src="./schemas/admin.cem-schema">
    <cem:action intent="danger">Delete account</cem:action>
</cem:section>
```

The host-node switch makes that node a schema scope. The loaded schema applies
inside the host only; siblings remain under the parent scope's active schema.

### Source Attributes

| Attribute | Value form | Resolution |
|-----------|------------|------------|
| `src` on `{cem:schema}` / `@cem:schema-src` on any node | URI literal | Resolved through the transform-source loader and resource policy. |
| `select` on `{cem:schema}` / `@cem:schema-select` on any node | cem-ql expression | Evaluated against the active document and scope chain; innermost match wins. |

`src` and `select` are mutually exclusive on one schema-switch host. A
schema-switching host with neither is a schema-compilation error.

### Identifier Resolution

| Aspect | Behavior |
|--------|----------|
| Declaration | `@cem:name="..."` on an inline `{cem:schema ...}` declaration. |
| Visibility | Scope-chain visible to the host scope and descendants. |
| Override | A nested inline declaration with the same `@cem:name` shadows the outer declaration inside the nested scope only. |
| Uniqueness | Names do not need to be globally unique; shadowing is legal. |
| Reference | `@cem:schema-select` or `{cem:schema @select=...}` resolves through cem-ql against the scope chain. |
| Cache identity | Inline schema cache identity is content-addressed from the body; `@cem:name` is an alias, not the cache key. |

Schema-scope rules:

- `@schema src="..."` is a prelude shorthand for a sibling-position schema switch.
- Use `@cem:schema-src="..."` and `@cem:schema-select="..."` on host nodes; the `cem:` prefix is part of the schema-owned attribute identity.
- Inline schema declarations use `{cem:schema @cem:name="..." | ...}`.
- Schema switches open child scopes; they do not mutate ancestor scopes.
- When NVDL-style namespace dispatch and an explicit schema switch both apply,
  namespace dispatch applies first and the explicit switch layers within that scope.

## Document Example

CEM-ML:

````cem
@doc cem-ml 1
@ns cem = "https://cem.dev/ns/core/1"
@ns html = "http://www.w3.org/1999/xhtml"
@default cem
@schema src="./schemas/checkout.cem-schema"

{screen @id=checkout |
  {style @type="text/css" |
    ```
    .checkout-card {
      display: grid;
      gap: 1rem;
    }
    ```
  }

  {@type="text/html" |
    ```
    <section class="checkout-card">
      <h1>Checkout</h1>
      <label>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M2 8h12"/>
        </svg>
        name: <input name="name"/>
      </label>
    </section>
    ```
  }

  {form @id=payment @state=pending |
    {action @id=pay @intent=primary |
      Pay now
    }
  }
}
````

XML convention:

```xml
<?xml version="1.0"?>
<cem:screen
        xmlns:cem="https://cem.dev/ns/core/1"
        xmlns:html="http://www.w3.org/1999/xhtml"
        cem:schema-src="./schemas/checkout.cem-schema"
        id="checkout">
    <cem:style type="text/css"><![CDATA[
    .checkout-card {
      display: grid;
      gap: 1rem;
    }
  ]]></cem:style>

    <cem:scope type="text/html">
        <section class="checkout-card">
          <h1>Checkout</h1>
          <label>
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M2 8h12"/>
            </svg>
            name: <input name="name"/>
          </label>
        </section>
    </cem:scope>

    <cem:form id="payment" state="pending">
        <cem:action id="pay" intent="primary">
            Pay now
        </cem:action>
    </cem:form>
</cem:screen>
```

## Draft Grammar Sketch

This sketch is intentionally incomplete. It captures the current shape only.
The grammar names token kinds using ASCII spellings for readability; the
tokenizer recognizes ASCII and Unicode spellings as the same lexical token kinds
before grammar parsing.

```text
document         := doc_directive directive* item*
fragment         := directive* item*
item             := node | expression_node | anonymous_scope | directive | comment | content
node             := "{" qname attribute* content_boundary? item* "}"
expression_node  := "{" "$" content_boundary? cem_ql_expression "}"
anonymous_scope  := "{" attribute+ content_boundary? item* "}"
attribute        := "@" qname ("=" value)?
content_boundary := "|"
qname            := name | prefix ":" name
value            := bare_value | quoted_string | fenced_block | cem_ql_span
cem_ql_span      := "{" cem_ql_expression "}"  // attribute-value mode only
doc_directive    := "@doc" format_id semver_constraint
directive        := "@ns" ... | "@default" ... | "@schema" ...
```

Remaining grammar details:

- Exact escaping rules for quoted strings and fenced blocks.
- Whether `|` inline content ends at line end or only at the closing `}`.
- Whether canonical CEM-ML quotes all text and attribute values.
- Whether comments are AST-preserved by default in every content type.
- The complete grammar for inline schema declarations.
