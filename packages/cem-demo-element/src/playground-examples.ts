// CEM-ML sources use the native tabular formatter.
// The login example is adapted from examples/cem-ml/login.cem.
export const playgroundExamples = [
    {
        title: 'Hello, CEM',
        description: 'A small HTML example to edit, convert, and render.',
        source: `<article>
    <h2>Hello, CEM</h2>
    <p>Edit this HTML, then choose another output format.</p>
    <button type="button">A live HTML button</button>
</article>`,
        input: 'html',
        output: 'html',
    },
    {
        title: 'Hello, CEM-ML',
        description: 'The same greeting written in CEM-ML and rendered as HTML.',
        source: `{article |
    {h2 | Hello, CEM-ML}
    {p | Edit this CEM-ML, then choose another output format.}
    {button @type=button | A live HTML button}
}
`,
        input: 'cem',
        output: 'html',
    },
    {
        title: 'Login form CEM-ML',
        description: 'A sign-in form with labeled email and password fields.',
        source: `@doc cem-ml 1
@ns cem = "https://cem.dev/ns/core/1"
@ns html = "http://www.w3.org/1999/xhtml"
@default html
{main @aria-labelledby=login-title @cem:screen=login |
    {h1 @id=login-title | Sign in}
    {form @action="#session" @method=post @cem:form=sign-in |
        {label @for=email | Email}
        {input @autocomplete=email @id=email @name=email @required @type=email}
        {label @for=password | Password}
        {input @autocomplete=current-password @id=password @name=password @required @type=password}
        {button @type=submit @cem:action=primary | Sign in}
}   }
`,
        input: 'cem',
        output: 'html',
    },
    {
        title: 'Markdown cem-ml',
        description: 'Markdown content with an inline SVG authored in a CEM-ML fence.',
        source: `# Markdown meets CEM-ML

Write **content in Markdown** and include structured graphics with CEM-ML.

- Markdown supplies headings, paragraphs, and lists.
- The CEM-ML fence below becomes an inline SVG in the HTML output.

\`\`\`cem-ml svg
@doc cem-ml 1
{svg @xmlns="http://www.w3.org/2000/svg" @viewBox="0 0 160 80" @width=320 @height=160 |
    {title | CEM-ML inline SVG}
    {path @d="M20 40h120M80 14v52M48 24l32 16-32 16" @stroke=teal @stroke-width=4 @fill=none}
}
\`\`\`

One document, with **Markdown and CEM-ML** rendered together.
`,
        input: 'markdown',
        output: 'html',
    },
    {
        title: 'YML to JSON',
        description: 'Convert YAML mappings, lists, numbers, and booleans to JSON.',
        source: `name: CEM
version: 1
enabled: true
formats:
  - YAML
  - JSON
`,
        input: 'yaml',
        output: 'json',
    },
    {
        title: 'CSV to CEM-ML',
        description: 'CSV to CEM-ML conversion example. Currently reports a missing CSV AST export adapter.',
        source: `name,format,enabled
Orders,CSV,true
Catalog,YAML,false
`,
        input: 'csv',
        output: 'cem',
    },
] as const;
