use super::*;
pub(super) fn hash(s: &[u8]) -> String {
    ContentHash::from_blake3(s).header_value()
}
pub(super) fn capture(text: &str) -> Arc<LexicallyScopedDocument> {
    Arc::new(
        CemSchemaMachine::new(
            CompiledSchema::cem_core(),
            CemEventNormalizer::new(CemTokenizer::from_source(BytesSource::new(
                SourceId(1),
                text.as_bytes().to_vec(),
            ))),
        )
        .build_with_lexical_scopes(),
    )
}
pub(super) fn datatype(c: &LexicallyScopedDocument, name: &str) -> AstNodeId {
    c.document().nodes.iter().find_map(|n|match n {CemAstNode::Element{node_id,expanded_name,attributes,..} if expanded_name.local_name=="type" && attributes.iter().any(|id|matches!(c.document().get(*id),Some(CemAstNode::Attribute{expanded_name,value:Some(n),..}) if expanded_name.local_name=="name" && n==name))=>Some(*node_id),_=>None}).unwrap()
}
pub(super) fn field(c: &LexicallyScopedDocument, name: &str, attr: &str) -> AstNodeId {
    let CemAstNode::Element { attributes, .. } = c.document().get(datatype(c, name)).unwrap()
    else {
        unreachable!()
    };
    *attributes.iter().find(|id|matches!(c.document().get(**id),Some(CemAstNode::Attribute{expanded_name,..}) if expanded_name.local_name==attr)).unwrap()
}
pub(super) fn source(uri: &str, text: &str, root: &str) -> ModuleSource {
    let c = capture(text);
    let roots = BTreeSet::from([datatype(&c, root)]);
    ModuleSource {
        identity: SourceIdentity::new(uri, SourceId(1), text.as_bytes()),
        capture: c,
        roots,
        bindings: BTreeMap::new(),
    }
}
pub(super) fn leaf() -> ModuleSource {
    source("file:///app/types/core.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:core | {types | {type @name=scalar @kind=scalar}}}","scalar")
}
pub(super) fn derived(id: &str, target: &str, target_root: AstNodeId) -> ModuleSource {
    let text=format!("@ns s = https://cem.dev/ns/schema/1\n@ns v = urn:{target}\n@default s\n{{schema @namespace=urn:{id} | {{types | {{type @name=value @base={{#selected}}}}}}}}");
    let mut m = source(&format!("file:///app/types/{id}.cem"), &text, "value");
    m.bindings.insert(
        field(&m.capture, "value", "base"),
        ModuleTarget::ModuleDatatype {
            module: target.into(),
            root: target_root,
        },
    );
    m
}
pub(super) fn chain() -> BTreeMap<String, ModuleSource> {
    let leaf = leaf();
    let vendor = derived("vendor", "core", *leaf.roots.first().unwrap());
    let app = derived("app", "vendor", *vendor.roots.first().unwrap());
    BTreeMap::from([
        ("core".into(), leaf),
        ("vendor".into(), vendor),
        ("app".into(), app),
    ])
}
