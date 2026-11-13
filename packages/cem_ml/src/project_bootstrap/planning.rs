//! Native questionnaire interpretation and hierarchy review shared by all hosts.
use super::*;
use crate::transform_template::transform_template_encode_cem_attribute_value;
use std::collections::BTreeSet;

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBootstrapAnswers {
    pub source_files: Option<String>,
    pub target_files: Option<String>,
    pub source_folder: Option<String>,
    pub output_folder: Option<String>,
    pub layout_folder: Option<String>,
    pub css_folder: Option<String>,
    pub static_folder: Option<String>,
    pub input_format: Option<String>,
    pub output_format: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapTree {
    pub key: String,
    pub name: String,
    pub kind: &'static str,
    pub path: String,
    pub children: Vec<BootstrapTree>,
}
impl BootstrapTree {
    fn root(key: &str, name: &str) -> Self {
        Self {
            key: key.into(),
            name: name.into(),
            kind: "folder",
            path: String::new(),
            children: vec![],
        }
    }
    fn insert(&mut self, path: &str, key: &str) {
        let mut parent = self;
        let parts: Vec<_> = path.split('/').collect();
        let mut prefix = String::new();
        for (index, part) in parts.iter().enumerate() {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(part);
            let folder = index + 1 < parts.len();
            let position = parent
                .children
                .iter()
                .position(|c| c.name == *part)
                .unwrap_or_else(|| {
                    parent.children.push(Self {
                        key: if folder {
                            format!("{}-{}", parent.key, parent.children.len())
                        } else {
                            key.into()
                        },
                        name: (*part).into(),
                        kind: if folder { "folder" } else { "file" },
                        path: prefix.clone(),
                        children: vec![],
                    });
                    parent.children.len() - 1
                });
            parent = &mut parent.children[position];
        }
    }
    fn folders(&self, result: &mut Vec<String>) {
        if self.kind == "folder" {
            result.push(self.key.clone());
        }
        for child in &self.children {
            child.folders(result);
        }
    }
    fn sort(&mut self) {
        self.children.sort_by(|a, b| a.path.cmp(&b.path));
        for c in &mut self.children {
            c.sort();
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapReview {
    pub sources: BootstrapTree,
    pub outputs: BootstrapTree,
    pub source_expanded: Vec<String>,
    pub output_expanded: Vec<String>,
    pub pipelines: Vec<BootstrapPipeline>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapPipeline {
    pub resource_id: String,
    pub path: String,
    pub nodes: Vec<BootstrapStage>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStage {
    pub id: String,
    pub kind: crate::transform_config::TransformGraphNodeKind,
    pub source_path: Option<String>,
    pub target_path: Option<String>,
}

pub fn review_configuration(
    config: &ProjectBootstrapConfiguration,
) -> Result<BootstrapReview, ProjectBootstrapError> {
    let generated = generate_project(config.clone())?;
    let plan = plan_project_filesystem(&generated.bundle.project)
        .map_err(|e| error("filesystem_invalid", e, None))?;
    let mut result = BootstrapReview {
        sources: BootstrapTree::root("sources", "Project sources"),
        outputs: BootstrapTree::root("outputs", "Target output files"),
        source_expanded: vec![],
        output_expanded: vec![],
        pipelines: vec![],
    };
    let mut outputs = BTreeSet::new();
    for resource in &config.resources {
        result
            .sources
            .insert(&resource.path, &format!("resource-{}", resource.id));
        if matches!(
            resource.role,
            StudioProjectResourceRole::Graph | StudioProjectResourceRole::TransformConfig
        ) {
            let uri = format!(
                "{}.resources/{}",
                generated.bundle.project.root_uri, resource.id
            );
            let graph = parse_transform_graph_config(TransformGraphParseRequest {
                bytes: resource.text.as_bytes().to_vec(),
                base_uri: Some(uri.clone()),
                identity: FormatIdentity {
                    content_type: Some(resource.content_type.clone()),
                    schema: resource.schema.clone(),
                    ..FormatIdentity::default()
                },
            })
            .map_err(|e| error("graph_invalid", e.to_string(), None))?
            .graph;
            let mut nodes = vec![];
            for node in graph.nodes {
                let source_path = node.src.as_ref().and_then(|reference| {
                    plan.file(&resolve_transform_graph_reference(&uri, reference))
                        .map(|f| f.path.clone())
                });
                let target_path = node
                    .out
                    .as_ref()
                    .map(|out| plan.output_path(&resolve_transform_graph_reference(&uri, out)))
                    .transpose()
                    .map_err(|e| error("output_invalid", e, None))?;
                if let Some(path) = &target_path {
                    outputs.insert(path.clone());
                }
                nodes.push(BootstrapStage {
                    id: node.id,
                    kind: node.kind,
                    source_path,
                    target_path,
                });
            }
            result.pipelines.push(BootstrapPipeline {
                resource_id: resource.id.clone(),
                path: resource.path.clone(),
                nodes,
            });
        }
    }
    for entry in &config.entries {
        if let Some(path) = &entry.entry_artifact {
            plan.output_uri(path)
                .map_err(|e| error("output_invalid", e, None))?;
            outputs.insert(path.clone());
        }
    }
    // Source folders must not also be a target file (and vice versa).
    for (index, output) in outputs.iter().enumerate() {
        if outputs
            .iter()
            .any(|other| other != output && other.starts_with(&format!("{output}/")))
        {
            return Err(error(
                "output_path_conflict",
                "A target file cannot also be an output folder",
                Some("targetFiles"),
            ));
        }
        for resource in &config.resources {
            if output.starts_with(&format!("{}/", resource.path))
                || resource.path.starts_with(&format!("{output}/"))
            {
                return Err(error(
                    "output_source_conflict",
                    "Target files conflict with source folders",
                    Some("targetFiles"),
                ));
            }
        }
        result.outputs.insert(output, &format!("target-{index}"));
    }
    result.sources.sort();
    result.outputs.sort();
    result.sources.folders(&mut result.source_expanded);
    result.outputs.folders(&mut result.output_expanded);
    Ok(result)
}

pub fn plan_project(
    preset: ProjectBootstrapPreset,
    project_id: String,
    name: String,
    created_at: String,
    mut answers: ProjectBootstrapAnswers,
) -> Result<Value, ProjectBootstrapError> {
    let mut config = preset_configuration(preset, project_id, name, created_at)?;
    let (sources, targets) = match preset {
        ProjectBootstrapPreset::MarkdownSite => ("index.md\nabout.md\nguides/start.md", ""),
        ProjectBootstrapPreset::TransformationPipeline => {
            ("data/source.xml", "generated/result.html")
        }
        ProjectBootstrapPreset::SimpleConversion => ("data/source.yaml", "generated/result.cem"),
        ProjectBootstrapPreset::SchemaValidation => ("data/document.xhtml", ""),
        ProjectBootstrapPreset::Custom => ("README.md", ""),
    };
    answers.source_files.get_or_insert_with(|| sources.into());
    answers.target_files.get_or_insert_with(|| targets.into());
    answers
        .source_folder
        .get_or_insert_with(|| "content".into());
    answers
        .output_folder
        .get_or_insert_with(|| "generated".into());
    answers.layout_folder.get_or_insert_with(|| {
        if preset == ProjectBootstrapPreset::MarkdownSite {
            "layouts"
        } else {
            "templates"
        }
        .into()
    });
    answers
        .css_folder
        .get_or_insert_with(|| "assets/css".into());
    answers.static_folder.get_or_insert_with(|| "static".into());
    answers.input_format.get_or_insert_with(|| "yaml".into());
    answers.output_format.get_or_insert_with(|| "cem".into());
    let sources = file_lines(answers.source_files.as_deref().unwrap(), "sourceFiles")?;
    let targets = file_lines(answers.target_files.as_deref().unwrap(), "targetFiles")?;
    if preset != ProjectBootstrapPreset::Custom && sources.is_empty() {
        return Err(error(
            "source_required",
            "Add at least one source file",
            Some("sourceFiles"),
        ));
    }
    config.resources.clear();
    config.entries.clear();
    match preset {
        ProjectBootstrapPreset::MarkdownSite => site(&mut config, &answers, &sources)?,
        ProjectBootstrapPreset::TransformationPipeline => {
            pipeline(&mut config, &answers, &sources, &targets)?
        }
        ProjectBootstrapPreset::SimpleConversion => {
            conversion(&mut config, &answers, &sources, &targets)?
        }
        ProjectBootstrapPreset::SchemaValidation => validation(&mut config, &sources)?,
        ProjectBootstrapPreset::Custom => {
            for (i, path) in sources.iter().enumerate() {
                let (content_type, schema, text) = stub(path);
                let mut item = resource(
                    &format!("file-{i}"),
                    if path.ends_with(".cemt") {
                        StudioProjectResourceRole::Template
                    } else {
                        StudioProjectResourceRole::Data
                    },
                    path,
                    content_type,
                    schema,
                    text,
                );
                if schema.is_empty() {
                    item.schema = None;
                }
                config.resources.push(item);
            }
        }
    }
    let review = review_configuration(&config)?;
    Ok(json!({"configuration":config,"answers":answers,"review":review}))
}

fn file_lines(text: &str, field: &str) -> Result<Vec<String>, ProjectBootstrapError> {
    let paths: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect();
    if paths.len() > 32 {
        return Err(error(
            "file_limit",
            "Use at most 32 file paths per list",
            Some(field),
        ));
    }
    let mut seen = BTreeSet::new();
    for path in &paths {
        crate::studio_project::validate_resource_path(path, field)?;
        if path == "project.cem" || !seen.insert(path) {
            return Err(error(
                "path_duplicate",
                "File paths must be unique and cannot replace project.cem",
                Some(field),
            ));
        }
    }
    Ok(paths)
}
fn folder(value: Option<&str>, field: &str) -> Result<String, ProjectBootstrapError> {
    let value = value.unwrap_or("").trim();
    crate::studio_project::validate_resource_path(value, field)?;
    Ok(value.into())
}
fn encoded(value: &str) -> String {
    transform_template_encode_cem_attribute_value(value, "bootstrap path").expect("validated path")
}
fn build_entry(
    config: &mut ProjectBootstrapConfiguration,
    kind: StudioProjectEntryKind,
    name: &str,
    argv: Vec<String>,
    artifact: Option<String>,
) -> Result<(), ProjectBootstrapError> {
    let command = CliCommandResource {
        schema: CLI_COMMAND_SCHEMA_URI.into(),
        schema_version: CLI_COMMAND_SCHEMA_VERSION,
        command_schema_version: CLI_COMMAND_GRAMMAR_SCHEMA_VERSION,
        common_version: crate::VERSION.into(),
        binary_name: CLI_COMMAND_BINARY_NAME.into(),
        argv,
    };
    config.resources.push(resource(
        "command",
        StudioProjectResourceRole::RunConfig,
        "commands/build.json",
        CLI_COMMAND_CONTENT_TYPE,
        CLI_COMMAND_SCHEMA_URI,
        &command
            .serialize(CliCommandProjection::Json)
            .map_err(|e| error("command_invalid", e.to_string(), None))?,
    ));
    config.entries.push(StudioProjectEntry {
        id: "build".into(),
        parent_id: None,
        kind,
        name: name.into(),
        description: None,
        run_config_resource_id: Some("command".into()),
        entry_artifact: artifact,
        resource_ids: config.resources.iter().map(|r| r.id.clone()).collect(),
        tags: vec![],
    });
    Ok(())
}
fn graph_command(
    config: &mut ProjectBootstrapConfiguration,
    name: &str,
    graph: String,
    artifact: String,
) -> Result<(), ProjectBootstrapError> {
    config.resources.push(resource(
        "pipeline",
        StudioProjectResourceRole::Graph,
        "pipelines/build.cem",
        "application/cem",
        TRANSFORM_CONFIG_SCHEMA_URI,
        &graph,
    ));
    build_entry(
        config,
        StudioProjectEntryKind::TransformationGraph,
        name,
        vec![
            "transform".into(),
            "--config".into(),
            ".resources/pipeline".into(),
        ],
        Some(artifact),
    )
}
fn paired(sources: &[String], targets: &[String]) -> Result<(), ProjectBootstrapError> {
    if sources.len() != targets.len() {
        return Err(error(
            "target_count",
            "Provide one target path for each source, in the same order",
            Some("targetFiles"),
        ));
    }
    Ok(())
}
fn pipeline(
    config: &mut ProjectBootstrapConfiguration,
    answers: &ProjectBootstrapAnswers,
    sources: &[String],
    targets: &[String],
) -> Result<(), ProjectBootstrapError> {
    paired(sources, targets)?;
    let templates = folder(answers.layout_folder.as_deref(), "layoutFolder")?;
    config.resources.push(resource(
        "template",
        StudioProjectResourceRole::Template,
        &format!("{templates}/page.cemt"),
        "application/vnd.cem.template+cem",
        "https://cem.dev/ns/template/cem-native/1",
        PIPELINE_TEMPLATE,
    ));
    let mut graph = String::from("@doc cem-ml 1\n{run |\n");
    for (i, (source, target)) in sources.iter().zip(targets).enumerate() {
        if !source.ends_with(".xml") || !target.ends_with(".html") {
            return Err(error(
                "pipeline_format",
                "This template starter pairs .xml sources with .html targets",
                Some("sourceFiles"),
            ));
        }
        config.resources.push(resource(
            &format!("input-{i}"),
            StudioProjectResourceRole::Data,
            source,
            "application/xml",
            "https://cem.dev/ns/data/xml/1",
            "<message>Hello from CEM</message>\n",
        ));
        graph.push_str(&format!("{{import @id=source-{i} @src=input-{i} @content-type=application/xml @schema=https://cem.dev/ns/data/xml/1 | {{transform @id=view-{i} @src=template @entrypoint=main @template-content-type=application/vnd.cem.template+cem @template-schema=https://cem.dev/ns/template/cem-native/1 | {{export @id=output-{i} @out={} @content-type=text/html @schema=https://cem.dev/ns/data/html/1}}}}}}\n",encoded(&format!("../{target}"))));
    }
    graph.push_str("}\n");
    graph_command(config, "Build pipeline", graph, targets[0].clone())
}
fn conversion(
    config: &mut ProjectBootstrapConfiguration,
    answers: &ProjectBootstrapAnswers,
    sources: &[String],
    targets: &[String],
) -> Result<(), ProjectBootstrapError> {
    paired(sources, targets)?;
    if sources.len() != 1 {
        return Err(error("conversion_single","Simple conversion uses exactly one source and one target; choose a pipeline for more files",Some("sourceFiles")));
    }
    let (kind, schema, text) = match answers.input_format.as_deref() {
        Some("yaml") => (
            "application/yaml",
            "https://cem.dev/ns/data/yaml/1",
            "message: Hello from CEM\n",
        ),
        Some("json") => (
            "application/json",
            "https://cem.dev/ns/data/json/1",
            "{\"message\":\"Hello from CEM\"}\n",
        ),
        _ => {
            return Err(error(
                "input_format",
                "Choose YAML or JSON",
                Some("inputFormat"),
            ))
        }
    };
    config.resources.push(resource(
        "source",
        StudioProjectResourceRole::Data,
        &sources[0],
        kind,
        schema,
        text,
    ));
    let (target_type, target_schema) = match answers.output_format.as_deref() {
        Some("cem") => ("application/cem", "https://cem.dev/ns/cem-ml/1"),
        Some("json") => ("application/json", "https://cem.dev/ns/data/json/1"),
        _ => {
            return Err(error(
                "output_format",
                "Choose CEM-ML or JSON",
                Some("outputFormat"),
            ))
        }
    };
    let mut argv: Vec<String> = vec![
        "convert",
        ".resources/source",
        "--to-content-type",
        target_type,
        "--to-schema",
        target_schema,
        "--output-color-type",
        "none",
        "--out",
        &targets[0],
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    if target_type == "application/cem" {
        argv.push("--tabular".into());
    }
    build_entry(
        config,
        StudioProjectEntryKind::Conversion,
        "Convert source",
        argv,
        Some(targets[0].clone()),
    )
}
fn validation(
    config: &mut ProjectBootstrapConfiguration,
    sources: &[String],
) -> Result<(), ProjectBootstrapError> {
    let seed = preset_configuration(
        ProjectBootstrapPreset::SchemaValidation,
        config.project_id.clone(),
        config.name.clone(),
        config.created_at.clone(),
    )?;
    let original = &seed.resources[0];
    let mut argv = vec!["validate".into()];
    for (i, path) in sources.iter().enumerate() {
        if !path.ends_with(".xhtml") {
            return Err(error(
                "validation_format",
                "Use .xhtml source paths for the built-in XHTML validation starter",
                Some("sourceFiles"),
            ));
        }
        let id = format!("source-{i}");
        config.resources.push(resource(
            &id,
            StudioProjectResourceRole::Data,
            path,
            &original.content_type,
            original.schema.as_deref().unwrap(),
            &original.text,
        ));
        argv.push(format!(".resources/{id}"));
    }
    argv.extend(["--fail-level".into(), "strict".into()]);
    build_entry(
        config,
        StudioProjectEntryKind::Validation,
        "Validate documents",
        argv,
        None,
    )
}

fn stub(path: &str) -> (&'static str, &'static str, &'static str) {
    if path.ends_with(".cemt") {
        (
            "application/vnd.cem.template+cem",
            "https://cem.dev/ns/template/cem-native/1",
            PIPELINE_TEMPLATE,
        )
    } else if path.ends_with(".cem") {
        (
            "application/cem",
            "https://cem.dev/ns/cem-ml/1",
            "@doc cem-ml 1\n{message | ```Hello from CEM```}\n",
        )
    } else if path.ends_with(".md") {
        (
            "text/markdown",
            "https://cem.dev/ns/data/markdown/1",
            "# Project notes\n\nAdd your project documentation here.\n",
        )
    } else if path.ends_with(".json") {
        (
            "application/json",
            "https://cem.dev/ns/data/json/1",
            "{\"message\":\"Hello from CEM\"}\n",
        )
    } else if path.ends_with(".yaml") || path.ends_with(".yml") {
        (
            "application/yaml",
            "https://cem.dev/ns/data/yaml/1",
            "message: Hello from CEM\n",
        )
    } else if path.ends_with(".xml") {
        (
            "application/xml",
            "https://cem.dev/ns/data/xml/1",
            "<message>Hello from CEM</message>\n",
        )
    } else if path.ends_with(".css") {
        ("text/css", "https://cem.dev/ns/data/css/1", SITE_CSS)
    } else if path.ends_with(".svg") {
        ("image/svg+xml", "https://cem.dev/ns/data/svg/1", LOGO)
    } else {
        ("text/plain", "", "Add your content here.\n")
    }
}

fn site(
    config: &mut ProjectBootstrapConfiguration,
    answers: &ProjectBootstrapAnswers,
    pages: &[String],
) -> Result<(), ProjectBootstrapError> {
    let source_folder = folder(answers.source_folder.as_deref(), "sourceFolder")?;
    let output_folder = folder(answers.output_folder.as_deref(), "outputFolder")?;
    let layout_folder = folder(answers.layout_folder.as_deref(), "layoutFolder")?;
    let css_folder = folder(answers.css_folder.as_deref(), "cssFolder")?;
    let static_folder = folder(answers.static_folder.as_deref(), "staticFolder")?;
    let mut navigation = vec![];
    let mut graph=String::from("@doc cem-ml 1\n{run |\n{import @id=site @src=site @content-type=application/json @schema=https://cem.dev/ns/data/json/1}\n");
    for (i, page) in pages.iter().enumerate() {
        let Some(stem) = page.strip_suffix(".md") else {
            return Err(error(
                "page_format",
                "Site page paths must end with .md",
                Some("sourceFiles"),
            ));
        };
        let title = stem.rsplit('/').next().unwrap().replace(['-', '_'], " ");
        let title = if title == "index" {
            "Home".into()
        } else {
            title
        };
        let href = format!("{stem}.html");
        navigation.push(json!({"label":title,"href":href}));
        let depth = page.split('/').count() - 1;
        let base = if depth == 0 {
            ".".into()
        } else {
            vec![".."; depth].join("/")
        };
        let text = format!(
            "---\ntitle: {}\ndescription: {}\nbase: {}\n---\n# {title}\n\nAdd your content here.\n",
            serde_json::to_string(&title).unwrap(),
            serde_json::to_string(&format!("{title} page")).unwrap(),
            serde_json::to_string(&base).unwrap()
        );
        config.resources.push(resource(
            &format!("page-{i}"),
            StudioProjectResourceRole::Data,
            &format!("{source_folder}/{page}"),
            "text/markdown; charset=utf-8; variant=CommonMark; front-matter=yaml",
            "https://cem.dev/ns/data/markdown/1",
            &text,
        ));
        graph.push_str(&format!("{{import @id=page-{i} @src=page-{i} @content-type=\"text/markdown; charset=utf-8; variant=CommonMark; front-matter=yaml\" @schema=https://cem.dev/ns/data/markdown/1 | {{convert @id=html-{i} @content-type=text/html @schema=https://cem.dev/ns/data/html/1 @converter=markdown-to-html-rust | {{convert @id=dom-{i} @content-type=application/vnd.cem.dom+cem-bin @schema=https://cem.dev/ns/projection/dom/1 @converter=html-to-cem-dom-projection-rust | {{transform @id=layout-{i} @src=layout @entrypoint=page @with:site=site @template-content-type=application/vnd.cem.transform+cem @template-schema=https://cem.dev/ns/transform/cem/1 | {{export @id=export-{i} @out={} @content-type=text/html @schema=https://cem.dev/ns/data/html/1}}}}}}}}}}\n",encoded(&format!("../{output_folder}/{href}"))));
    }
    config.resources.push(resource("site",StudioProjectResourceRole::Data,"data/site.json","application/json","https://cem.dev/ns/data/json/1",&format!("{}\n",json!({"title":config.name,"navigation":navigation,"cssPath":format!("{css_folder}/site.css")}))));
    for (id, path, text) in [
        ("layout", "page.cemt", include_str!("assets/page.cemt")),
        ("chrome", "chrome.cemt", include_str!("assets/chrome.cemt")),
        (
            "content",
            "content.cemt",
            include_str!("assets/content.cemt"),
        ),
    ] {
        config.resources.push(resource(
            id,
            StudioProjectResourceRole::Template,
            &format!("{layout_folder}/{path}"),
            "application/vnd.cem.transform+cem",
            "https://cem.dev/ns/transform/cem/1",
            text,
        ));
    }
    for (id, path, kind, schema, text) in [
        (
            "stylesheet",
            format!("{css_folder}/site.css"),
            "text/css",
            "https://cem.dev/ns/data/css/1",
            SITE_CSS,
        ),
        (
            "logo",
            format!("{static_folder}/logo.svg"),
            "image/svg+xml",
            "https://cem.dev/ns/data/svg/1",
            LOGO,
        ),
    ] {
        config.resources.push(resource(
            id,
            StudioProjectResourceRole::Data,
            &path,
            kind,
            schema,
            text,
        ));
        graph.push_str(&format!(
            "{{import @id={id} @src={id} @content-type={} @opaque=true | {{export @id={id}-copy @out={}}}}}\n",
            encoded(kind),
            encoded(&format!("../{output_folder}/{path}"))
        ));
    }
    graph.push_str("}\n");
    let first = pages[0].strip_suffix(".md").unwrap();
    graph_command(
        config,
        "Build site",
        graph,
        format!("{output_folder}/{first}.html"),
    )
}
const SITE_CSS:&str=":root { color-scheme: light dark; }\nbody { max-inline-size: 72rem; margin: auto; padding: 1.5rem; font-family: system-ui, sans-serif; background: Canvas; color: CanvasText; }\nnav { display: flex; flex-wrap: wrap; gap: 1rem; }\na { color: LinkText; }\nimg { max-inline-size: 100%; block-size: auto; }\n:focus-visible { outline: 2px solid Highlight; outline-offset: 3px; }\n";
const LOGO:&str="<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 64 64\"><title>Site mark</title><path fill=\"currentColor\" d=\"M8 8h48v48H8z\"/></svg>\n";
