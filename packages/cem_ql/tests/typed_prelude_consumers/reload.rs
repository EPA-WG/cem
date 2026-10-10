use super::*;
use cem_ml::ast::reload::{ReferenceReloadBundle, ReloadLimits, ReloadSource};

fn reload(input: &ScopedCemImport, text: &str) -> ScopedCemImport {
    let bundle = ReferenceReloadBundle::export(
        &input.captured,
        vec![
            ReloadSource::new(SourceId(0), "typed-preludes.cem", text.as_bytes(), true),
            ReloadSource::new(SourceId(1), "typed-preludes.cem", text.as_bytes(), true),
        ],
        ReloadLimits::default(),
    )
    .unwrap();
    let bytes = bundle.encode(ReloadLimits::default()).unwrap();
    let (_, restored) =
        ReferenceReloadBundle::decode_with_document(&bytes, ReloadLimits::default()).unwrap();
    let captured = restored.require_lexical().unwrap().clone();
    let tree = RetainedCemTree::from_shared(
        restored.document,
        "typed-preludes.cem",
        text,
        CemTreeSemantics::default(),
        None,
    )
    .unwrap();
    ScopedCemImport { captured, tree }
}

#[test]
fn reload_requires_fresh_contexts_and_directed_grants_and_preserves_source() {
    let text = "@doc cem-ml 1.1\n@ns ui = {$ library}\n@default ui\n{item}";
    let original = import(text);
    let vendor = import("@ns public = urn:vendor");
    let ctx = context(&vendor, "library", &elements(&vendor, "@ns"));
    let mut old = CemQlSchemaDeclarationHost::new();
    let origin = old.register_scope(original.tree.clone(), Some(ctx.clone()), policy());
    let destination = old.register_scope(vendor.tree.clone(), Some(Default::default()), policy());
    old.attach_captured_namespaces(original.captured.clone())
        .unwrap();
    old.attach_captured_namespaces(vendor.captured.clone())
        .unwrap();
    old.allow_scope_crossing(origin, destination);
    let report = old
        .prepare_namespace_property(
            node(&original, elements(&original, "@ns")[0]),
            policy().limits,
        )
        .unwrap();
    assert!(report.is_ready());
    old.publish_namespace_property(&report).unwrap();
    let restored = reload(&original, text);
    assert!(!Arc::ptr_eq(
        original.tree.ast_owner(),
        restored.tree.ast_owner()
    ));
    let mut fresh = CemQlSchemaDeclarationHost::new();
    let origin = fresh.register_scope(restored.tree.clone(), None, policy());
    let destination = fresh.register_scope(vendor.tree.clone(), Some(Default::default()), policy());
    fresh
        .attach_captured_namespaces(restored.captured.clone())
        .unwrap();
    fresh
        .attach_captured_namespaces(vendor.captured.clone())
        .unwrap();
    let declaration = node(&restored, elements(&restored, "@ns")[0]);
    assert!(!fresh
        .prepare_namespace_property(declaration.clone(), policy().limits)
        .unwrap()
        .is_ready());
    fresh.set_context(origin, Some(ctx.clone()));
    assert!(!fresh
        .prepare_namespace_property(declaration.clone(), policy().limits)
        .unwrap()
        .is_ready());
    fresh.allow_scope_crossing(origin, destination);
    let ready = fresh
        .prepare_namespace_property(declaration, policy().limits)
        .unwrap();
    assert!(ready.is_ready());
    assert!(fresh.publish_namespace_property(&report).is_err());
    let (snapshot, ()) = fresh
        .with_namespace_lifecycle(
            restored.captured.clone(),
            &elements(&restored, "item"),
            policy().limits,
            |_, _, _, _| (Some(ctx.clone()), Default::default()),
            |host, snapshot| {
                assert!(snapshot.is_complete());
                assert_eq!(
                    host.consuming_expanded_name(&node(&restored, elements(&restored, "item")[0]))
                        .unwrap()
                        .namespace_uri,
                    "urn:vendor"
                );
            },
        )
        .unwrap();
    assert!(snapshot.is_complete());
    assert_original(&original);
    assert_original(&restored);
}

#[test]
fn reloaded_schema_slot_enters_native_following_lifecycle() {
    use cem_ml::schema::document_model::compile_schema_document_model;
    let text = "@doc cem-ml 1.1\n@ns s = https://cem.dev/ns/schema/1\n@schema select={$ library}\n{after @selected=yes} {s:schema | {elements | {element @name=after @required-attributes=selected}}}";
    let input = reload(&import(text), text);
    let ctx = context(&input, "library", &elements(&input, "schema"));
    let mut host = CemQlSchemaDeclarationHost::new();
    host.register_scope(input.tree.clone(), Some(ctx.clone()), policy());
    host.attach_captured_names(&input.captured).unwrap();
    let outer = compile_schema_document_model(
        "outer",
        "{schema | {elements | {element @name=after @required-attributes=wrong}}}",
    );
    let report = host
        .validate_input_runtime_host_regions(
            "reload",
            input.tree.clone(),
            &[elements(&input, "@schema")[0], elements(&input, "after")[0]],
            &outer,
            policy().limits,
            |_| Some(ctx.clone()),
        )
        .unwrap();
    assert!(
        report.validation.complete && !report.validation.failed,
        "{:?}",
        report.validation.diagnostics
    );
    assert_eq!(report.scopes.len(), 1);
    assert_original(&input);
}
