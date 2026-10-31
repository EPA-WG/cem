use super::*;
pub(super) fn node(c: &LexicallyScopedDocument, local: &str, n: usize) -> u32 {
    c.document()
        .nodes
        .iter()
        .filter_map(|x| match x {
            CemAstNode::Element {
                node_id,
                expanded_name,
                ..
            } if expanded_name.local_name == local => Some(*node_id),
            _ => None,
        })
        .nth(n)
        .unwrap()
}
pub(super) fn reference(c: &LexicallyScopedDocument, text: &str) -> u32 {
    c.document()
        .nodes
        .iter()
        .find_map(|x| match x {
            CemAstNode::Reference {
                node_id,
                expression,
                ..
            } if expression == text => Some(*node_id),
            _ => None,
        })
        .unwrap()
}
pub(super) fn wrapped(uri: &str, text: &str, root: Option<&str>) -> assembly::ModuleSource {
    let c = capture(text);
    let roots = root
        .map(|n| BTreeSet::from([datatype(&c, n)]))
        .unwrap_or_default();
    assembly::ModuleSource::new(ModuleSource {
        identity: SourceIdentity::new(uri, SourceId(1), text.as_bytes()),
        capture: c,
        roots,
        bindings: BTreeMap::new(),
    })
}
pub(super) fn pack(m: &assembly::ModuleSource, name: &str) -> assembly::ProfileArtifact {
    let c = &m.types.capture;
    let schema = SchemaDeclarationNode::new(c.document().clone(), node(c, "schema", 0)).unwrap();
    let declaration = SchemaDeclarationNode::new(c.document().clone(), datatype(c, name)).unwrap();
    let mut registry = DatatypeRegistry::default();
    registry.insert(schema.clone(), declaration).unwrap();
    let source = registry.source(&schema, name).unwrap();
    let control = OperationControl::default();
    let a = dependency_sources::export_operation(
        c,
        &source,
        OperationMode::Constant,
        m.types.identity.clone(),
        ImplementationIdentity::constant_interpreter(
            "constant",
            hash(b"native"),
            "constant",
            ScalarRepresentation::String,
        ),
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    let p = profiles::OperationProfile {
        source: m.types.identity.clone(),
        scope: schema.node_id(),
        declaration: source.declaration().node_id(),
        kind: cem_ml::schema::datatype_registry::DatatypeKind::Scalar,
        representation: cem_ml::schema::datatype_validation::ValueRepresentation::Scalar(
            ScalarRepresentation::String,
        ),
        dependencies: BTreeMap::from([(OperationRole::Constant, a.pin.clone())]),
    };
    let sources = vec![a];
    let b = profiles::export(
        &BTreeMap::from([("constant".into(), p)]),
        &sources,
        &hash(b"registry"),
        &control,
    )
    .unwrap();
    assembly::ProfileArtifact {
        bytes: b.bytes,
        pin: b.pin,
        sources,
    }
}
pub(super) fn fixture() -> (
    BTreeMap<String, assembly::ModuleSource>,
    Vec<assembly::ProfileArtifact>,
) {
    let core = assembly::ModuleSource::new(leaf());
    let palette=wrapped("file:///app/palette.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:palette | {constants | {constant @value=blue} {constant @value=red}}}",None);
    let mut app=wrapped("file:///app/app.cem","@ns s = https://cem.dev/ns/schema/1\n@default s\n{schema @namespace=urn:app | {types | {type @name=choice @base={#base} | {#colors} {constant @value=local}}}}",Some("choice"));
    let c = app.types.capture.clone();
    app.types.bindings.insert(
        field(&c, "choice", "base"),
        ModuleTarget::ModuleDatatype {
            module: "core".into(),
            root: *core.types.roots.first().unwrap(),
        },
    );
    app.imports = BTreeSet::from(["core".into(), "palette".into()]);
    let mut palette = palette;
    let blue = node(&palette.types.capture, "constant", 0);
    let red = node(&palette.types.capture, "constant", 1);
    palette.constant_exports = BTreeSet::from([blue, red]);
    let target = |id| assembly::ModuleNodeId {
        module: "palette".into(),
        node: id,
    };
    app.constants.insert(
        reference(&c, "#colors"),
        vec![target(blue), target(red), target(blue)],
    );
    let artifact = pack(&app, "choice");
    app.profiles.insert(
        datatype(&c, "choice"),
        assembly::ProfileSelection {
            pin: artifact.pin.clone(),
            id: "constant".into(),
        },
    );
    (
        BTreeMap::from([
            ("core".into(), core),
            ("palette".into(), palette),
            ("app".into(), app),
        ]),
        vec![artifact],
    )
}
