use cem_ml::{
    command_service::VirtualResourceV1,
    engine::{CemMlEngine, EngineContext, FormatIdentity},
    real::RealCemMlEngine,
    transform_config::{
        parse_transform_graph_config, TransformGraphParseRequest, TRANSFORM_CONFIG_SCHEMA_URI,
    },
    transform_graph_request::{
        lower_transform_graph_request, transform_graph_reference_matches,
        ManifestTransformGraphResourceProvider,
    },
};
use std::collections::BTreeMap;

const CONFIG: &str = "studio://site/graphs/build.cem";
fn parse(source: &str) -> cem_ml::transform_config::TransformGraphConfig {
    let response = parse_transform_graph_config(TransformGraphParseRequest {
        bytes: source.as_bytes().to_vec(),
        identity: FormatIdentity {
            content_type: Some("application/cem".into()),
            schema: Some(TRANSFORM_CONFIG_SCHEMA_URI.into()),
            ..Default::default()
        },
        base_uri: Some(CONFIG.into()),
    })
    .unwrap();
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    response.graph
}
fn resource(bytes: &[u8], content_type: Option<&str>) -> VirtualResourceV1 {
    VirtualResourceV1 {
        bytes: bytes.to_vec(),
        identity: content_type.map(|value| FormatIdentity {
            content_type: Some(value.into()),
            ..Default::default()
        }),
    }
}
fn assets() -> BTreeMap<String, VirtualResourceV1> {
    BTreeMap::from([
        (
            "studio://site/static/site.css".into(),
            resource(b"/* exact */\r\nh1 {color: red}", None),
        ),
        (
            "studio://site/static/images/logo.svg".into(),
            resource(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>", None),
        ),
        (
            "studio://site/static/images/pixel.png".into(),
            resource(&[137, 80, 78, 71, 0, 255], None),
        ),
        (
            "studio://site/static/download".into(),
            resource(&[0, 255, 128, 13, 10], Some("application/x-demo")),
        ),
        (
            "studio://site/static/mystery.bin".into(),
            resource(&[0, 254], None),
        ),
    ])
}
const STATIC: &str = r#"{run | {import @id=static @src="../static/**/*" @root="../static" @opaque=true |
    {export @id=publish @out="../dist/{path}"}
}}"#;

#[test]
fn recursive_folder_pattern_includes_root_and_nested_files() {
    use cem_ml::transform_graph_request::resolve_transform_graph_reference;
    assert_eq!(
        resolve_transform_graph_reference("build.cem", "../static/site.css"),
        "../static/site.css"
    );
    for candidate in [
        "studio://site/content/home.md",
        "studio://site/content/posts/intro.md",
        "studio://site/content/posts/deep/next.md",
    ] {
        assert!(
            transform_graph_reference_matches("studio://site/content/**/*.md", candidate).unwrap(),
            "{candidate}"
        );
    }
    assert!(!transform_graph_reference_matches(
        "studio://site/content/**/*.md",
        "studio://site/content-other/home.md"
    )
    .unwrap());
    assert!(!transform_graph_reference_matches(
        "studio://site/content/*.md",
        "studio://site/content/posts/intro.md"
    )
    .unwrap());
}

#[test]
fn rooted_static_tree_preserves_exact_bytes_and_each_resource_media_type() {
    let graph = parse(STATIC);
    let resources = assets();
    let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
    let request =
        lower_transform_graph_request(&EngineContext::default(), &graph, &provider, CONFIG, true)
            .unwrap();
    assert_eq!(request.imports.len(), 5);
    let expected = [
        "application/x-demo",
        "image/svg+xml",
        "image/png",
        "application/octet-stream",
        "text/css",
    ];
    for ((input, output), content_type) in
        request.imports.iter().zip(&request.exports).zip(expected)
    {
        let relative = input
            .input
            .uri
            .strip_prefix("studio://site/static/")
            .unwrap();
        assert_eq!(
            output.destination.as_deref(),
            Some(format!("studio://site/dist/{relative}").as_str())
        );
        assert_eq!(input.input.bytes, resources[&input.input.uri].bytes);
        assert_eq!(
            input
                .input
                .identity
                .as_ref()
                .unwrap()
                .content_type
                .as_deref(),
            Some(content_type)
        );
        assert_eq!(
            output.target.as_ref().unwrap().content_type.as_deref(),
            Some(content_type)
        );
    }
    let response = RealCemMlEngine::new().transform_graph(request).unwrap();
    assert!(
        response.diagnostics.is_empty(),
        "{:?}",
        response.diagnostics
    );
    assert_eq!(response.artifacts.len(), 5);
    for artifact in response.artifacts {
        let source = artifact
            .destination
            .as_ref()
            .unwrap()
            .replace("/dist/", "/static/");
        assert_eq!(
            artifact.primary_bytes.unwrap().bytes,
            resources[&source].bytes
        );
        // Opaque bytes have whole-resource identity; they do not claim parsed character lineage.
        assert!(artifact.source_map.is_none());
        assert!(artifact.output_spans.is_empty());
    }
}

#[test]
fn rooted_parsed_folder_keeps_source_identity_and_exposes_folder_metadata_to_collect() {
    let graph = parse(
        r#"{run | {import @id=content @src="../content/**/*.cem" @root="../content" @content-type=application/cem |
        {join @id=site @mode=collect | {export @id=manifest @out="../dist/site.json" @content-type=application/json}}
    }}"#,
    );
    let resources = BTreeMap::from([
        (
            "studio://site/content/home.cem".into(),
            resource(b"{home}", None),
        ),
        (
            "studio://site/content/posts/intro.cem".into(),
            resource(b"{intro}", None),
        ),
    ]);
    let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
    let request =
        lower_transform_graph_request(&EngineContext::default(), &graph, &provider, CONFIG, true)
            .unwrap();
    let items = &request.joins[0].inputs;
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].bindings["path"], "home.cem");
    assert_eq!(items[0].bindings["dir"], "");
    assert_eq!(items[1].bindings["path"], "posts/intro.cem");
    assert_eq!(items[1].bindings["dir"], "posts");
    assert_eq!(items[1].bindings["stem"], "intro");
    assert_eq!(
        items[1].bindings["src"],
        "studio://site/content/posts/intro.cem"
    );
    assert_eq!(request.imports[1].input.uri, items[1].bindings["src"]);
}

#[test]
fn folder_root_rejects_foreign_resources_and_invalid_roots() {
    for (source, code) in [
        (
            STATIC.replace("@root=\"../static\"", "@root=\"../other\""),
            "cem.transform_config.import_root_outside",
        ),
        (
            STATIC.replace("@root=\"../static\"", "@root=\"\""),
            "cem.transform_config.import_root_invalid",
        ),
        (
            STATIC.replace("@root=\"../static\"", "@root=\"../sta*\""),
            "cem.transform_config.import_root_invalid",
        ),
    ] {
        let parsed = parse_transform_graph_config(TransformGraphParseRequest {
            bytes: source.as_bytes().to_vec(),
            identity: FormatIdentity {
                content_type: Some("application/cem".into()),
                ..Default::default()
            },
            base_uri: Some(CONFIG.into()),
        })
        .unwrap();
        let resources = assets();
        let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
        let result = lower_transform_graph_request(
            &EngineContext::default(),
            &parsed.graph,
            &provider,
            CONFIG,
            true,
        );
        assert!(
            parsed.diagnostics.iter().any(|d| d.code == code)
                || result.as_ref().is_err_and(|error| error.code() == code),
            "{source}: {result:?}"
        );
    }
}

#[test]
fn folder_imports_are_nonempty_and_bounded() {
    let graph = parse(STATIC);
    for count in [0, 1025] {
        let resources = (0..count)
            .map(|i| {
                (
                    format!("studio://site/static/{i}.bin"),
                    resource(b"x", None),
                )
            })
            .collect();
        let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
        let error = lower_transform_graph_request(
            &EngineContext::default(),
            &graph,
            &provider,
            CONFIG,
            false,
        )
        .unwrap_err();
        assert_eq!(
            error.code(),
            if count == 0 {
                "cem.transform_config.import_glob_empty"
            } else {
                "cem.transform_config.import_glob_too_many"
            }
        );
    }
}

#[test]
fn filesystem_and_manifest_rooted_folder_builds_have_identical_bindings_and_outputs() {
    use cem_ml::transform_graph_request::FilesystemTransformGraphResourceProvider;
    let root = std::env::temp_dir().join(format!("cem-rooted-build-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("graphs")).unwrap();
    std::fs::create_dir_all(root.join("static/images")).unwrap();
    let config = root.join("build.cem").to_string_lossy().into_owned();
    let mut resources = BTreeMap::new();
    for (relative, bytes) in [
        ("site.css", b"h1 {color:red}".as_slice()),
        ("images/pixel.png", &[137, 80, 78, 71, 0, 255]),
    ] {
        let path = root.join("static").join(relative);
        std::fs::write(&path, bytes).unwrap();
        resources.insert(path.to_string_lossy().into_owned(), resource(bytes, None));
    }
    let context = EngineContext::default();
    let graph = parse(
        &STATIC
            .replace("../static", "static")
            .replace("../dist", "dist"),
    );
    let manifest = ManifestTransformGraphResourceProvider::new(&config, &resources);
    let filesystem = FilesystemTransformGraphResourceProvider::new(&context, &config);
    let a = lower_transform_graph_request(&context, &graph, &manifest, &config, true).unwrap();
    let b = lower_transform_graph_request(&context, &graph, &filesystem, &config, true).unwrap();
    assert_eq!(format!("{:?}", a.imports), format!("{:?}", b.imports));
    assert_eq!(format!("{:?}", a.exports), format!("{:?}", b.exports));
    assert_eq!(a.exports.len(), 2);
    assert_eq!(
        a.exports[1].destination.as_deref(),
        root.join("dist/site.css").to_str()
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn recursive_filesystem_folder_walk_stops_at_depth_limit() {
    use cem_ml::transform_graph_request::FilesystemTransformGraphResourceProvider;
    let root = std::env::temp_dir().join(format!("cem-rooted-depth-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut deep = root.join("static");
    for _ in 0..66 {
        deep.push("d");
    }
    std::fs::create_dir_all(&deep).unwrap();
    std::fs::write(deep.join("asset.bin"), b"x").unwrap();
    let config = root.join("build.cem").to_string_lossy().into_owned();
    let graph = parse(&STATIC.replace("../static", "static"));
    let context = EngineContext::default();
    let provider = FilesystemTransformGraphResourceProvider::new(&context, &config);
    let result = lower_transform_graph_request(&context, &graph, &provider, &config, false);
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(
        result.err().map(|error| error.code().to_owned()).as_deref(),
        Some("cem.transform_config.import_glob_traversal_limit")
    );
}

#[test]
fn rooted_folder_rejects_ambiguous_output_collisions_and_media_type_changes() {
    let resources = assets();
    let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
    let graph = parse(&STATIC.replace("../dist/{path}", "../dist/one.bin"));
    let request =
        lower_transform_graph_request(&EngineContext::default(), &graph, &provider, CONFIG, false)
            .unwrap();
    assert!(
        cem_ml::engine::validate_transform_graph_runtime_contract(&request)
            .iter()
            .any(|d| d.code == "cem.transform_runtime.duplicate_destination")
    );
    let graph = parse(&STATIC.replace("@out=", "@content-type=text/plain @out="));
    let result =
        lower_transform_graph_request(&EngineContext::default(), &graph, &provider, CONFIG, false);
    assert_eq!(
        result.err().map(|error| error.code().to_owned()).as_deref(),
        Some("cem.transform_config.import_opaque_export_identity_mismatch")
    );
}

#[test]
fn rooted_folder_cancellation_and_unsafe_paths_do_not_produce_requests() {
    let resources = assets();
    let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
    let context = EngineContext::default();
    context.abort_signal().abort();
    assert_eq!(
        lower_transform_graph_request(&context, &parse(STATIC), &provider, CONFIG, false)
            .unwrap_err()
            .code(),
        "cem.operation.cancelled"
    );
    for path in [
        "%2e%2e/escape.bin",
        "images%2flogo.svg",
        "image\\logo.svg",
        "asset.bin?query",
    ] {
        let resources =
            BTreeMap::from([(format!("studio://site/static/{path}"), resource(b"x", None))]);
        let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
        let result = lower_transform_graph_request(
            &EngineContext::default(),
            &parse(STATIC),
            &provider,
            CONFIG,
            false,
        );
        assert_eq!(
            result.err().map(|error| error.code().to_owned()).as_deref(),
            Some("cem.transform_config.import_root_invalid"),
            "{path}"
        );
    }
}

#[test]
fn folder_root_can_be_the_relative_configuration_directory() {
    let config = "build.cem";
    let graph = parse(
        r#"{run | {import @id=content @src="content/**/*.cem" @root="." @content-type=application/cem |
        {join @id=files @mode=collect | {export @id=manifest @out="generated/files.json" @content-type=application/json}}
    }}"#,
    );
    let resources = BTreeMap::from([("content/home.cem".into(), resource(b"{home}", None))]);
    let provider = ManifestTransformGraphResourceProvider::new(config, &resources);
    let request =
        lower_transform_graph_request(&EngineContext::default(), &graph, &provider, config, false)
            .unwrap();
    assert_eq!(
        request.joins[0].inputs[0].bindings["path"],
        "content/home.cem"
    );
    assert_eq!(
        request.exports[0].destination.as_deref(),
        Some("generated/files.json")
    );
}

#[test]
fn native_graph_authoring_exposes_and_validates_the_folder_root() {
    use cem_ml::transform_config_authoring::{
        author_transform_config, GraphAuthoringEdit, GraphAuthoringRequest,
    };
    let request = |value: Option<&str>| GraphAuthoringRequest {
        source: STATIC.into(),
        edit: value.map(|value| GraphAuthoringEdit {
            node: "static".into(),
            attribute: "root".into(),
            value: Some(value.into()),
        }),
        create: None,
        edge: None,
        parameter: None,
    };
    let described = author_transform_config(request(None)).unwrap();
    assert_eq!(
        described.nodes[0]
            .fields
            .iter()
            .find(|field| field.id == "root")
            .unwrap()
            .value
            .as_deref(),
        Some("../static")
    );
    let edited = author_transform_config(request(Some("../assets"))).unwrap();
    assert_eq!(edited.graph.nodes[0].root.as_deref(), Some("../assets"));
    assert!(author_transform_config(request(Some(""))).is_err());
}

#[test]
fn native_markdown_conversion_retains_folder_paths_through_export() {
    let graph = parse(
        r#"{run | {import @id=content @src="../content/**/*.md" @root="../content"
        @content-type="text/markdown; charset=utf-8; variant=CommonMark" @schema=https://cem.dev/ns/data/markdown/1 |
        {convert @id=html @content-type=text/html @schema=https://cem.dev/ns/data/html/1 @converter=markdown-to-html-rust |
            {export @id=pages @out="../generated/{dir}/{stem}.html" @content-type=text/html}}
    }}"#,
    );
    let resources = BTreeMap::from([
        (
            "studio://site/content/index.md".into(),
            resource(b"# Home\n", None),
        ),
        (
            "studio://site/content/posts/intro.md".into(),
            resource(b"# Nested\n", None),
        ),
    ]);
    let provider = ManifestTransformGraphResourceProvider::new(CONFIG, &resources);
    let request =
        lower_transform_graph_request(&EngineContext::default(), &graph, &provider, CONFIG, true)
            .unwrap();
    assert_eq!(request.conversions.len(), 2);
    let result = RealCemMlEngine::new().transform_graph(request).unwrap();
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    assert_eq!(result.artifacts.len(), 2);
    for (artifact, (path, title)) in result
        .artifacts
        .iter()
        .zip([("index.html", "Home"), ("posts/intro.html", "Nested")])
    {
        assert_eq!(
            artifact.destination.as_deref(),
            Some(format!("studio://site/generated/{path}").as_str())
        );
        let html = artifact.primary.as_str().expect("native HTML text export");
        assert!(html.starts_with("<h1 "));
        assert!(html.contains(&format!(">{title}</h1>")));
        assert!(artifact.source_map.is_some());
        let uri = format!("studio://site/content/{}", path.replace(".html", ".md"));
        for span in &artifact.output_spans {
            assert!(span.origin.frames.iter().any(|frame| matches!(&frame.transform,
                cem_ml::source_map::TransformKind::SourceBoundary { uri: actual } if actual == &uri)), "{span:?}");
        }
    }
}

#[test]
fn unrooted_filesystem_imports_preserve_legacy_parent_relative_bindings() {
    use cem_ml::transform_graph_request::FilesystemTransformGraphResourceProvider;
    let root = std::env::temp_dir().join(format!("cem-unrooted-build-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("content")).unwrap();
    std::fs::create_dir_all(root.join("graphs")).unwrap();
    std::fs::write(root.join("content/a.cem"), b"{a}").unwrap();
    let config = root.join("graphs/build.cem").to_string_lossy().into_owned();
    let graph = parse(
        r#"{run | {import @id=content @src="../content/*.cem" @content-type=application/cem |
        {join @id=files @mode=collect | {export @id=manifest @out="../generated/files.json" @content-type=application/json}}
    }}"#,
    );
    let context = EngineContext::default();
    let provider = FilesystemTransformGraphResourceProvider::new(&context, &config);
    let result =
        lower_transform_graph_request(&context, &graph, &provider, &config, false).unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert_eq!(
        result.joins[0].inputs[0].bindings["path"],
        "../content/a.cem"
    );
}

#[test]
fn rooted_windows_filesystem_addresses_keep_identity_and_use_portable_bindings() {
    use cem_ml::transform_graph_request::{
        TransformGraphRequestError, TransformGraphResource, TransformGraphResourceProvider,
    };
    struct WindowsProvider;
    impl TransformGraphResourceProvider for WindowsProvider {
        fn import_resources(
            &self,
            _: &str,
            _: Option<&str>,
        ) -> Result<Vec<TransformGraphResource>, TransformGraphRequestError> {
            Ok(vec![TransformGraphResource {
                uri: r"C:\site\static\images\logo.svg".into(),
                bytes: b"<svg/>".to_vec(),
                identity: None,
                content_type: None,
            }])
        }
        fn read_resource(
            &self,
            _: &str,
            _: cem_ml::resolver::ResolvePurpose,
            _: Option<&str>,
        ) -> Result<TransformGraphResource, TransformGraphRequestError> {
            unreachable!()
        }
        fn output_uri(&self, reference: &str) -> Result<String, TransformGraphRequestError> {
            Ok(if reference == "../static" {
                r"C:\site\graphs\..\static".into()
            } else {
                format!("C:/site/graphs/{reference}")
            })
        }
        fn binding_uri(&self, _: &str) -> String {
            unreachable!()
        }
    }
    let request = lower_transform_graph_request(
        &EngineContext::default(),
        &parse(STATIC),
        &WindowsProvider,
        "C:/site/graphs/build.cem",
        false,
    )
    .unwrap();
    assert_eq!(
        request.imports[0].input.uri,
        r"C:\site\static\images\logo.svg"
    );
    assert_eq!(
        request.exports[0].destination.as_deref(),
        Some("C:/site/graphs/../dist/images/logo.svg")
    );
}

#[test]
fn filesystem_folder_outputs_detect_empty_directory_binding_collisions() {
    use cem_ml::transform_graph_request::FilesystemTransformGraphResourceProvider;
    let root = std::env::temp_dir().join(format!("cem-folder-collision-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for directory in ["content", "static"] {
        std::fs::create_dir_all(root.join(directory)).unwrap();
        std::fs::write(root.join(directory).join("index.html"), b"<h1>Index</h1>").unwrap();
    }
    let config = root.join("build.cem").to_string_lossy().into_owned();
    let graph = parse(
        r#"{run |
        {import @id=content @src="content/*.html" @root="content" @content-type=text/html @opaque=true |
            {export @id=pages @out="generated/{dir}/{stem}.html"}}
        {import @id=static @src="static/*" @root="static" @opaque=true |
            {export @id=assets @out="generated/{path}"}}
    }"#,
    );
    let context = EngineContext::default();
    let provider = FilesystemTransformGraphResourceProvider::new(&context, &config);
    let result =
        lower_transform_graph_request(&context, &graph, &provider, &config, false).unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert!(
        cem_ml::engine::validate_transform_graph_runtime_contract(&result)
            .iter()
            .any(|diagnostic| diagnostic.code == "cem.transform_runtime.duplicate_destination")
    );
}
