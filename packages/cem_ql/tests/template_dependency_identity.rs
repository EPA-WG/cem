use cem_ml::{content_cache::ContentHash, dependency_identity::*};
use cem_ql::render::{
    compile_template_module_closure, compile_template_module_closure_with_dependencies,
    render_compiled_template, render_plan_to_html, render_plan_to_html_with_source_map,
    CompileTemplateOptions, TemplateData, TemplateModuleClosure, TemplateModuleSource,
};

fn hash(source: &str) -> String {
    ContentHash::from_blake3(source.as_bytes()).header_value()
}
fn closure(source: &str, partial: &str) -> TemplateModuleClosure {
    TemplateModuleClosure {
        root_uri: "studio://sample/root.cemt".into(),
        root_content_hash: hash(source),
        resolver_policy_stamp: "resolver-policy/1".into(),
        entrypoint: "body".into(),
        modules: vec![TemplateModuleSource {
            alias: "docs".into(),
            parent_uri: None,
            uri: "studio://sample/docs.cemt".into(),
            content_hash: hash(partial),
            source: partial.into(),
        }],
        ..TemplateModuleClosure::default()
    }
}
const ROOT: &str =
    "{module | {import @as=docs @src=\"docs.cemt\"} {body | {call @from=docs @template=label}}}";
const PARTIAL: &str =
    "{module | {template @name=label @visibility=public | {body | {p | Imported content}}}}";
#[test]
fn native_compilation_receipt_preserves_render_and_source_frames_and_invalidates_dependencies() {
    let options = CompileTemplateOptions::default();
    let before = closure(ROOT, PARTIAL);
    let compiled = compile_template_module_closure_with_dependencies(ROOT, &before, &options);
    let key = compiled.identity.as_ref().unwrap();
    let ordinary = compile_template_module_closure(ROOT, &before, &options);
    let output = render_compiled_template(&compiled.artifact, &TemplateData::default());
    let expected = render_compiled_template(&ordinary, &TemplateData::default());
    assert_eq!(output.nodes, expected.nodes);
    assert_eq!(compiled.artifact.diagnostics, ordinary.diagnostics);
    assert!(render_plan_to_html(&output).contains("Imported content"));
    let mapped = render_plan_to_html_with_source_map(&output);
    assert!(mapped.output_spans.iter().any(|span| span.origin.frames.iter().any(|frame| matches!(&frame.transform, cem_ml::source_map::TransformKind::SourceBoundary { uri } if uri == "studio://sample/docs.cemt"))));
    assert_eq!(
        *key,
        compile_template_module_closure_with_dependencies(ROOT, &before, &options)
            .identity
            .unwrap()
    );
    let changed_source = PARTIAL.replace("Imported content", "Edited content");
    let changed = compile_template_module_closure_with_dependencies(
        ROOT,
        &closure(ROOT, &changed_source),
        &options,
    );
    assert_ne!(*key, changed.identity.unwrap());
    let mut policy = before.clone();
    policy.resolver_policy_stamp = "resolver-policy/2".into();
    assert_ne!(
        *key,
        compile_template_module_closure_with_dependencies(ROOT, &policy, &options)
            .identity
            .unwrap()
    );
    let binding = CompileTemplateOptions {
        host_bindings: vec!["extra".into()],
        ..options.clone()
    };
    assert_ne!(
        *key,
        compile_template_module_closure_with_dependencies(ROOT, &before, &binding)
            .identity
            .unwrap()
    );
    let skip = CompileTemplateOptions {
        skip_cemt_function_bodies: true,
        ..options
    };
    assert_ne!(
        *key,
        compile_template_module_closure_with_dependencies(ROOT, &before, &skip)
            .identity
            .unwrap()
    );
}
#[test]
fn invalid_or_unversioned_native_closures_keep_diagnostics_and_never_offer_an_identity() {
    let options = CompileTemplateOptions::default();
    let mut opaque = closure(ROOT, PARTIAL);
    opaque.resolver_policy_stamp.clear();
    let result = compile_template_module_closure_with_dependencies(ROOT, &opaque, &options);
    assert!(result.artifact.diagnostics.is_empty());
    assert_eq!(
        result.identity,
        Err(DependencyIdentityError::OpaqueDependency)
    );
    for corrupt in [0, 1, 2, 3] {
        let mut invalid = closure(ROOT, PARTIAL);
        match corrupt {
            0 => invalid.root_content_hash = hash("wrong"),
            1 => invalid.modules[0].content_hash = hash("wrong"),
            2 => invalid.modules.clear(),
            _ => invalid.cem_ql_version = "different-engine".into(),
        }
        let result = compile_template_module_closure_with_dependencies(ROOT, &invalid, &options);
        assert!(result.identity.is_err());
        assert!(!result.artifact.diagnostics.is_empty());
    }
}

#[test]
fn nested_import_content_resolution_and_declaration_order_are_identity_axes() {
    let middle = "{module | {import @as=cells @src=\"cells.cemt\"} {template @name=label @visibility=public | {body | {call @from=cells @template=value}}}}";
    let leaf = "{module | {template @name=value @visibility=public | {body | {strong | Nested}}}}";
    let mut before = closure(ROOT, middle);
    before.modules.push(TemplateModuleSource {
        alias: "cells".into(),
        parent_uri: Some(before.modules[0].uri.clone()),
        uri: "studio://sample/cells.cemt".into(),
        content_hash: hash(leaf),
        source: leaf.into(),
    });
    let options = CompileTemplateOptions::default();
    let original = compile_template_module_closure_with_dependencies(ROOT, &before, &options);
    let key = original.identity.unwrap();
    assert!(render_plan_to_html(&render_compiled_template(
        &original.artifact,
        &TemplateData::default()
    ))
    .contains("<strong>Nested</strong>"));
    let mut changed = before.clone();
    changed.modules[1].source = leaf.replace("Nested", "Leaf edit");
    changed.modules[1].content_hash = hash(&changed.modules[1].source);
    assert_ne!(
        key,
        compile_template_module_closure_with_dependencies(ROOT, &changed, &options)
            .identity
            .unwrap()
    );
    let mut changed = before.clone();
    changed.modules[1].uri = "studio://sample/moved.cemt".into();
    assert_ne!(
        key,
        compile_template_module_closure_with_dependencies(ROOT, &changed, &options)
            .identity
            .unwrap()
    );
    let mut changed = before.clone();
    changed.modules.reverse();
    assert_ne!(
        key,
        compile_template_module_closure_with_dependencies(ROOT, &changed, &options)
            .identity
            .unwrap()
    );
    let mut dangling = before.clone();
    dangling.modules[1].parent_uri = Some("studio://sample/missing.cemt".into());
    assert!(
        compile_template_module_closure_with_dependencies(ROOT, &dangling, &options)
            .identity
            .is_err()
    );
}
#[test]
fn effective_options_are_canonical_and_conflicting_provenance_owners_fail_closed() {
    let options = CompileTemplateOptions {
        host_bindings: vec!["b".into(), "a".into(), "a".into()],
        ..CompileTemplateOptions::default()
    };
    let before = closure(ROOT, PARTIAL);
    let key = compile_template_module_closure_with_dependencies(ROOT, &before, &options)
        .identity
        .unwrap();
    let reordered = CompileTemplateOptions {
        host_bindings: vec!["a".into(), "b".into()],
        source_uri: Some("ignored://overridden-by-closure".into()),
        ..options
    };
    assert_eq!(
        key,
        compile_template_module_closure_with_dependencies(ROOT, &before, &reordered)
            .identity
            .unwrap()
    );
    let mut conflicting = before.clone();
    conflicting.modules[0].uri = conflicting.root_uri.clone();
    assert!(
        compile_template_module_closure_with_dependencies(ROOT, &conflicting, &reordered)
            .identity
            .is_err()
    );
}
