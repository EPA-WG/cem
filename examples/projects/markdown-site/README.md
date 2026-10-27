# Markdown site

Open **Examples → Content types and schemas → Documents and UI → Markdown site**,
or find the same project under **Amsterdam 2026**. Choose **Clone selected** to
make an editable saved project. Each
copy owns its resources and configurations independently of the catalog original.
There is no dedicated site-example screen.

## Resources and configurations

The declared source identities in [example.json](example.json) are:

| Resources | Content type | Schema identity |
| --- | --- | --- |
| `content/**/*.md`, `failures/invalid-article.md` | `text/markdown; charset=utf-8; variant=CommonMark; front-matter=yaml` | `https://cem.dev/ns/data/markdown/1` |
| `data/site.json`, `expected/article-outline.json` | `application/json` | `https://cem.dev/ns/data/json/1` |
| `queries/article-outline.xpath` | `application/vnd.cem.xpath` | `https://cem.dev/ns/query/xpath/1` |
| `layouts/page.cemt`, `layouts/partials/*.cemt` | `application/vnd.cem.transform+cem` | `https://cem.dev/ns/transform/cem/1` |
| `graphs/build.cem` | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` |
| `expected/**/*.html` | `text/html` | `https://cem.dev/ns/data/html/1` |
| `assets/site.css` | `text/css` | None declared; opaque asset copy |
| `static/images/harbor.svg` | `image/svg+xml` | None declared; opaque asset copy |
| `static/downloads/notes.txt` | `text/plain; charset=utf-8` | None declared; opaque asset copy |

The three static assets are untyped by a declared schema and copied unchanged.
Their presence does not claim CSS/SVG/plain-text validation or transformation
support. The native graph owns publication and the shared preview owns admission.

`content/index.md` is home; `content/articles/index.md` lists two articles.
All four resources opt into native YAML front matter. `title` and `description`
feed the HTML head. The project-owned `base` string is `.` at home and `..`
in the articles folder; the layout uses it for relative navigation and CSS.
Output paths come from the explicit graph's rooted folder bindings, not metadata.
`tags`, `weight` and `draft` demonstrate typed metadata; this graph does not
sort, filter or schedule content using them.

`layouts/page.cemt` composes the native HTML DOM with portable CEM-ML partials:
`partials/content.cemt` emits the retained content, and `partials/chrome.cemt`
supplies shared navigation and footer. `data/site.json` is an explicitly declared
input, imported through the common native data boundary. `assets/site.css` and
`static/` are published byte for byte. The stylesheet uses browser system colors
and follows the viewer's light/dark or forced-color settings independently of
Studio's own theme. No JavaScript assembles or renders the site.

- **Build and preview site** (`build-site`) executes `graphs/build.cem`: native Markdown → HTML
  → retained DOM → CEM-ML layout → HTML export, plus opaque asset publication.
  The seven output artifacts are four HTML documents, CSS, an SVG and a download.
  The configuration declares `entryArtifact: "generated/index.html"`; Build
  opens home without an artifact URL or manual selection. Follow article links,
  images and nested navigation inside the same immutable output tree. Selecting
  an artifact does not change the source file open in the editor.
- **Validate article metadata** (`validate-article`) validates the saved first article using its
  declared Markdown identity, including the opt-in YAML metadata contract.
- **Preview Markdown article** (`preview-article`) converts that saved article body on its own.
  Its exact HTML matches `expected/article-body.html`; front matter is omitted.
  The full build supplies the shared layout and assets.
- **Inspect Markdown source events** (`inspect-article`) presents the native Markdown
  events and original source ranges before HTML lowering, including front matter,
  text and link destinations. Its heading text starts at original UTF-8 byte 171.
  This event projection is not a complete Markdown language AST. Use **Preview
  Markdown article** for the generated HTML. Version **1.1.1** retains resource and
  configuration IDs and paths; existing saved projects keep their own settings.
- **Query the generated article outline** (`query-article-outline`) runs `queries/article-outline.xpath`
  over the same generated HTML using the explicit namespace binding
  `h=http://www.w3.org/1999/xhtml`. Its four result strings match
  `expected/article-outline.json`.
- **Repair the article metadata type** (`repair-article-metadata`) validates
  `failures/invalid-article.md`, a deliberately invalid resource outside the
  site's `content/**/*.md` glob. It can be repaired independently without making
  the ordinary site build fail.

## Work with one article

Choose **Preview Markdown article** and run the saved configuration to see the
body without its site layout or YAML prefix. Its relative links and image URL
are retained as authored references. This standalone artifact does not bundle
the other pages, image, CSS or download; use **Build and preview site** to follow
those references inside the admitted output tree. Shared preview controls keep
the existing script and network policy.

Run **Query the generated article outline**. The result is:

```text
heading=Build a small site
images=1
links=3
image=../images/harbor.svg
```

The heading comes from `# Build a small site` in the body. Changing only the
front-matter `title` changes the complete site's HTML title, while leaving this
body outline unchanged. Metadata and page content are separate native inputs to
the layout; a generated HTML query does not expose front matter as visible text.
The `inspect-article` configuration likewise presents generated HTML structure.
Its displayed coordinates do not become owned artifact-source links in the
current Parse/Inspect publication contract. Use the build's actual provenance
for original Markdown/layout navigation, as described below.
The outline configuration supplies its `h` binding during execution. Standalone
XPath source validation currently does not inherit caller namespace bindings;
standalone XPath Parse/Inspect/format are not configurations here.

For the deliberate type failure, run **Repair the article metadata type**. The
native validator reports `cem.markdown.front_matter_invalid` for `weight: heavy`
at line 2, column 1, original byte offset 4. Replace it with `weight: 1`, save
and rerun the same configuration. The error clears. All metadata contracts are
opted into through the declared `front-matter=yaml` MIME parameter; removing
that parameter gives ordinary Markdown interpretation rather than a metadata
validation contract.

Source validation checks Markdown's supported syntax, MIME parameters and
declared metadata types. It does not establish application rules such as unique
titles, publishing dates, sorting or link availability. Native Markdown
formatter profiles currently preserve lexical content rather than reflowing
paragraphs. See the [Markdown package](../../../packages/cem_ml/schema-packages/markdown/v1/README.md)
for the supported parser, front-matter and conversion boundaries.

## Build the complete folder site

In an editable saved copy, expand **Build entry settings** to save another
project-relative output path or clear the default. URL/selector choices take
precedence. A missing entry reports unavailable instead of opening another
index document. Entry changes use native validation and project revision/undo
guards, and travel with portable copies.

`expected/` contains the baseline HTML for all four documents, the standalone
article body and its query result. The graph's
expected asset results are their original source bytes. Command configurations
under `config/` are ordinary editable resources in each portable Studio bundle.

Native CLI equivalent, from this project folder:

```sh
cem-ml transform --config graphs/build.cem --config-content-type application/cem --config-schema https://cem.dev/ns/cli/transform-config/1
```

Serve the contents of `generated/` as a static site root. Native CLI source-map
sidecars are separate provenance exports; Studio retains provenance on its run.
This folder structure is Hugo-inspired. It does not interpret Hugo configuration,
Go templates, implicit menus, slug routing or automatic asset processing.

## Edit, fail and repair

Open `content/articles/first-build.md`, change **Build a small site** to
**Build an edited site**, save, and build. Its HTML title and heading change;
the other content resources stay independent. Build always captures saved
resources. Unsaved drafts remain drafts and mark the preview stale.

Open `layouts/partials/chrome.cemt` and change **Built from saved project
resources.** to **Shared layout edited.** Save and rebuild: all four HTML
documents now use that footer. Inspect output provenance to return to an
original Markdown resource or the shared layout/partial. Native writer-token
ranges retain honest source boundaries, without inventing a character mapping
for computed values or copied static assets.

To navigate a generated range with the shared source controls, select the
**Build and preview site** configuration, expand **Commands and results**, and
run it with **Run configuration**. Open **Results → Provenance**, then choose
an entry in **Source-map provenance**. Studio opens that source resource and
selects its original range. The article's metadata title and body heading have
separate origins; layout and partial instructions retain their own resource
identities. Synthetic bytes and computed strings do not acquire arbitrary
character-by-character lineage to the Markdown source. **Build project** runs
the same graph for the project output panel; command results provide the source
navigation controls.

In the first article, replace `title: Build an edited site` (or the original
title) with `title: [broken`. Save. Validation and Build report
`cem.markdown.front_matter_invalid`; the failed run publishes no new artifact
set, and Studio keeps the last successful preview with its failure notice.
Repair to a string title, save and rebuild. Removing a declared layout/partial
dependency also fails explicitly rather than loading an undeclared resource.

Export the saved project as a portable bundle and import it under a new name.
Reopen and build offline after Studio has cached its worker and samples. Copies
retain stable resource IDs/reference paths, but independent project roots,
revisions, drafts and builds. Editing one copy never changes the sample or
another copy. HTML links and assets resolve inside the admitted build; active
scripts and external network dependencies remain subject to the shared preview
policy. A download link demonstrates a published asset; use Studio's exact
artifact download control to save those bytes.

Offline commands require their runtime and source packages to be cached. In the
optional-package deployment, first run the Markdown-dependent workflow online
to admit its package before reopening it offline. This project adds no theme or
storage keys and contains no credential controls. Studio's existing rule still
blocks recovery while an entered password or one-time code is present elsewhere.
Automatic whole-Studio recovery remains disabled. The project supplies no local
JavaScript editing, preview or recovery behavior.
