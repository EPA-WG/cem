# Markdown site

Open **Samples → Amsterdam 2026 → Markdown site** in the ordinary Studio
workspace. Choose **Clone selected** to make an editable saved project. Each
copy owns its resources and configurations independently of the catalog original.
There is no dedicated site-example screen.

## Resources and configurations

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

- **Build and preview site** executes `graphs/build.cem`: native Markdown → HTML
  → retained DOM → CEM-ML layout → HTML export, plus opaque asset publication.
  The seven output artifacts are four HTML documents, CSS, an SVG and a download.
  Choose `generated/index.html` in **Preview artifact**. Follow article links,
  images and nested navigation inside the same immutable output tree. Selecting
  an artifact does not change the source file open in the editor.
- **Validate article metadata** validates the saved first article using its
  declared Markdown identity, including the opt-in YAML metadata contract.
- **Preview Markdown article** converts that saved article body on its own.
  The full build supplies the shared layout and assets.

`expected/` contains the baseline HTML for all four documents. The graph's
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
