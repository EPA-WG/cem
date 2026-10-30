//! Bounded native input routing. JSON carries names/indices only, never values.
use crate::{
    eval::{portable::decode_values_with_control, Item, ItemStream},
    render::TemplateData,
};
use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    value::artifact::CemValueArtifactLimits,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const MAX_BINDING_BYTES: usize = 16 * 1024;
pub(super) const LIMITS: CemValueArtifactLimits = CemValueArtifactLimits {
    max_bytes: super::MAX_INPUT_BYTES,
    max_values: 16_384,
    max_depth: 64,
};
const INVALID: &str = "compiled declaration native inputs refused";

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum Target {
    Attribute,
    Slice,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    name: String,
    index: usize,
    target: Target,
    attribute: Option<String>,
}

fn name_allowed(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 256
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        && !matches!(
            name,
            "datadom"
                | "attributes"
                | "slices"
                | "island"
                | "instanceID"
                | "suggestions"
                | "input"
                | "position"
        )
}

pub(super) fn bind(
    bytes: &[u8],
    routing: &str,
    control: &OperationControl,
) -> Result<TemplateData, String> {
    if bytes.len() > LIMITS.max_bytes || routing.len() > MAX_BINDING_BYTES {
        return Err(INVALID.into());
    }
    let bindings: Vec<Binding> = serde_json::from_str(routing).map_err(|_| INVALID)?;
    if bindings.len() > 64 {
        return Err(INVALID.into());
    }
    let mut names = BTreeSet::new();
    for binding in &bindings {
        // Attribute and slice aliases share a query namespace. Reserve runtime
        // roots and reject overlap before constructing the temporary input frame.
        if !name_allowed(&binding.name)
            || !names.insert(&binding.name)
            || matches!(binding.target, Target::Attribute) && binding.attribute.is_some()
            || binding
                .attribute
                .as_ref()
                .is_some_and(|name| name.is_empty() || name.len() > 256)
        {
            return Err(INVALID.into());
        }
    }
    let values = decode_values_with_control(bytes, &LIMITS, control, ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| INVALID)?;
    // These are native control envelopes, not reconstructed CEM document nodes.
    // Explicit fields keep slice aliases out of synthesized host attributes.
    let mut data = TemplateData::default().with_binding(
        "datadom",
        ItemStream::once(Item::Record(BTreeMap::from([
            ("attributes".into(), vec![Item::Record(BTreeMap::new())]),
            ("slices".into(), vec![Item::Record(BTreeMap::new())]),
        ]))),
    );
    for binding in bindings {
        let value = values.items.get(binding.index).cloned().ok_or(INVALID)?;
        match binding.target {
            Target::Attribute => {
                crate::api::native_values::attribute_values(&value, &binding.name)
                    .map_err(|_| INVALID)?;
                data.bind_native_attribute(value).map_err(|_| INVALID)?;
            }
            Target::Slice => {
                let values = match binding.attribute {
                    Some(name) => crate::api::native_values::attribute_values(&value, &name)
                        .map_err(|_| INVALID)?,
                    None => ItemStream::once(value),
                };
                data.bind_native_slice(&binding.name, values)
                    .map_err(|_| INVALID)?;
            }
        }
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::{AtomValue, Item};
    use cem_ml::{
        source::{ByteRange, SourceId},
        source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind},
        value::artifact::{CemValueGraph, CemValueProvenance, CemValueRecord},
    };

    fn graph() -> CemValueGraph {
        let source = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(7),
                span: FrameSpan::Single(ByteRange { start: 1, len: 2 }),
                transform: TransformKind::SourceBoundary {
                    uri: "app://cart/quantity.cem".into(),
                },
            }],
        };
        CemValueGraph {
            records: vec![
                CemValueRecord {
                    kind: "attribute".into(),
                    name: "quantity".into(),
                    lexical: "compatibility projection".into(),
                    values: vec![1],
                    ..Default::default()
                },
                CemValueRecord {
                    kind: "atomic".into(),
                    datatype: "integer".into(),
                    lexical: "2".into(),
                    provenance: Some(CemValueProvenance {
                        source_uri: Some("app://cart/quantity.cem".into()),
                        source_key: Some("quantity-source-key".into()),
                        line_number: Some(1),
                    }),
                    source,
                    ..Default::default()
                },
                CemValueRecord {
                    kind: "reference".into(),
                    targets: vec![1],
                    ..Default::default()
                },
            ],
            roots: vec![0, 2],
        }
    }
    #[test]
    fn shared_codec_retains_typed_values_references_and_original_source() {
        let graph = graph();
        let bytes = graph.encode(&Default::default()).unwrap();
        let data = bind(&bytes, r#"[{"target":"attribute","name":"quantity","index":0},{"target":"slice","name":"saved","index":0,"attribute":"quantity"},{"target":"slice","name":"reference","index":1}]"#, &super::super::render_control().unwrap()).unwrap();
        for name in ["quantity", "saved"] {
            let value = &data.bindings[name].items[0];
            assert_eq!(value.atom(), Some(AtomValue::Integer(2)));
            assert_eq!(value.source_map(), Some(graph.records[1].source.clone()));
            let provenance = value.view().unwrap().provenance().unwrap();
            assert_eq!(
                provenance.source_uri.as_deref(),
                Some("app://cart/quantity.cem")
            );
            assert_eq!(
                provenance.source_key.as_deref(),
                Some("quantity-source-key")
            );
        }
        let reference = data.bindings["reference"].items[0].view().unwrap();
        let target = reference.field("targets").unwrap();
        assert_eq!(target[0].atom(), Some(AtomValue::Integer(2)));
        assert_eq!(
            target[0].view().unwrap().identity(),
            data.bindings["quantity"].items[0]
                .view()
                .unwrap()
                .identity()
        );
        assert_eq!(
            target[0].source_map(),
            Some(graph.records[1].source.clone())
        );
        assert!(matches!(data.bindings["datadom"].items[0], Item::Record(_)));
    }
    #[test]
    fn attribute_and_slice_channels_are_independent_of_binding_order() {
        let bytes = graph().encode(&Default::default()).unwrap();
        for routing in [
            r#"[{"target":"attribute","name":"quantity","index":0},{"target":"slice","name":"saved","index":1}]"#,
            r#"[{"target":"slice","name":"saved","index":1},{"target":"attribute","name":"quantity","index":0}]"#,
        ] {
            let data = bind(&bytes, routing, &super::super::render_control().unwrap()).unwrap();
            let [Item::Record(document)] = data.bindings["datadom"].items.as_slice() else {
                panic!("missing data envelope")
            };
            let [Item::Record(attributes)] = document["attributes"].as_slice() else {
                panic!("missing attributes")
            };
            let [Item::Record(slices)] = document["slices"].as_slice() else {
                panic!("missing slices")
            };
            assert_eq!(
                attributes.keys().map(String::as_str).collect::<Vec<_>>(),
                ["quantity"]
            );
            assert_eq!(
                slices.keys().map(String::as_str).collect::<Vec<_>>(),
                ["saved"]
            );
        }
    }
    #[test]
    fn refuses_invalid_routing_without_echoing_data_or_loosening_limits() {
        let bytes = graph().encode(&Default::default()).unwrap();
        let control = super::super::render_control().unwrap();
        for routing in [
            r#"[{"target":"attribute","name":"wrong","index":0}]"#,
            r#"[{"target":"attribute","name":"quantity","index":1}]"#,
            r#"[{"target":"slice","name":"saved","index":0,"attribute":"wrong"}]"#,
            r#"[{"target":"attribute","name":"quantity","index":0,"attribute":"quantity"}]"#,
            r#"[{"target":"slice","name":"saved","index":99}]"#,
            r#"[{"target":"slice","name":"saved","index":-1}]"#,
            r#"[{"target":"provider","name":"saved","index":0}]"#,
            r#"[{"target":"slice","name":"saved","index":0,"value":"private credential"}]"#,
            r#"[{"target":"attribute","name":"quantity","index":0},{"target":"slice","name":"quantity","index":1}]"#,
            r#"[{"target":"slice","name":"saved","index":0},{"target":"slice","name":"saved","index":1}]"#,
        ] {
            assert_eq!(
                bind(&bytes, routing, &control).unwrap_err(),
                INVALID,
                "{routing}"
            );
        }
        for name in [
            "",
            "datadom",
            "attributes",
            "slices",
            "island",
            "instanceID",
            "suggestions",
            "input",
            "position",
            "attributes.label",
            "a b",
            "$secret",
        ] {
            let routing = serde_json::json!([{"target":"slice","name":name,"index":0}]).to_string();
            assert_eq!(bind(&bytes, &routing, &control).unwrap_err(), INVALID);
        }
        let many = (0..65)
            .map(|index| serde_json::json!({"target":"slice","name":format!("s{index}"),"index":0}))
            .collect::<Vec<_>>();
        assert_eq!(
            bind(&bytes, &serde_json::to_string(&many).unwrap(), &control).unwrap_err(),
            INVALID
        );
        assert_eq!(
            bind(b"private credential", "[]", &control).unwrap_err(),
            INVALID
        );
        assert!(bind(&vec![0; LIMITS.max_bytes + 1], "[]", &control).is_err());
        assert!(bind(&bytes, &" ".repeat(MAX_BINDING_BYTES + 1), &control).is_err());
        let oversized = CemValueGraph {
            records: vec![
                CemValueRecord {
                    kind: "atomic".into(),
                    datatype: "null".into(),
                    ..Default::default()
                };
                LIMITS.max_values + 1
            ],
            roots: vec![0],
        };
        assert!(bind(
            &oversized.encode(&Default::default()).unwrap(),
            "[]",
            &control
        )
        .is_err());
        let mut deep = graph();
        deep.roots = vec![0];
        deep.records.clear();
        for index in 0..66 {
            deep.records.push(CemValueRecord {
                kind: "element".into(),
                name: "nested".into(),
                parent: if index > 0 { Some(index - 1) } else { None },
                children: if index < 65 { vec![index + 1] } else { vec![] },
                ..Default::default()
            });
        }
        assert!(bind(&deep.encode(&Default::default()).unwrap(), "[]", &control).is_err());
        assert!(bind(
            &bytes,
            r#"[{"target":"attribute","name":"quantity","index":0}]"#,
            &super::super::render_control().unwrap()
        )
        .is_ok());
    }
}
