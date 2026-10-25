//! The ordinary projection project uses native owners and explicit export boundaries.
use cem_ml::{
    diagnostics::{Diagnostic, Severity},
    engine::{
        CemMlEngine, CheckRequest, EngineContext, EngineInput, FailLevel, FormatIdentity,
        InputFormat, SourceValidationStage, ValidateProjection, ValidateRequest,
    },
    events::cem::CemEventNormalizer,
    parser::builder::CemAstBuilder,
    projection::{
        ast_binary_projection_artifact, dom_binary_projection_artifact,
        events_binary_projection_artifact_as, BinaryProjectionArtifact,
    },
    real::RealCemMlEngine,
    source::{BytesSource, SourceId},
    tokenizer::cem::CemTokenizer,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "cem-native-projections-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }

    fn run(&self, args: &[String]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap()
    }

    fn validate(&self, path: &Path, family: &str, encoding: &str) -> Output {
        self.run(&[
            "validate".into(),
            path.to_string_lossy().into_owned(),
            "--content-type".into(),
            format!("application/vnd.cem.{family}+{encoding}"),
            "--schema".into(),
            format!("https://cem.dev/ns/projection/{family}/1"),
            "--format".into(),
            "json".into(),
            "--no-color".into(),
        ])
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

struct Project {
    directory: Directory,
    definition: Value,
}
impl Project {
    fn new() -> Self {
        let definition: Value = serde_json::from_slice(
            &fs::read(workspace().join("examples/projects/native-projections/example.json"))
                .unwrap(),
        )
        .unwrap();
        let directory = Directory::new();
        for source in definition["sources"].as_array().unwrap() {
            let path = directory.0.join(source["path"].as_str().unwrap());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::copy(workspace().join(source["source"].as_str().unwrap()), path).unwrap();
        }
        Self {
            directory,
            definition,
        }
    }

    fn source(&self, id: &str) -> &Value {
        self.definition["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|source| source["id"] == id)
            .unwrap_or_else(|| panic!("missing authored resource {id}"))
    }

    fn path(&self, id: &str) -> PathBuf {
        self.directory
            .0
            .join(self.source(id)["path"].as_str().unwrap())
    }

    fn read(&self, id: &str) -> Vec<u8> {
        fs::read(self.path(id)).unwrap()
    }

    fn configs(&self) -> &[Value] {
        self.definition["configurations"].as_array().unwrap()
    }

    fn config(&self, id: &str) -> &Value {
        self.configs()
            .iter()
            .find(|config| config["id"] == id)
            .unwrap_or_else(|| panic!("missing authored configuration {id}"))
    }

    fn run(&self, config: &Value) -> Output {
        let args = config["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| {
                let value = arg.as_str().unwrap();
                match if value == "$source" {
                    config["source"].as_str()
                } else {
                    value.strip_prefix('$')
                } {
                    Some(id) => self.path(id).to_string_lossy().into_owned(),
                    None => value.to_owned(),
                }
            })
            .collect::<Vec<_>>();
        self.directory.run(&args)
    }
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let errors = String::from_utf8_lossy(&output.stderr);
    assert!(
        !errors.contains("error:")
            && !errors.contains("fatal:")
            && !errors.contains("cem.lifecycle.input_adapter_unsupported"),
        "a successful example must not conceal source-routing failures: {errors}"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("@kind=error"));
}

fn report(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "invalid native report {error}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn same_path(uri: &str, path: &Path) {
    assert_eq!(
        fs::canonicalize(uri.strip_prefix("file://").unwrap_or(uri)).unwrap(),
        fs::canonicalize(path).unwrap()
    );
}

#[test]
fn authored_projection_configurations_preserve_native_bytes_and_diagnostics() {
    let project = Project::new();
    assert!(project.configs().len() >= 18);
    for config in project.configs() {
        let output = project.run(config);
        let expected_exit = config["expectedExit"].as_i64().unwrap_or(0) as i32;
        assert_eq!(
            output.status.code(),
            Some(expected_exit),
            "{}: {} {}",
            config["id"],
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if expected_exit == 0 {
            success(&output);
        }
        for key in ["expectedOutput", "expectedOutputBytes"] {
            if let Some(expected) = config[key].as_str() {
                assert_eq!(output.stdout, project.read(expected), "{}", config["id"]);
            }
        }
        for text in config["expectedText"].as_array().into_iter().flatten() {
            assert!(
                String::from_utf8_lossy(&output.stdout).contains(text.as_str().unwrap()),
                "{} misses {text}",
                config["id"]
            );
        }
        if let Some(expected) = config["expectedValidation"].as_object() {
            let result = report(&output);
            for key in ["errorCount", "warningCount"] {
                assert_eq!(result["summary"][key], expected[key]);
            }
            assert_eq!(result["summary"]["fatalCount"], json!(0));
        }
    }
}

#[test]
fn original_projection_shape_failures_repair_without_rewriting_catalog_sources() {
    let definition = Project::new();
    let repairs = definition
        .configs()
        .iter()
        .filter(|config| config["repair"].is_object())
        .collect::<Vec<_>>();
    assert_eq!(repairs.len(), 4);
    for config in repairs {
        let project = Project::new();
        let source = config["source"].as_str().unwrap();
        let original = String::from_utf8(project.read(source)).unwrap();
        let output = project.run(config);
        assert_eq!(output.status.code(), Some(1));
        let result = report(&output);
        let expected = &config["expectedValidation"];
        let diagnostic = result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .find(|diagnostic| diagnostic["code"] == expected["errorCode"])
            .expect("the original native diagnostic");
        assert_eq!(diagnostic["severity"], "error");
        same_path(diagnostic["uri"].as_str().unwrap(), &project.path(source));
        for (key, value) in expected["location"].as_object().into_iter().flatten() {
            assert_eq!(&diagnostic[key], value, "{} / {key}", config["id"]);
        }
        let repaired = original.replacen(
            config["repair"]["find"].as_str().unwrap(),
            config["repair"]["replace"].as_str().unwrap(),
            1,
        );
        assert_ne!(repaired, original);
        fs::write(project.path(source), repaired).unwrap();
        let fixed = project.run(config);
        success(&fixed);
        assert_eq!(report(&fixed)["summary"]["hardViolationCount"], 0);
        assert_eq!(
            fs::read_to_string(
                workspace().join(project.source(source)["source"].as_str().unwrap())
            )
            .unwrap(),
            original
        );
    }
}

fn native_artifacts(source: &[u8]) -> Vec<BinaryProjectionArtifact> {
    let document = CemAstBuilder::new(CemEventNormalizer::new(CemTokenizer::from_source(
        BytesSource::new(SourceId(1), source.to_vec()),
    )))
    .build();
    assert!(!document
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity.is_hard_violation()));
    vec![
        dom_binary_projection_artifact(&document),
        ast_binary_projection_artifact(&document),
        events_binary_projection_artifact_as(source, InputFormat::Cem),
    ]
}

#[test]
fn generated_native_chunks_replay_owned_bytes_and_original_utf8_ranges() {
    let source =
        fs::read(workspace().join("examples/projects/native-projections/data/dispatch.cem"))
            .unwrap();
    let label = "Café Noord".as_bytes();
    let offset = source
        .windows(label.len())
        .position(|bytes| bytes == label)
        .unwrap();
    let artifacts = native_artifacts(&source);
    let repeated = native_artifacts(&source);
    assert_eq!(artifacts, repeated);
    let changed = native_artifacts(
        String::from_utf8(source.clone())
            .unwrap()
            .replace("Café Noord", "Café Haven")
            .as_bytes(),
    );
    for (artifact, edited) in artifacts.iter().zip(changed) {
        assert!(artifact.bytes.starts_with(b"CEMPROJ\0"));
        assert_ne!(artifact.hash, edited.hash);
        assert_ne!(artifact.bytes, edited.bytes);
        let stream = artifact.to_chunk_stream();
        assert_eq!(stream.replay_bytes(), artifact.bytes);
        assert_eq!(stream.hash, artifact.hash);
        assert_eq!(stream.byte_length, artifact.bytes.len());
        let mut chunks = stream.chunks().to_vec();
        chunks.sort_by_key(|chunk| chunk.byte_offset);
        let mut next = 0;
        for chunk in &chunks {
            assert_eq!(chunk.byte_offset as usize, next);
            assert_eq!(chunk.bytes().len(), chunk.byte_length);
            assert!(!chunk.hash.is_empty());
            assert_eq!(
                chunk.bytes(),
                &artifact.bytes[next..next + chunk.byte_length]
            );
            next += chunk.byte_length;
            for link in &chunk.child_links {
                assert!(stream.chunk_by_id(&link.chunk_id).is_some());
                assert!(stream.chunk_by_root_id(&link.root_id).is_some());
            }
            for range in &chunk.source_map_deltas {
                let start = range.byte_offset as usize;
                let end = start + range.byte_length as usize;
                assert!(end <= source.len());
                assert!(std::str::from_utf8(&source[start..end]).is_ok());
                if range.source_id == 0 {
                    // Trivia has no tokenizer owner; preserve its native builder origin.
                    assert_eq!(range.transform, "cem-ast-builder");
                    assert!(source[start..end].iter().all(u8::is_ascii_whitespace));
                } else {
                    assert_eq!(range.source_id, 1);
                }
            }
        }
        assert_eq!(next, artifact.bytes.len());
        assert!(chunks
            .iter()
            .flat_map(|chunk| &chunk.source_map_deltas)
            .any(|range| {
                range.byte_offset as usize == offset && range.byte_length as usize == label.len()
            }));
    }
}

#[test]
fn binary_source_validation_checks_headers_without_claiming_payload_decode() {
    let project = Project::new();
    let directory = &project.directory;
    for family in ["dom", "ast", "events"] {
        let id = format!("{family}-binary");
        let original = project.read(&id);
        let path = project.path(&id);
        let valid = directory.validate(&path, family, "cem-bin");
        success(&valid);
        assert_eq!(report(&valid)["summary"]["hardViolationCount"], 0);

        let resource = project.source(&id);
        assert_eq!(
            resource["schema"],
            format!("https://cem.dev/ns/projection/{family}/1")
        );
        assert_eq!(
            resource["contentType"],
            format!("application/vnd.cem.{family}+cem-bin")
        );
        let header_length = 8
            + 2
            + 1
            + 4
            + resource["schema"].as_str().unwrap().len()
            + 4
            + resource["contentType"].as_str().unwrap().len()
            + 4;
        assert!(header_length < original.len());
        // Current validation owns the header only; this does not admit payload replay.
        fs::write(&path, &original[..header_length]).unwrap();
        success(&directory.validate(&path, family, "cem-bin"));

        let mut magic = original.clone();
        magic[0] = b'X';
        let mut version = original.clone();
        version[8..10].copy_from_slice(&255_u16.to_be_bytes());
        let mut kind = original.clone();
        kind[10] = if kind[10] == 3 { 1 } else { kind[10] + 1 };
        for (bytes, suffix) in [
            (magic, "binary_magic"),
            (version, "binary_version"),
            (kind, "projection_mismatch"),
            (original[..10].to_vec(), "binary_truncated"),
        ] {
            fs::write(&path, bytes).unwrap();
            let output = directory.validate(&path, family, "cem-bin");
            assert_eq!(output.status.code(), Some(1));
            let result = report(&output);
            let diagnostic = result["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["code"] == format!("cem.projection.{family}.{suffix}"))
                .expect("the original header refusal");
            assert_eq!(diagnostic["severity"], "error");
            same_path(diagnostic["uri"].as_str().unwrap(), &path);
            for key in ["line", "column", "byteOffset"] {
                assert!(diagnostic[key].is_null());
            }
        }
    }
}

#[test]
fn explicit_debug_exports_preserve_ranges_and_the_ast_source_contract_gap() {
    let project = Project::new();
    let source = project.read("cem-source");
    let label = "Café Noord";
    let offset = source
        .windows(label.len())
        .position(|bytes| bytes == label.as_bytes())
        .unwrap();
    let events = project.run(project.config("export-events-json"));
    success(&events);
    let event_data = report(&events);
    let event = event_data
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["kind"] == "value" && item["value"] == label)
        .unwrap();
    assert_eq!(
        event["byteRange"],
        json!({ "start": offset, "len": label.len() })
    );
    assert_eq!(&source[offset..offset + label.len()], label.as_bytes());

    let dom = project.run(project.config("export-dom-json"));
    success(&dom);
    let dom_data = report(&dom);
    fn text_node<'a>(node: &'a Value, text: &str) -> Option<&'a Value> {
        if node["kind"] == "text" && node["data"] == text {
            return Some(node);
        }
        node["children"]
            .as_array()?
            .iter()
            .find_map(|child| text_node(child, text))
    }
    let text = text_node(&dom_data, label).expect("native DOM text value");
    // DOM's compatibility range is broad; its native stack retains the finer builder range.
    assert_eq!(
        text["byteRange"],
        json!({ "start": 0, "len": source.len() })
    );
    assert!(text["sourceMap"]["frames"]
        .as_array()
        .unwrap()
        .iter()
        .any(|frame| {
            frame["span"]["ranges"] == json!({ "start": offset, "len": label.len() })
        }));

    let ast = project.run(project.config("export-ast-json"));
    success(&ast);
    assert!(
        report(&ast).is_array(),
        "the actual debug AST writer produces an array"
    );
    let ast_path = project.directory.0.join("actual-export.ast.json");
    fs::write(&ast_path, ast.stdout).unwrap();
    let refused = project.directory.validate(&ast_path, "ast", "json");
    assert_eq!(refused.status.code(), Some(1));
    let result = report(&refused);
    assert!(result["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| {
            item["code"] == "cem.projection.ast.json_shape"
                && item["message"]
                    .as_str()
                    .is_some_and(|text| text.contains("must be an object"))
        }));
    // The authored document-object fixture is independently validated; it is not a wrapped export.
    success(&project.run(project.config("validate-ast-json")));
}

#[test]
fn binary_exports_stay_host_neutral_and_default_views_name_the_original_source() {
    let first = Project::new();
    let second = Project::new();
    for family in ["dom", "ast", "events"] {
        let config = first.config(&format!("export-{family}-binary"));
        let original = first.run(config);
        success(&original);
        let independent = second.run(config);
        success(&independent);
        assert_eq!(original.stdout, independent.stdout);
        assert!(original.stdout.starts_with(b"CEMPROJ\0"));
    }
    for id in ["parse-ast", "parse-events"] {
        let output = first.run(first.config(id));
        success(&output);
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.starts_with("@doc cem-ml 1\n"));
        assert!(text.contains(first.path("cem-source").to_str().unwrap()));
        assert!(text.contains("@byte-offset="));
        assert!(!text.contains("\u{1b}["));
    }
}

fn common_input(project: &Project, id: &str) -> EngineInput {
    let resource = project.source(id);
    EngineInput {
        uri: format!(
            "studio://native-projections/{}",
            resource["path"].as_str().unwrap()
        ),
        bytes: project.read(id),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: resource["contentType"].as_str().map(str::to_owned),
            schema: resource["schema"].as_str().map(str::to_owned),
            ..FormatIdentity::default()
        }),
        root_scope: Default::default(),
    }
}

fn common_validate(input: EngineInput, context: EngineContext) -> cem_ml::engine::ValidateResponse {
    RealCemMlEngine::new()
        .validate(ValidateRequest {
            inputs: vec![input],
            projection: ValidateProjection::Json,
            fail_level: FailLevel::Validate,
            context,
        })
        .unwrap()
}

#[test]
fn common_engine_validates_projection_sources_without_document_import_or_utf8_decode() {
    let project = Project::new();
    for id in [
        "dom-json",
        "ast-json",
        "events-json",
        "dom-binary",
        "ast-binary",
        "events-binary",
    ] {
        let input = common_input(&project, id);
        let validated = common_validate(input.clone(), EngineContext::default());
        assert_eq!(validated.report.summary.input_count, 1);
        assert_eq!(
            validated.report.summary.hard_violation_count, 0,
            "{id}: {:?}",
            validated.report.diagnostics
        );
        assert!(
            validated.report.diagnostics.is_empty(),
            "{id}: {:?}",
            validated.report.diagnostics
        );
        let checked = RealCemMlEngine::new()
            .check(CheckRequest {
                inputs: vec![input],
                projection: ValidateProjection::Json,
                fail_level: FailLevel::Validate,
                zero_hard_violations: true,
                context: EngineContext::default(),
            })
            .unwrap();
        assert_eq!(
            checked.hard_violation_count, 0,
            "{id}: {:?}",
            checked.report.diagnostics
        );
    }
    for family in ["dom", "ast", "events"] {
        for encoding in ["json", "binary"] {
            let id = format!("{family}-{encoding}");
            let mut input = common_input(&project, &id);
            if encoding == "binary" {
                input.bytes[8..10].copy_from_slice(&255_u16.to_be_bytes());
            } else {
                input.bytes = if family == "events" {
                    br#"[{"kind":"value","value":"x","byteRange":{"start":0,"len":-1}}]"#.to_vec()
                } else {
                    br#"{"kind":"document","children":[{"kind":"text","data":"x","byteRange":{"start":-1,"len":1}}]}"#.to_vec()
                };
            }
            let expected = format!(
                "cem.projection.{family}.{}",
                if encoding == "binary" {
                    "binary_version"
                } else {
                    "json_shape"
                }
            );
            let uri = input.uri.clone();
            let response = common_validate(input, EngineContext::default());
            assert_eq!(
                response.report.diagnostics.len(),
                1,
                "{id}: {:?}",
                response.report.diagnostics
            );
            let diagnostic = &response.report.diagnostics[0];
            assert_eq!(diagnostic.code, expected);
            assert_eq!(diagnostic.uri.as_deref(), Some(uri.as_str()));
            assert_eq!(diagnostic.severity, Severity::Error);
            assert_eq!(
                (diagnostic.line, diagnostic.column, diagnostic.byte_offset),
                (None, None, None)
            );
        }
    }
    let actual_array = common_validate(
        common_input(&project, "ast-export"),
        EngineContext::default(),
    );
    assert_eq!(actual_array.report.diagnostics.len(), 1);
    assert_eq!(
        actual_array.report.diagnostics[0].code,
        "cem.projection.ast.json_shape"
    );
}

#[derive(Debug)]
struct ProjectionValidationOverride;
impl SourceValidationStage for ProjectionValidationOverride {
    fn validate(&self, input: &EngineInput) -> Option<Vec<Diagnostic>> {
        Some(vec![Diagnostic {
            code: "fixture.projection.source_stage".into(),
            uri: Some(input.uri.clone()),
            severity: Severity::Error,
            message: "The explicitly registered native stage owns this input".into(),
            ..Diagnostic::default()
        }])
    }
}

#[test]
fn common_projection_validation_preserves_registered_stage_authority_and_explicit_identities() {
    let project = Project::new();
    let mut invalid = common_input(&project, "invalid-dom");
    let context = EngineContext {
        source_validation_stages: vec![Arc::new(ProjectionValidationOverride)],
        ..EngineContext::default()
    };
    let override_result = common_validate(invalid.clone(), context);
    assert_eq!(override_result.report.diagnostics.len(), 1);
    assert_eq!(
        override_result.report.diagnostics[0].code,
        "fixture.projection.source_stage"
    );

    for id in ["dom-json", "dom-binary"] {
        let input = common_input(&project, id);
        let loaded = cem_ml::lifecycle::LifecycleRegistry::with_builtin_adapters()
            .load(&input, &EngineContext::default());
        assert!(
            loaded.ast_stream.is_none(),
            "source validation grants no document owner"
        );
        assert!(loaded.adapter_id.is_none());
        assert!(loaded
            .diagnostics
            .iter()
            .any(|item| item.code == "cem.lifecycle.adapter_unsupported"));
        let mut mime_only = input.clone();
        mime_only.identity.as_mut().unwrap().schema = None;
        assert!(common_validate(mime_only, EngineContext::default())
            .report
            .diagnostics
            .is_empty());
        let mut schema_only = input.clone();
        schema_only.identity.as_mut().unwrap().content_type = None;
        assert!(common_validate(schema_only, EngineContext::default())
            .report
            .diagnostics
            .is_empty());
        let mut contextual = input.clone();
        let identity = contextual.identity.take().unwrap();
        assert!(common_validate(
            contextual,
            EngineContext {
                schema: identity.schema,
                content_type: identity.content_type,
                ..EngineContext::default()
            }
        )
        .report
        .diagnostics
        .is_empty());
        let mut mismatched = input.clone();
        mismatched.identity.as_mut().unwrap().schema =
            Some("https://cem.dev/ns/projection/ast/1".into());
        assert!(
            common_validate(mismatched, EngineContext::default())
                .report
                .summary
                .hard_violation_count
                > 0
        );
    }
    invalid.bytes = project.read("dom-json");
    invalid.identity.as_mut().unwrap().schema =
        Some("https://cem.dev/ns/projection/dom-json/1".into());
    assert!(common_validate(invalid, EngineContext::default())
        .report
        .diagnostics
        .is_empty());
    if cem_ml::schema::package_sources::embedded_source_profile()["externalBodies"] == true {
        let optional_csv = EngineInput {
            uri: "studio://native-projections/unadmitted.csv".into(),
            bytes: b"name\nAda\n".to_vec(),
            from_format: None,
            identity: Some(FormatIdentity {
                content_type: Some("text/csv".into()),
                schema: Some("https://cem.dev/ns/data/csv/1".into()),
                ..FormatIdentity::default()
            }),
            root_scope: Default::default(),
        };
        let refused = common_validate(optional_csv, EngineContext::default());
        assert!(
            refused
                .report
                .diagnostics
                .iter()
                .any(|item| item.code == "cem.schema_asset.missing"),
            "unadmitted optional sources must still refuse: {:?}",
            refused.report.diagnostics
        );
    }
}
