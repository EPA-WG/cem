# CEM-Native Template Schema (`https://cem.dev/ns/template/cem-native/1`)

**Status:** reserved adapter-owned module syntax contract; execution is deferred.

This is the source-of-truth schema artifact for CEM-native transform templates.
It is separate from the CEM core document schema and separate from the CLI
transform graph config schema. The template schema describes module
declarations, template entrypoints, params, imports, and calls consumed by a
CEM-native template adapter; it does not describe ordinary CEM documents and it
does not define CLI graph wiring.

Schema URI: `https://cem.dev/ns/template/cem-native/1`
Namespace URI: `https://cem.dev/ns/template/cem-native/1`

## Element Vocabulary

| Element | Required attributes | Optional attributes | Child elements |
| ------- | ------------------- | ------------------- | -------------- |
| `module` | none | `version` | `import`, `param`, `template`, `body` |
| `import` | `as`, `src` | `content-type`, `contentType`, `schema` | none |
| `param` | `name` | `default`, `required`, `type`, `visibility` | none |
| `template` | `name` | `visibility` | `param`, `body` |
| `body` | none | none | `*` |
| `call` | `template` | `from`, `with:*` | none |

## Module Semantics

- A document contains one `module` root for the native-template declaration
  surface.
- `module` may declare imports, module params, named templates, and one default
  `body`.
- The module-level `body` is the implicit render entrypoint.
- `template @name="NAME"` declares an explicit named entrypoint.
- Declarations are private by default. `@visibility="public"` is required for
  cross-module template entrypoints.
- `param` declarations are immutable for a render call. `@type` may be `any`,
  `string`, `boolean`, `number`, `integer`, `array`, `object`, or `json`; omitted
  `@type` means `any`.
- `@default` provides a literal template default value. For `any` and `string`,
  the attribute text is a string value. For `boolean`, the literal must be
  `true` or `false`. For `number`, `integer`, `array`, `object`, and `json`, the
  literal is parsed as JSON and must match the declared type.
- `@required="true"` requires a caller value when no default is present.
- `import @as="ALIAS" @src="URI"` loads a separate module through the template
  resolver. The imported module's private declarations remain isolated.
- `content-type`/`contentType` and `schema` on `import` provide identity hints
  for the imported template resource.
- `call @template="NAME"` invokes a template in the current module.
- `call @from="ALIAS" @template="NAME"` invokes a public template exported by an
  imported module.
- `with:*` attributes on `call` pass named data/param bindings to the callee.
- `body` contains ordinary CEM fragment content plus adapter instruction
  elements such as `call`.

## Reserved Semantics

- `include` is intentionally not part of this schema. Lexical include remains a
  reserved future feature.
- Default expressions remain reserved. Defaults are literal values only until the
  template adapter contract defines expression context, resolver access, and
  failure reporting for default evaluation.

## Example

```cem
{@doc cem-ml 1}
{module @version="1" |
  {import @as="ui" @src="ui.cem" @content-type="text/cem-ml"}
  {param @name="locale" @default="en-US"}

  {body |
    {call @from="ui" @template="page" @with:locale="locale"}
  }

  {template @name="card" @visibility="public" |
    {param @name="title" @required="true"}
    {body |
      {article |
        {h2 | {$title}}
      }
    }
  }
}
```
