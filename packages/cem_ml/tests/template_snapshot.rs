use cem_ml::{engine::EngineContext, resolver::*, template_snapshot::*};
use std::sync::Arc;

fn snapshot(root: &str) -> Arc<TemplateSourceSnapshot> {
    TemplateSourceSnapshot::new(
        vec![
            TemplateSnapshotSource {
                uri: "https://example.test/root.cemt".into(),
                revision: "root:1".into(),
                bytes: root.as_bytes().to_vec(),
                content_type: "text/cem-ml".into(),
            },
            TemplateSnapshotSource {
                uri: "https://example.test/child.cemt".into(),
                revision: "child:1".into(),
                bytes: b"{module | {template @name=card | {span | Child}}}".to_vec(),
                content_type: "text/cem-ml".into(),
            },
        ],
        Default::default(),
    )
    .unwrap()
}
#[test]
fn snapshot_preflight_retains_exact_sources_and_policy_substitution() {
    let sources = snapshot(
        r#"{module | {import @as=card @src="./alias.cemt"} {template @name=body | {card.card}}}"#,
    );
    let mut context = EngineContext::default();
    context.resolver_policy.register_substitution(
        ResolvePolicyRequestKey::new(
            "./alias.cemt",
            ResolvePurpose::Template,
            ResolveDirection::Read,
        ),
        ResolvePolicySubstitution {
            substituted_uri: "https://example.test/child.cemt".into(),
            reason: "fixture".into(),
        },
    );
    let context = Arc::new(context);
    let record = preflight_template_snapshot(
        sources.clone(),
        context.clone(),
        "https://example.test/root.cemt",
        8,
    )
    .unwrap();
    let child = &record
        .inspection()
        .preflight
        .as_ref()
        .unwrap()
        .resolved_imports[0];
    assert_eq!(child.uri, "https://example.test/child.cemt");
    assert_eq!(child.substituted_uri.as_deref(), Some(child.uri.as_str()));
    assert_eq!(child.bytes, sources.source(&child.uri).unwrap().bytes);
    assert_eq!(sources.source(&child.uri).unwrap().revision, "child:1");
    record.validate_current(&sources, &context).unwrap();
    assert!(record
        .validate_current(&snapshot(r#"{span | Replacement}"#), &context)
        .is_err());
    assert!(record
        .validate_current(&sources, &Arc::new((*context).clone()))
        .is_err());
}
#[test]
fn snapshot_preflight_denies_absent_resources_and_policy_before_any_fallback() {
    for uri in [
        "https://missing.test/no.cemt",
        "file:///etc/passwd",
        "./absent.cemt",
    ] {
        let sources = snapshot(&format!(r#"{{module | {{import @as=x @src="{uri}"}}}}"#));
        let result = preflight_template_snapshot(
            sources,
            Arc::new(EngineContext::default()),
            "https://example.test/root.cemt",
            8,
        );
        assert!(result.is_err(), "{uri}");
    }
    let sources = snapshot(r#"{module | {import @as=x @src="./child.cemt"}}"#);
    let mut context = EngineContext::default();
    context.resolver_policy.register_denial(
        ResolvePolicyRequestKey::new(
            "./child.cemt",
            ResolvePurpose::Template,
            ResolveDirection::Read,
        ),
        ResolvePolicyDenial {
            reason: "denied".into(),
        },
    );
    assert!(preflight_template_snapshot(
        sources.clone(),
        Arc::new(context),
        "https://example.test/root.cemt",
        8
    )
    .is_err());
    assert!(preflight_template_snapshot(
        sources,
        Arc::new(EngineContext::default()),
        "https://example.test/root.cemt",
        0
    )
    .is_err());
}
#[test]
fn snapshot_validates_membership_budgets_and_cancellation() {
    let sources = snapshot("{div | Root}");
    let source = sources
        .source("https://example.test/root.cemt")
        .unwrap()
        .clone();
    assert!(
        TemplateSourceSnapshot::new(vec![source.clone(), source.clone()], Default::default())
            .is_err()
    );
    assert!(TemplateSourceSnapshot::new(
        vec![source.clone()],
        TemplateSnapshotLimits {
            max_sources: 0,
            max_bytes: 1000,
            ..Default::default()
        }
    )
    .is_err());
    assert!(TemplateSourceSnapshot::new(
        vec![source.clone()],
        TemplateSnapshotLimits {
            max_sources: 5,
            max_bytes: 1,
            ..Default::default()
        }
    )
    .is_err());
    let mut invalid = source;
    invalid.revision.clear();
    assert!(TemplateSourceSnapshot::new(vec![invalid], Default::default()).is_err());
    let context = Arc::new(EngineContext::default());
    context.operation_control.abort_signal().abort();
    assert!(
        preflight_template_snapshot(sources, context, "https://example.test/root.cemt", 8).is_err()
    );
}

#[test]
fn snapshot_preflight_bounds_repeated_import_edges() {
    let original = snapshot(
        r#"{module | {import @as=a @src="./child.cemt"} {import @as=b @src="./child.cemt"}}"#,
    );
    let sources = TemplateSourceSnapshot::new(
        vec![
            original
                .source("https://example.test/root.cemt")
                .unwrap()
                .clone(),
            original
                .source("https://example.test/child.cemt")
                .unwrap()
                .clone(),
        ],
        TemplateSnapshotLimits {
            max_imports: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(preflight_template_snapshot(
        sources,
        Arc::new(EngineContext::default()),
        "https://example.test/root.cemt",
        8
    )
    .is_err());
}

#[test]
fn snapshot_file_lookup_never_reads_even_an_existing_valid_template() {
    let path =
        std::env::temp_dir().join(format!("cem-snapshot-fixture-{}.cemt", std::process::id()));
    std::fs::write(&path, "{module | {template @name=card | {span | Disk}}}").unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().to_string();
    let original = snapshot(&format!(r#"{{module | {{import @as=x @src="{uri}"}}}}"#));
    let result = preflight_template_snapshot(
        original.clone(),
        Arc::new(EngineContext::default()),
        "https://example.test/root.cemt",
        8,
    );
    std::fs::remove_file(&path).unwrap();
    assert!(result.err().unwrap().contains("fallback I/O forbidden"));
    let retained = TemplateSourceSnapshot::new(
        vec![
            original
                .source("https://example.test/root.cemt")
                .unwrap()
                .clone(),
            TemplateSnapshotSource {
                uri,
                revision: "snapshot-only".into(),
                bytes: b"{module | {template @name=card | {span | Snapshot}}}".to_vec(),
                content_type: "text/cem-ml".into(),
            },
        ],
        Default::default(),
    )
    .unwrap();
    let record = preflight_template_snapshot(
        retained,
        Arc::new(EngineContext::default()),
        "https://example.test/root.cemt",
        8,
    )
    .unwrap();
    assert!(std::str::from_utf8(
        &record
            .inspection()
            .preflight
            .as_ref()
            .unwrap()
            .resolved_imports[0]
            .bytes
    )
    .unwrap()
    .contains("Snapshot"));
}

#[test]
fn snapshot_preflight_bounds_cumulative_acquired_bytes() {
    let root = TemplateSnapshotSource {
        uri: "https://example.test/root.cemt".into(),
        revision: "1".into(),
        content_type: "text/cem-ml".into(),
        bytes:
            br#"{module | {import @as=a @src="./child.cemt"} {import @as=b @src="./child.cemt"}}"#
                .to_vec(),
    };
    let child = TemplateSnapshotSource {
        uri: "https://example.test/child.cemt".into(),
        revision: "1".into(),
        content_type: "text/cem-ml".into(),
        bytes: format!("{{span | {}}}", "x".repeat(4000)).into_bytes(),
    };
    let sources = TemplateSourceSnapshot::new(
        vec![root, child],
        TemplateSnapshotLimits {
            max_bytes: 6000,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(preflight_template_snapshot(
        sources,
        Arc::new(EngineContext::default()),
        "https://example.test/root.cemt",
        8
    )
    .is_err());
}
