use cem_ml::{
    import::import_bytes_with_lexical_scopes,
    operation_control::OperationControl,
    parser::CemAstNode,
    resolver::{
        ResolveDirection, ResolvePolicyDenial, ResolvePolicyRequestKey, ResolvePolicySubstitution,
        ResolvePurpose, ResolveRequest, ResolvedRead, ResolvedWrite, ResolverDiagnostic,
        ResolverPolicy, ResolverRegistry, ResourceResolver,
    },
    scheduler::AbortSignal,
    schema::{
        declaration_references::SchemaDeclarationNode, uri_loading::SchemaUriResourceRequest,
        vocab::CompiledSchema,
    },
    source_map::SourceMapStack,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
fn request(uri: &str, policy: &ResolverPolicy) -> SchemaUriResourceRequest {
    SchemaUriResourceRequest::new(
        uri,
        "https://vendor.test/source/main.cem",
        Some("text/cem-ml"),
        policy,
        SourceMapStack::default(),
    )
    .unwrap()
}
fn response(text: &str, mime: &str) -> ResolvedRead {
    ResolvedRead {
        uri: "https://cdn.test/loaded.cem".into(),
        bytes: text.as_bytes().to_vec(),
        content_type: Some(mime.into()),
    }
}
fn no_exports(
    _: &cem_ml::import::ScopedCemImport,
    _: &str,
) -> Result<Vec<SchemaDeclarationNode>, String> {
    Err("no public exports".into())
}
const SINGLE: &str =
    "@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=child}}}";
#[test]
fn schema_byte_import_retains_cem_xml_owners_names_and_unresolved_occurrences() {
    for (text,mime) in [(SINGLE,"text/cem-ml"),("<s:schema xmlns:s='https://cem.dev/ns/schema/1'><s:elements><s:element name='child'/></s:elements></s:schema>","application/xml")] {
        let imported = import_bytes_with_lexical_scopes(text.as_bytes(),mime,"https://vendor.test/schema",CompiledSchema::cem_core()).unwrap();
        assert!(Arc::ptr_eq(imported.tree.ast_owner(), imported.captured.document()));
        let loaded=request("../schema",&ResolverPolicy::new()).import_response(response(text,mime),no_exports).unwrap();
        assert_eq!(loaded.targets.len(),1);
        assert!(Arc::ptr_eq(loaded.targets[0].document(), loaded.imported.tree.ast_owner()));
        assert_eq!(loaded.imported.captured.expanded_name(loaded.targets[0].document(),loaded.targets[0].node_id()).unwrap().namespace_uri,"https://cem.dev/ns/schema/1");
        assert_eq!(loaded.imported.tree.source_uri(),"https://cdn.test/loaded.cem");
    }
    let imported = import_bytes_with_lexical_scopes(
        format!("{SINGLE} {{#later}}").as_bytes(),
        "text/cem-ml",
        "source.cem",
        CompiledSchema::cem_core(),
    )
    .unwrap();
    assert_eq!(imported.captured.occurrences().count(), 1);
    assert!(imported.tree.ast().nodes.iter().all(|node| !matches!(
        node,
        CemAstNode::Reference {
            targets: Some(_),
            ..
        }
    )));
}
#[test]
fn schema_uri_default_selection_rejects_zero_multiple_and_nested_candidates() {
    for text in [
        "{application}",
        "{container | {s:schema}}",
        "{s:schema} {s:schema}",
    ] {
        let text = format!("@ns s = https://cem.dev/ns/schema/1\n{text}");
        assert!(request("schema", &ResolverPolicy::new())
            .import_response(response(&text, "text/cem-ml"), no_exports)
            .is_err());
    }
    let malformed_wrapper = "@ns s = https://cem.dev/ns/schema/1\n@ns c = https://cem.dev/ns/core/1\n{c:schema @c:name=bad | {s:schema} {s:schema}} {s:schema}";
    assert!(request("schema", &ResolverPolicy::new())
        .import_response(response(malformed_wrapper, "text/cem-ml"), no_exports)
        .is_err());
    let text = format!("{SINGLE} {{application}}");
    assert!(request("schema", &ResolverPolicy::new())
        .import_response(response(&text, "text/cem-ml"), no_exports)
        .is_ok());
    let wrapper = "@ns s = https://cem.dev/ns/schema/1\n@ns c = https://cem.dev/ns/core/1\n{c:schema @c:name=chosen | {s:schema}}";
    assert!(request("schema", &ResolverPolicy::new())
        .import_response(response(wrapper, "text/cem-ml"), no_exports)
        .is_ok());
}
#[test]
fn public_fragment_requires_explicit_loader_exports_and_checks_original_owner() {
    let load = request("../schema#public", &ResolverPolicy::new());
    assert_eq!(load.request().uri, "https://vendor.test/schema");
    assert_eq!(load.public_part(), Some("public"));
    assert!(load
        .import_response(response(SINGLE, "text/cem-ml"), no_exports)
        .is_err());
    let loaded = load
        .import_response(response(SINGLE, "text/cem-ml"), |imported, part| {
            assert_eq!(part, "public");
            let target = imported
                .tree
                .ast()
                .nodes
                .iter()
                .find_map(|node| match node {
                    CemAstNode::Element {
                        node_id,
                        expanded_name,
                        ..
                    } if expanded_name.local_name == "schema" => {
                        SchemaDeclarationNode::new(imported.tree.ast_owner().clone(), *node_id)
                    }
                    _ => None,
                })
                .unwrap();
            Ok(vec![target])
        })
        .unwrap();
    let foreign = loaded.targets[0].clone();
    assert!(load
        .import_response(response(SINGLE, "text/cem-ml"), |_, _| Ok(vec![foreign]))
        .is_err());
}
struct Reader(Arc<AtomicUsize>);
impl ResourceResolver for Reader {
    fn read(&self, _: &ResolveRequest) -> Result<ResolvedRead, ResolverDiagnostic> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(response(SINGLE, "text/cem-ml"))
    }
    fn write(&self, _: &ResolveRequest, _: &[u8]) -> Result<ResolvedWrite, ResolverDiagnostic> {
        unreachable!()
    }
}
#[test]
fn schema_uri_policy_and_abort_gate_transport_and_preserve_response_location() {
    let key = ResolvePolicyRequestKey::new(
        "https://vendor.test/schema",
        ResolvePurpose::Template,
        ResolveDirection::Read,
    );
    let policy = ResolverPolicy::new().with_denial(key.clone(), ResolvePolicyDenial::new("denied"));
    assert!(SchemaUriResourceRequest::new(
        "../schema",
        "https://vendor.test/source/main.cem",
        None,
        &policy,
        SourceMapStack::default()
    )
    .is_err());
    let policy = ResolverPolicy::new().with_substitution(
        key,
        ResolvePolicySubstitution::new("https://mirror.test/schema", "mirror"),
    );
    let load = request("../schema", &policy);
    assert_eq!(load.request().uri, "https://mirror.test/schema");
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = ResolverRegistry::new();
    registry.register(
        "https",
        ResolvePurpose::Template,
        ResolveDirection::Read,
        Reader(calls.clone()),
    );
    let abort = AbortSignal::new();
    abort.abort();
    let operation = OperationControl::new(abort);
    assert!(load
        .read(&registry, &operation, operation.root_scope())
        .is_err());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let operation = OperationControl::new(AbortSignal::new());
    let loaded = load
        .import_response(
            load.read(&registry, &operation, operation.root_scope())
                .unwrap(),
            no_exports,
        )
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        loaded.imported.tree.source_uri(),
        "https://cdn.test/loaded.cem"
    );
}
#[test]
fn schema_response_import_requires_supported_metadata_and_bounded_valid_source() {
    let load = request("schema", &ResolverPolicy::new());
    for result in [
        response(SINGLE, "application/json"),
        response("{broken", "text/cem-ml"),
        ResolvedRead {
            uri: "https://vendor.test/schema".into(),
            bytes: vec![0; cem_ml::import::MAX_DOCUMENT_BYTES + 1],
            content_type: Some("text/cem-ml".into()),
        },
    ] {
        assert!(load.import_response(result, no_exports).is_err());
    }
    let unknown = SchemaUriResourceRequest::new(
        "schema",
        "https://vendor.test/main.cem",
        None,
        &ResolverPolicy::new(),
        SourceMapStack::default(),
    )
    .unwrap();
    assert!(unknown
        .import_response(
            ResolvedRead {
                uri: "https://vendor.test/schema".into(),
                bytes: SINGLE.as_bytes().to_vec(),
                content_type: None
            },
            no_exports
        )
        .is_err());
    assert!(load
        .import_response(
            ResolvedRead {
                uri: "https://vendor.test/schema".into(),
                bytes: vec![0xff],
                content_type: Some("text/cem-ml".into())
            },
            no_exports
        )
        .is_err());
    let local = SchemaUriResourceRequest::new(
        "../schema.cem",
        "file:///project/input/main.cem",
        None,
        &ResolverPolicy::new(),
        SourceMapStack::default(),
    )
    .unwrap();
    assert_eq!(local.request().uri, "file:///project/schema.cem");
    assert!(import_bytes_with_lexical_scopes(
        SINGLE.as_bytes(),
        "text/cem-ml;charset=latin1",
        "source.cem",
        CompiledSchema::cem_core()
    )
    .is_err());
}
