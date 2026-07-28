# Browser Markdown

Markdown defines content that is normally expressed in the browser as HTML.

The next block embeds an SVG authored as CEM-ML and exports it as inline HTML
SVG markup.

```cem-ml svg
@doc cem-ml 1
{svg @xmlns="http://www.w3.org/2000/svg" @viewBox="0 0 160 80" |
    {title | CEM-ML inline SVG}
    {path @d="M20 40h120M80 14v52M48 24l32 16-32 16"}
}
```

The generated HTML keeps the SVG inline, so a browser can render it without a
separate image file.
