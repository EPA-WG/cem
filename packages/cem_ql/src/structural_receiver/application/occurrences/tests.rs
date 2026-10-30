use super::*;
fn source(nodes: usize) -> ComponentReceiverSource {
    let text = "{node}".repeat(nodes - 1);
    let imported = cem_ml::import::import_bytes_with_lexical_scopes(
        text.as_bytes(),
        "text/cem-ml",
        "file:///bounds.cem",
        cem_ml::schema::vocab::CompiledSchema::cem_core(),
    )
    .unwrap();
    assert_eq!(imported.tree.ast().nodes.len(), nodes);
    ComponentReceiverSource {
        schema_uri: "https://app.test/receiver".into(),
        source: imported.tree,
        attributes: BTreeMap::new(),
    }
}
#[test]
fn source_node_bounds_count_shared_arenas_once_and_refuse_unique_overflow() {
    let one = source(16_384);
    let mut sources = BTreeMap::from([("first".into(), one)]);
    assert!(bounds(&sources).is_ok());
    let shared = sources["first"].source.clone();
    for i in 0..5 {
        sources.insert(
            format!("shared{i}"),
            ComponentReceiverSource {
                schema_uri: "https://app.test/receiver".into(),
                source: shared.clone(),
                attributes: BTreeMap::new(),
            },
        );
    }
    assert!(bounds(&sources).is_ok());
    for i in 0..3 {
        sources.insert(format!("unique{i}"), source(16_384));
    }
    assert!(bounds(&sources).is_ok());
    sources.insert("overflow".into(), source(1));
    assert!(bounds(&sources).is_err());
    assert!(bounds(&BTreeMap::from([("oversize".into(), source(16_385))])).is_err());
}
#[test]
fn occurrence_bounds_accept_64_per_component_and_256_total_then_refuse_overflow() {
    let mut sources = BTreeMap::new();
    for i in 0..4 {
        let mut source = source(1);
        source.attributes = (0..64).map(|i| (format!("attribute{i}"), 0)).collect();
        sources.insert(format!("component{i}"), source);
    }
    assert!(bounds(&sources).is_ok());
    sources
        .get_mut("component0")
        .unwrap()
        .attributes
        .insert("overflow".into(), 0);
    assert!(bounds(&sources).is_err());
    sources
        .get_mut("component0")
        .unwrap()
        .attributes
        .remove("overflow");
    let mut extra = source(1);
    extra.attributes.insert("extra".into(), 0);
    sources.insert("extra".into(), extra);
    assert!(bounds(&sources).is_err());
}
