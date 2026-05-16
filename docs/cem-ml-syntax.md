# CEM-ML Syntax Draft

> Status: proposed syntax draft.
>
> This document records the current CEM-ML surface syntax proposal and compares
> it with equivalent XML-convention forms. The examples are syntax sketches, not
> a final grammar specification.

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

### Content Runs

The `|` token starts the content plane. After `|`, bare words are content, not
attributes. Nested `{name ...}` forms remain structured CEM-ML nodes.

CEM-ML:

```cem
{action @id=pay @intent=primary |
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

Content-run rules:

- Before `|`, the parser is in the node header plane.
- After `|`, the parser is in the content plane.
- In the content plane, `{name ...}` opens a nested CEM-ML node.
- In the content plane, native syntax may be used when the active content type
  says it owns that region.
- Source-preserving AST nodes retain original content segments even when
  canonical semantic text is merged.

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
- Prefix declarations are scoped to the containing block and descendants.
- Unprefixed attributes are interpreted by the current node schema.
- Prefixed attributes, such as `@html:class`, are explicitly namespace-owned.
- Rendered XML/HTML uses normal `xmlns` declarations.

## Schema Scoping

CEM-ML keeps schema scoping explicit. The CEM-ML syntax differs from XML, but
the scope behavior maps to the same parser/schema model.

CEM-ML:

```cem
@schema src="./schemas/checkout.cem-schema"

{screen @id=checkout |
  {form @id=payment @state=pending}
}
```

XML convention:

```xml

<cem:screen
        xmlns:cem="https://cem.dev/ns/core/1"
        cem:schema-src="./schemas/checkout.cem-schema"
        id="checkout">
    <cem:form id="payment" state="pending"/>
</cem:screen>
```

CEM-ML:

```cem
{section @schema-src="./schemas/admin.cem-schema" |
  {action @intent=danger |Delete account}
}
```

XML convention:

```xml

<cem:section cem:schema-src="./schemas/admin.cem-schema">
    <cem:action intent="danger">Delete account</cem:action>
</cem:section>
```

Schema-scope rules:

- `@schema src="..."` applies to the current document or block scope.
- `@schema-src="..."` on a node makes that node a schema scope.
- `@schema-select="..."` resolves a schema through CEM-QL in the active scope.
- Inline schema declarations are allowed where the schema language permits them.
- Schema switches open child scopes; they do not mutate ancestor scopes.

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
document        := directive* item*
item            := node | anonymous_scope | directive | comment | content
node            := "{" qname attribute* content_run? item* "}"
anonymous_scope := "{" attribute+ content_run? item* "}"
attribute       := "@" qname ("=" value)?
content_run     := "|" content_body
qname           := name | prefix ":" name
value           := bare_value | quoted_string | fenced_block
content_body    := text | fenced_block | native_content | item*
directive       := "@doc" ... | "@ns" ... | "@default" ... | "@schema" ...
```

Open decisions:

- Exact escaping rules for quoted strings and fenced blocks.
- Whether `|` inline content ends at line end or only at the closing `}`.
- Whether canonical CEM-ML quotes all text and attribute values.
- Exact comment syntax and whether comments are AST-preserved by default.
- The complete grammar for inline schema declarations.
