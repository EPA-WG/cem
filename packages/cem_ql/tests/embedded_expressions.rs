use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use cem_ql::embedded::{
    checked_in_cem_sources, compile_embedded_expression, compile_embedded_expressions,
    compile_repository_embedded_expressions, extract_embedded_expressions_from_source,
    extract_repository_embedded_expressions, parse_embedded_functional_waivers_json,
    validate_embedded_functional_fixtures, validate_embedded_functional_waivers,
    EmbeddedArtifactRole, EmbeddedCompileStage, EmbeddedExpression, EmbeddedFunctionalFixture,
    EmbeddedHostKind,
};
use cem_ql::eval::{AtomValue, Item, ItemStream};
use serde_json::json;

fn workspace_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("packages/cem_ql has workspace root two levels up")
}

fn string(value: &str) -> Item {
    Item::Atomic(AtomValue::String(value.to_owned()))
}

fn record(fields: &[(&str, Vec<Item>)]) -> Item {
    Item::Record(
        fields
            .iter()
            .map(|(name, items)| ((*name).to_owned(), items.clone()))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn bindings(entries: &[(&str, ItemStream)]) -> BTreeMap<String, ItemStream> {
    entries
        .iter()
        .map(|(name, stream)| ((*name).to_owned(), stream.clone()))
        .collect()
}

fn real_expression(
    expressions: &[EmbeddedExpression],
    source_path: &str,
    host_kind: EmbeddedHostKind,
    normalized_source: &str,
) -> EmbeddedExpression {
    expressions
        .iter()
        .find(|expression| {
            expression.source_path().ends_with(source_path)
                && expression.host_kind() == host_kind
                && expression.normalized_source == normalized_source
        })
        .unwrap_or_else(|| {
            panic!(
                "missing embedded expression `{normalized_source}` in `{source_path}` as {host_kind:?}"
            )
        })
        .clone()
}

fn embedded_functional_fixtures(
    expressions: &[EmbeddedExpression],
) -> Vec<EmbeddedFunctionalFixture> {
    let dom_projection =
        "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt";
    let data_island_story = "packages/cem-elements/demo/data-island-tree.cemt";

    let attribute = record(&[
        ("name", vec![string("class")]),
        ("namespace", vec![string("")]),
        ("value", vec![string("hero")]),
    ]);
    let child = record(&[
        ("kind", vec![string("text")]),
        ("data", vec![string("Hello")]),
    ]);
    let dom_node = record(&[
        ("kind", vec![string("element")]),
        ("name", vec![string("article")]),
        ("namespace", vec![string("https://example.test/html")]),
        ("attributes", vec![attribute.clone()]),
        ("children", vec![child.clone()]),
    ]);
    let story_node = record(&[
        ("kind", vec![string("element")]),
        ("name", vec![string("section")]),
        ("tag", vec![string("section")]),
        (
            "attributes",
            vec![record(&[
                ("data-root", vec![string("root")]),
                ("data-level", vec![string("1")]),
            ])],
        ),
        ("children", vec![child.clone()]),
    ]);

    vec![
        EmbeddedFunctionalFixture::new(
            "cem-dom-projection.node-name",
            "schema-package:cem-dom-projection/v1",
            real_expression(
                expressions,
                dom_projection,
                EmbeddedHostKind::AttributeValueTemplate,
                "node.name",
            ),
            bindings(&[("node", ItemStream::once(dom_node.clone()))]),
            vec![string("article")],
        ),
        EmbeddedFunctionalFixture::new(
            "cem-dom-projection.node-attributes",
            "schema-package:cem-dom-projection/v1",
            real_expression(
                expressions,
                dom_projection,
                EmbeddedHostKind::SelectAttribute,
                "node.attributes",
            ),
            bindings(&[("node", ItemStream::once(dom_node.clone()))]),
            vec![attribute.clone()],
        ),
        EmbeddedFunctionalFixture::new(
            "cem-dom-projection.attribute-value",
            "schema-package:cem-dom-projection/v1",
            real_expression(
                expressions,
                dom_projection,
                EmbeddedHostKind::AttributeValueTemplate,
                "attribute.value",
            ),
            bindings(&[("attribute", ItemStream::once(attribute.clone()))]),
            vec![string("hero")],
        ),
        EmbeddedFunctionalFixture::new(
            "cem-dom-projection.child-binding",
            "schema-package:cem-dom-projection/v1",
            real_expression(
                expressions,
                dom_projection,
                EmbeddedHostKind::CallWithAttribute,
                "child",
            ),
            bindings(&[("child", ItemStream::once(child.clone()))]),
            vec![child.clone()],
        ),
        EmbeddedFunctionalFixture::new(
            "cem-elements.data-island.element-test",
            "story:cem-elements/data-island-tree",
            real_expression(
                expressions,
                data_island_story,
                EmbeddedHostKind::TestAttribute,
                r#"node.kind == "element""#,
            ),
            bindings(&[("node", ItemStream::once(story_node.clone()))]),
            vec![Item::Atomic(AtomValue::Boolean(true))],
        ),
        EmbeddedFunctionalFixture::new(
            "cem-elements.data-island.attribute-test",
            "story:cem-elements/data-island-tree",
            real_expression(
                expressions,
                data_island_story,
                EmbeddedHostKind::TestAttribute,
                "node.attributes.data-root",
            ),
            bindings(&[("node", ItemStream::once(story_node.clone()))]),
            vec![string("root")],
        ),
        EmbeddedFunctionalFixture::new(
            "cem-elements.data-island.children-select",
            "story:cem-elements/data-island-tree",
            real_expression(
                expressions,
                data_island_story,
                EmbeddedHostKind::SelectAttribute,
                "node.children",
            ),
            bindings(&[("node", ItemStream::once(story_node))]),
            vec![child],
        ),
    ]
}

fn embedded_functional_waivers() -> Vec<cem_ql::embedded::EmbeddedFunctionalWaiver> {
    parse_embedded_functional_waivers_json(include_str!(
        "../fixtures/embedded-expression-waivers.json"
    ))
    .expect("embedded functional waiver JSON parses")
}

#[test]
fn checked_in_scan_finds_cem_and_cemt_sources() {
    let sources = checked_in_cem_sources(workspace_root()).expect("checked-in CEM sources");
    assert!(sources.iter().any(
        |path| path.ends_with("packages/cem_ml/schema-packages/csv/v1/formatters/pretty.cemt")
    ));
    assert!(sources.iter().any(|path| path.ends_with(
        "packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem"
    )));
    assert!(sources.iter().all(|path| {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| matches!(extension, "cem" | "cemt"))
    }));
}

#[test]
fn repository_extractor_covers_required_embedding_classes() {
    let expressions =
        extract_repository_embedded_expressions(workspace_root()).expect("repository expressions");
    assert!(
        expressions.len() > 100,
        "expected repository-wide embedded expression coverage, got {}",
        expressions.len()
    );
    assert!(expressions.iter().any(|expression| {
        expression.provenance.artifact_role == EmbeddedArtifactRole::Formatter
            && expression
                .provenance
                .source_path
                .ends_with("packages/cem_ml/schema-packages/csv/v1/formatters/pretty.cemt")
            && expression.provenance.host.kind == EmbeddedHostKind::ExpressionNode
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.artifact_role == EmbeddedArtifactRole::Colorizer
            && expression
                .provenance
                .source_path
                .ends_with("packages/cem_ml/schema-packages/csv/v1/colorizers/terminal.cemt")
            && expression.provenance.host.kind == EmbeddedHostKind::ExpressionNode
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.artifact_role == EmbeddedArtifactRole::Converter
            && expression.provenance.source_path.ends_with(
                "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            )
            && matches!(
                expression.provenance.host.kind,
                EmbeddedHostKind::SelectAttribute | EmbeddedHostKind::TestAttribute
            )
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.artifact_role == EmbeddedArtifactRole::Validator
            && expression.provenance.source_path.ends_with(
                "packages/cem_ml/schema-packages/schema/v1/examples/custom-behavior-schema.cem",
            )
            && matches!(
                expression.provenance.host.kind,
                EmbeddedHostKind::BehaviorSelectAttribute
                    | EmbeddedHostKind::BehaviorMatchAttribute
            )
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.host.kind == EmbeddedHostKind::AttributeValueTemplate
            && expression.provenance.source_path.ends_with(
                "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            )
            && expression.normalized_source == "node.name"
    }));
    assert!(expressions.iter().any(|expression| {
        expression.provenance.artifact_role == EmbeddedArtifactRole::TransformConfig
            && expression
                .provenance
                .source_path
                .ends_with("packages/custom-element/material/importmap.transform.cem")
            && expression.provenance.host.kind == EmbeddedHostKind::AttributeValueTemplate
            && expression.normalized_source == "file"
    }));
}

#[test]
fn repository_extractor_preserves_complete_source_provenance() {
    let expressions =
        extract_repository_embedded_expressions(workspace_root()).expect("repository expressions");
    for expression in expressions {
        let provenance = &expression.provenance;
        assert!(
            !provenance.source_path.as_os_str().is_empty(),
            "missing source path for {expression:#?}"
        );
        assert!(
            provenance.host.range.len > 0,
            "missing host byte range for {expression:#?}"
        );
        assert!(
            provenance.cem_ql_range.len > 0,
            "missing CEM-QL sub-span for {expression:#?}"
        );
        assert!(
            provenance.host.range.start <= provenance.cem_ql_range.start,
            "host range must contain CEM-QL span for {expression:#?}"
        );
        assert!(
            provenance.host.range.end() >= provenance.cem_ql_range.end(),
            "host range must contain CEM-QL span for {expression:#?}"
        );
        assert_ne!(
            provenance.artifact_role,
            EmbeddedArtifactRole::Unknown,
            "repository expression must have an artifact role for {expression:#?}"
        );
        if provenance
            .source_path
            .starts_with("packages/cem_ml/schema-packages")
        {
            assert!(
                provenance.schema_package.is_some(),
                "schema-package expression must identify owning package for {expression:#?}"
            );
        }
        match provenance.host.kind {
            EmbeddedHostKind::ExpressionNode => {
                assert!(
                    provenance.host.attribute_name.is_none(),
                    "expression node must not report an attribute name for {expression:#?}"
                );
            }
            _ => {
                assert!(
                    provenance.host.attribute_name.is_some(),
                    "attribute-hosted expression must report the attribute name for {expression:#?}"
                );
            }
        }
    }
}

#[test]
fn compiler_audit_runs_rust_first_expressions_through_all_stages() {
    let source = r#"{root @title="field {node.name}" |
  {cem:if @test='node.kind == "element" && visible' |
    {$ format(node.name) }
  }
}"#;
    let expressions = extract_embedded_expressions_from_source(
        "packages/cem-elements/demo/rust-first.cemt",
        source,
    );
    let reports = compile_embedded_expressions(&expressions);

    assert!(
        reports.len() >= 3,
        "expected AVT, test attribute, and expression-node reports, got {reports:#?}"
    );
    for report in reports {
        assert!(
            report.parse_succeeded,
            "Rust-first embedded expression should parse: {report:#?}"
        );
        assert!(report.resolve_ran, "resolver did not run: {report:#?}");
        assert!(
            report.resolve_succeeded,
            "host-tolerant resolver should accept inferred bindings: {report:#?}"
        );
        assert!(
            report.type_check_ran,
            "type checker did not run: {report:#?}"
        );
        assert!(
            report.type_check_succeeded,
            "dev-profile type checker should not raise hard diagnostics: {report:#?}"
        );
        assert!(
            !report.has_hard_diagnostics(),
            "Rust-first expression emitted hard diagnostics: {report:#?}"
        );
    }
}

#[test]
fn compiler_audit_rejects_old_xpath_boolean_syntax_with_source_provenance() {
    let source = r#"{cem:if @test='node.kind = "element" and visible' | ok}"#;
    let expressions = extract_embedded_expressions_from_source(
        "packages/cem_ml/schema-packages/demo/v1/formatters/legacy.cemt",
        source,
    );
    let expression = expressions
        .iter()
        .find(|expression| expression.host_kind() == EmbeddedHostKind::TestAttribute)
        .expect("test attribute expression");
    let report = compile_embedded_expression(expression);

    assert!(!report.parse_succeeded, "{report:#?}");
    assert!(!report.resolve_ran, "{report:#?}");
    assert!(!report.type_check_ran, "{report:#?}");
    let diagnostic = report
        .diagnostics_for_stage(EmbeddedCompileStage::Parse)
        .find(|diagnostic| diagnostic.diagnostic.code == "cem.ql.use_rust_boolean_ops")
        .expect("XPath `and` should report the Rust-first boolean diagnostic");
    assert_eq!(
        diagnostic.diagnostic.uri.as_deref(),
        Some("packages/cem_ml/schema-packages/demo/v1/formatters/legacy.cemt")
    );
    let source_offset = diagnostic
        .source_byte_offset
        .expect("mapped source byte offset");
    assert!(source_offset >= expression.expression_range().start);
    assert!(source_offset < expression.expression_range().end());
}

#[test]
fn compiler_audit_wraps_cem_native_template_diagnostics_with_slot_contract() {
    let source = r#"{module |
  {template @name="page" |
    {body |
      {cem:if @test='node.kind = "element" and visible' | ok}
    }
  }
}"#;
    let expressions = extract_embedded_expressions_from_source(
        "packages/cem_ml/schema-packages/cem-native-template/v1/examples/invalid-slot.cem",
        source,
    );
    let expression = expressions
        .iter()
        .find(|expression| expression.host_kind() == EmbeddedHostKind::TestAttribute)
        .expect("test attribute expression");
    let report = compile_embedded_expression(expression);

    assert_eq!(report.slot.host_package, "cem-native-template/v1");
    assert_eq!(report.slot.slot_kind, EmbeddedHostKind::TestAttribute);
    assert_eq!(report.slot.evaluation_phase.as_str(), "render");
    assert_eq!(
        report
            .slot
            .expected_type
            .as_ref()
            .map(|ty| format!("{ty:?}")),
        Some("Atom(Boolean)".to_owned())
    );

    let diagnostic = report
        .diagnostics_for_stage(EmbeddedCompileStage::Parse)
        .find(|diagnostic| diagnostic.diagnostic.code == "cem.ql.use_rust_boolean_ops")
        .expect("parse diagnostic from CEM-QL");
    let details = diagnostic
        .diagnostic
        .details
        .as_ref()
        .expect("diagnostic keeps expression slot details");
    assert_eq!(details["behavior"], json!("cem-ql-expression-report-fact"));
    assert_eq!(
        details["expressionSlot"]["contract"],
        json!("expression-slot")
    );
    assert_eq!(
        details["expressionSlot"]["hostPackage"],
        json!("cem-native-template/v1")
    );
    assert_eq!(
        details["expressionSlot"]["slotKind"],
        json!("test-attribute")
    );
    assert_eq!(
        details["expressionSlot"]["expectedType"],
        json!("schema:boolean")
    );
    assert_eq!(
        details["expressionSlot"]["sourceUri"],
        json!("packages/cem_ml/schema-packages/cem-native-template/v1/examples/invalid-slot.cem")
    );
    assert!(details["expressionSlot"]["hostRange"]["byteOffset"].is_u64());
    assert!(details["expressionSlot"]["expressionRange"]["byteOffset"].is_u64());
}

#[test]
fn repository_compile_audit_runs_parsable_expressions_and_keeps_dom_converter_migrated() {
    let reports = compile_repository_embedded_expressions(workspace_root())
        .expect("repository compile audit reports");
    assert!(
        reports.len() > 100,
        "expected repository-wide compile audit coverage, got {}",
        reports.len()
    );
    assert!(reports.iter().all(|report| {
        !report.parse_succeeded || (report.resolve_ran && report.type_check_ran)
    }));

    let dom_predicate_report = reports
        .iter()
        .find(|report| {
            report.expression.source_path().ends_with(
                "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            ) && report.expression.source
                == r#"node.kind == "element" && str:starts_with(node.name, "@")"#
        })
        .expect("dom-to-html fixture keeps the migrated Rust-first element predicate");
    assert!(
        dom_predicate_report.parse_succeeded,
        "{dom_predicate_report:#?}"
    );
    assert!(
        reports.iter().all(|report| {
            !report.expression.source_path().ends_with(
                "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
            ) || !report.expression.source.contains(" and ")
        }),
        "checked-in dom-to-html converter should not retain XPath-style `and` syntax"
    );
}

#[test]
fn checked_in_dom_converter_expression_reports_exact_source_file_and_byte_range() {
    let source_path = Path::new(
        "packages/cem_ml/schema-packages/cem-dom-projection/v1/converters/dom-to-html.cemt",
    );
    let expression_source = r#"node.kind == "element" && str:starts_with(node.name, "@")"#;
    let host_source = format!("'{expression_source}'");
    let checked_in_source = std::fs::read_to_string(workspace_root().join(source_path))
        .expect("checked-in dom-to-html CEMT fixture");
    let expected_host_start = checked_in_source
        .find(&host_source)
        .expect("checked-in fixture keeps the migrated Rust-first @test expression")
        as u64;
    let expected_expression_start = expected_host_start + 1;

    let reports = compile_repository_embedded_expressions(workspace_root())
        .expect("repository compile audit reports");
    let report = reports
        .iter()
        .find(|report| {
            report.expression.source_path() == source_path
                && report.expression.host_kind() == EmbeddedHostKind::TestAttribute
                && report.expression.source == expression_source
        })
        .expect("repository audit report for checked-in Rust-first expression");

    assert_eq!(report.expression.source_path(), source_path);
    assert_eq!(report.expression.host_range().start, expected_host_start);
    assert_eq!(report.expression.host_range().len, host_source.len() as u32);
    assert_eq!(
        report.expression.expression_range().start,
        expected_expression_start
    );
    assert_eq!(
        report.expression.expression_range().len,
        expression_source.len() as u32
    );
    assert!(report.parse_succeeded, "{report:#?}");
    assert!(report.resolve_ran, "{report:#?}");
    assert!(report.type_check_ran, "{report:#?}");
    assert!(!report.has_hard_diagnostics(), "{report:#?}");
}

#[test]
fn functional_fixtures_evaluate_checked_in_expressions_by_group() {
    let expressions =
        extract_repository_embedded_expressions(workspace_root()).expect("repository expressions");
    let fixtures = embedded_functional_fixtures(&expressions);

    let groups = fixtures
        .iter()
        .map(|fixture| fixture.group.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        groups,
        BTreeSet::from([
            "schema-package:cem-dom-projection/v1",
            "story:cem-elements/data-island-tree"
        ])
    );

    let reports = validate_embedded_functional_fixtures(&fixtures);
    for report in &reports {
        assert!(
            report.succeeded(),
            "{} failed: {:?}\n{report:#?}",
            report.id,
            report.failure_reason()
        );
    }
}

#[test]
fn explicit_functional_waivers_are_well_scoped_and_owned() {
    let expressions =
        extract_repository_embedded_expressions(workspace_root()).expect("repository expressions");
    let waivers = embedded_functional_waivers();

    assert!(
        waivers.len() >= 5,
        "expected concrete waivers for known embedded audit gaps, got {waivers:#?}"
    );
    let errors = validate_embedded_functional_waivers(&waivers, &expressions);
    assert!(errors.is_empty(), "invalid embedded waivers: {errors:#?}");

    let ids = waivers
        .iter()
        .map(|waiver| waiver.id.as_str())
        .collect::<BTreeSet<_>>();
    assert!(ids.contains("schema-package-csv-v1-cemt-helper-dsl"));
    assert!(ids.contains("schema-package-schema-v1-behavior-runtime"));
    assert!(ids.contains("custom-element-material-importmap-external-resources"));
    assert!(waivers.iter().all(|waiver| {
        !waiver.owner.trim().is_empty()
            && !waiver.reason.trim().is_empty()
            && !waiver.removal_condition.trim().is_empty()
    }));

    for waiver in &waivers {
        let matched = expressions
            .iter()
            .filter(|expression| waiver.matches_expression(expression))
            .count();
        assert!(
            matched > 0,
            "waiver {} must match at least one expression",
            waiver.id
        );
    }
}

#[test]
fn embedded_expression_audit_gate_runs_compile_fixtures_and_waivers() {
    let expressions =
        extract_repository_embedded_expressions(workspace_root()).expect("repository expressions");
    assert!(
        expressions.len() > 100,
        "audit gate expected repository-wide expression coverage"
    );

    let compile_reports = compile_embedded_expressions(&expressions);
    assert_eq!(compile_reports.len(), expressions.len());
    assert!(
        compile_reports
            .iter()
            .all(|report| !report.parse_succeeded || (report.resolve_ran && report.type_check_ran)),
        "all parsable expressions must reach resolver and type checker"
    );
    assert!(
        compile_reports.iter().all(|report| {
            !report
                .diagnostics_for_stage(EmbeddedCompileStage::Parse)
                .any(|diagnostic| diagnostic.diagnostic.code == "cem.ql.use_rust_boolean_ops")
        }),
        "checked-in embedded expressions must stay on Rust-first boolean syntax"
    );

    let fixture_reports =
        validate_embedded_functional_fixtures(&embedded_functional_fixtures(&expressions));
    let fixture_failures = fixture_reports
        .iter()
        .filter_map(|report| {
            (!report.succeeded()).then(|| (report.id.clone(), report.failure_reason()))
        })
        .collect::<Vec<_>>();
    assert!(
        fixture_failures.is_empty(),
        "embedded functional fixture failures: {fixture_failures:#?}"
    );

    let waiver_errors =
        validate_embedded_functional_waivers(&embedded_functional_waivers(), &expressions);
    assert!(
        waiver_errors.is_empty(),
        "embedded waiver validation errors: {waiver_errors:#?}"
    );
}

#[test]
fn embedded_expression_audit_target_is_registered() {
    let project = include_str!("../project.json");
    assert!(
        project.contains("\"verify-embedded-expressions\""),
        "project.json must expose the embedded expression audit verification target"
    );
    assert!(
        project.contains("--test embedded_expressions"),
        "embedded expression audit target must run the focused Rust integration suite"
    );
}

#[test]
fn source_extractor_keeps_host_and_expression_ranges_distinct() {
    let source = r#"{item @title="prefix { $label } suffix" | {$ $body }}"#;
    let expressions = extract_embedded_expressions_from_source("examples/range.cem", source);
    assert_eq!(expressions.len(), 2);

    let avt = &expressions[0];
    assert_eq!(avt.source, "$label");
    assert_eq!(avt.normalized_source, "label");
    assert!(avt.provenance.host.range.start < avt.provenance.cem_ql_range.start);
    assert!(avt.provenance.host.range.end() > avt.provenance.cem_ql_range.end());

    let content = &expressions[1];
    assert_eq!(content.source, "$body");
    assert_eq!(content.normalized_source, "body");
    assert!(content.provenance.host.range.start < content.provenance.cem_ql_range.start);
}
