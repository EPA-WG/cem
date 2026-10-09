use super::*;

fn datatype_compiler(name: &str, tokenizer_ready: bool) -> CemQlSchemaPackageCompiler {
    compiler(Some(name)).with_datatypes(move |request,host,limits| {
        use cem_ml::schema::{datatype_registry::{DatatypeRegistry,DatatypeKind},datatype_validation::{ScalarRepresentation,ValueRepresentation},declaration_references::SchemaDeclarationNode};
        use cem_ql::datatype_compilation::{DatatypeImplementation,DatatypeImplementations,TokenizerBinding,compile_datatypes};
        let ast=request.source.ast_owner();
        let schema=ast.nodes.iter().find_map(|n|match n {CemAstNode::Element{node_id,expanded_name,..} if expanded_name.local_name=="schema"=>SchemaDeclarationNode::new(ast.clone(),*node_id),_=>None}).unwrap();
        let mut sources=vec![];let mut registry=DatatypeRegistry::default();let mut implementations=DatatypeImplementations::default();
        for n in &ast.nodes {if let CemAstNode::Element{node_id,expanded_name,attributes,..}=n {if expanded_name.local_name=="type" {
            let declaration=SchemaDeclarationNode::new(ast.clone(),*node_id).unwrap();registry.insert(schema.clone(),declaration).unwrap();
            let name=attributes.iter().find_map(|id|match ast.get(*id){Some(CemAstNode::Attribute{expanded_name,value,..}) if expanded_name.local_name=="name"=>value.as_deref(),_=>None}).unwrap();
            let source=registry.source(&schema,name).unwrap();host.register_datatype_source(source.clone()).unwrap();host.bind_literal_datatype(&schema,name,source.declaration().clone()).unwrap();
            let list=name=="names";
            implementations.register(DatatypeImplementation{source:source.clone(),kind:if list {DatatypeKind::List}else{DatatypeKind::Scalar},representation:if list {ValueRepresentation::List(ScalarRepresentation::String)}else{ValueRepresentation::Scalar(ScalarRepresentation::String)},accepted_bases:vec![],bounds:Default::default(),tokenizer:if !list {TokenizerBinding::Absent}else if tokenizer_ready {TokenizerBinding::Ready(cem_ml::schema::datatype_contracts::RegisteredTokenizer::whitespace())}else{TokenizerBinding::Unavailable},validator:None}).unwrap();sources.push(source);
        }}}
        Ok(compile_datatypes(ast.clone(),&sources,host,&implementations,&Default::default(),limits))
    })
}
#[test]
fn incomplete_datatype_capability_preserves_package_and_retries_original_candidate() {
    let authored=SOURCE.replace("{elements |", "{types | {type @name=item @kind=scalar} {type @name=names @kind=list @base=item}} {elements |");
    let mut context = context(&authored);
    context.schema_package_compiler = Some(Arc::new(datatype_compiler("old", true)));
    load(&mut context, &input());
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    let original = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    let active = context
        .schema_document_models
        .resolve_for_identity(Some(SCHEMA_URI), None, None)
        .unwrap()
        .datatype_compilation
        .clone()
        .unwrap();
    assert!(active.is_ready());
    set_source(
        &mut context,
        &authored.replace("@kind=list", "@kind=list @max-items=2"),
    );
    let mut replacement = input();
    replacement.bytes = MANIFEST
        .replace("runtime-converter", "new-converter")
        .replace("old.cemt", "new.cemt")
        .into_bytes();
    context.schema_package_compiler = Some(Arc::new(datatype_compiler("new", false)));
    load(&mut context, &replacement);
    assert_active(&context, "old", "runtime-converter", "old.cemt");
    assert!(context
        .converter_registry
        .converter("new-converter")
        .is_none());
    let candidate = context
        .schema_package_sources
        .get(SOURCE_URI)
        .unwrap()
        .clone();
    assert!(!Arc::ptr_eq(&original, &candidate));
    let pending = context
        .schema_document_models
        .get(SCHEMA_URI)
        .unwrap()
        .datatype_compilation
        .as_ref()
        .unwrap();
    assert!(!pending.is_ready());
    assert_eq!(pending.issues[0].code, "datatype-tokenizer-unavailable");
    assert!(pending.matches_owner(candidate.ast_owner()));
    assert!(active.matches_owner(original.ast_owner()));
    context.schema_package_compiler = Some(Arc::new(datatype_compiler("new", true)));
    load(&mut context, &replacement);
    assert_active(&context, "new", "new-converter", "new.cemt");
    assert!(context
        .converter_registry
        .converter("runtime-converter")
        .is_none());
    assert!(Arc::ptr_eq(
        &candidate,
        context.schema_package_sources.get(SOURCE_URI).unwrap()
    ));
    set_source(
        &mut context,
        &authored.replace("@kind=list", "@kind=list @max-items=-1"),
    );
    let invalid = load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
    assert!(
        invalid.iter().any(|d| d.severity.is_hard_violation()
            && d.message.contains("invalid-item-bound")
            && d.source_map.is_some()),
        "{invalid:?}"
    );
    assert_active(&context, "new", "new-converter", "new.cemt");
    // A successful snapshot from the previous owner cannot authorize another source.
    context.schema_package_compiler = Some(Arc::new(
        compiler(Some("untrusted")).with_datatypes(move |_, _, _| Ok((*active).clone())),
    ));
    let diagnostics =
        load_schema_package_manifest_into_context(&mut context, &replacement).unwrap();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.severity.is_hard_violation() && d.message.contains("another source owner")),
        "{diagnostics:?}"
    );
    assert_active(&context, "new", "new-converter", "new.cemt");
}
